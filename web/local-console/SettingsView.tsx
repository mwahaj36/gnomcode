'use client';

import { type FormEvent, useCallback, useEffect, useRef, useState } from 'react';
import { ProviderSetupPanel } from './ProviderSetup';
import { fetchAccount, sectionMatches, SETTINGS_SECTIONS, signOutDesktop } from './settings-index';
import {
  updateProviderSources,
  type ProviderOverview,
} from '../lib/console-presentation.js';
import { fetchLocalJson, fetchLocalMutation, getConsoleBaseUrl } from './local-api';
import {
  applyThemeMode,
  currentThemeMode,
  navigateTo,
  revealSection,
  startOnboarding,
  takeSettingsQuery,
  type ThemeMode,
} from './shell-bus';

interface RemoteDeviceRow {
  id: string;
  name: string;
  connected?: boolean;
}

interface RemoteDevicesResponse {
  directory_error?: boolean;
  desktop_id?: string | null;
  account_access?: boolean;
  remote_enabled?: boolean;
  devices?: RemoteDeviceRow[];
}

interface ResumeQueueEntry {
  session_id: string;
  agent: string;
  cwd?: string;
  resume_prompt?: string;
  max_runs: number;
  runs_completed: number;
  eligible_after?: string;
}

interface GuardrailRule {
  id: string;
  agent?: string | null;
  max_spend_usd?: number | null;
  max_turns?: number | null;
  action: string;
}

interface GuardrailAlert {
  session_id: string;
  rule_id: string;
  action: string;
  created_at?: string;
}

interface RelayEntry {
  id: string;
  session_id: string;
  project_name?: string;
  source_agent: string;
  target_agent: string;
  fill_pct: number;
  status: string;
  created_at?: string;
}

interface CustomPricingRule {
  model_pattern: string;
  input_per_mtok_usd: number;
  output_per_mtok_usd: number;
  cache_read_per_mtok_usd: number;
}

interface UpdateStatusResponse {
  auto_update_enabled?: boolean;
  available_version?: string;
  current_version?: string;
  state?: string;
  last_error?: string;
}

const POLL_INTERVAL_MS = 15_000;

function isSettingsVisible(): boolean {
  if (typeof document === 'undefined') return false;
  const view = document.getElementById('view-settings');
  return Boolean(view && view.classList.contains('active') && document.visibilityState !== 'hidden');
}

function formatAgeLabel(isoStr?: string): string {
  if (!isoStr) return 'N/A';
  const parsed = new Date(isoStr).getTime();
  if (!Number.isFinite(parsed)) return isoStr;
  const diffMins = Math.floor((Date.now() - parsed) / 60000);
  if (diffMins < 1) return 'Just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${Math.floor(diffHours / 24)}d ago`;
}

export function SettingsView() {
  const providerListRef = useRef<HTMLDivElement | null>(null);
  const [query, setQuery] = useState(() => takeSettingsQuery());
  const [selectedId, setSelectedId] = useState<string>(() => {
    if (typeof window !== 'undefined') {
      const hashId = window.location.hash.replace(/^#/, '');
      if (hashId === 'settings/providers') return 'settings-providers';
      if (SETTINGS_SECTIONS.some((section) => section.id === hashId)) return hashId;
    }
    return SETTINGS_SECTIONS[0]?.id ?? 'settings-account';
  });
  const [accountState, setAccountState] = useState<{ paired: boolean; email?: string | null } | null>(null);
  useEffect(() => {
    void fetchAccount().then(setAccountState, () => setAccountState(null));
    const onState = (event: Event) => {
      if (!(event as CustomEvent<{ paired: boolean }>).detail.paired) setAccountState({ paired: false });
      else void fetchAccount().then(setAccountState, () => undefined);
    };
    const onQuery = () => setQuery(takeSettingsQuery());
    const onReveal = (event: Event) => {
      const id = (event as CustomEvent<{ id?: string }>).detail?.id;
      if (id && SETTINGS_SECTIONS.some((section) => section.id === id)) {
        setSelectedId(id);
        setQuery('');
      }
    };
    const onHash = () => {
      const hashId = window.location.hash.replace(/^#/, '');
      if (SETTINGS_SECTIONS.some((section) => section.id === hashId)) setSelectedId(hashId);
    };
    const onProviderSetup = () => {
      setSelectedId('settings-providers');
      setQuery('');
    };
    window.addEventListener('gnomcode:account-state', onState);
    window.addEventListener('gnomcode:settings-query', onQuery);
    window.addEventListener('gnomcode:reveal-section', onReveal);
    window.addEventListener('hashchange', onHash);
    window.addEventListener('gnomcode:provider-setup', onProviderSetup);
    return () => {
      window.removeEventListener('gnomcode:account-state', onState);
      window.removeEventListener('gnomcode:settings-query', onQuery);
      window.removeEventListener('gnomcode:reveal-section', onReveal);
      window.removeEventListener('hashchange', onHash);
      window.removeEventListener('gnomcode:provider-setup', onProviderSetup);
    };
  }, []);
  const baseUrl = getConsoleBaseUrl();

  const [themeMode, setThemeMode] = useState<ThemeMode>(() => currentThemeMode());

  useEffect(() => {
    const onTheme = () => setThemeMode(currentThemeMode());
    window.addEventListener('gnomcode:theme-changed', onTheme);
    const bridge = typeof window !== 'undefined' ? window.gnomcodeDesktop : undefined;
    let active = true;
    if (bridge?.getLoginItem && bridge?.setLoginItem) {
      setLoginItemSupported(true);
      void bridge.getLoginItem().then(
        (enabled) => {
          if (active) setLoginItemEnabled(Boolean(enabled));
        },
        () => undefined,
      );
    }
    return () => {
      active = false;
      window.removeEventListener('gnomcode:theme-changed', onTheme);
    };
  }, []);

  // Remote devices state
  const [remoteStatus, setRemoteStatus] = useState('Checking devices...');
  const [desktopId, setDesktopId] = useState<string | null>(null);
  const [accountAccess, setAccountAccess] = useState(false);
  const [remoteEnabled, setRemoteEnabled] = useState(false);
  const [devices, setDevices] = useState<RemoteDeviceRow[]>([]);
  const [remoteBusy, setRemoteBusy] = useState(false);
  const [pendingDeviceAction, setPendingDeviceAction] = useState<
    | { kind: 'toggle_access'; enable: boolean }
    | { kind: 'remove'; id: string; name: string }
    | null
  >(null);

  // Automations state
  const [resumeQueue, setResumeQueue] = useState<ResumeQueueEntry[]>([]);
  const [rules, setRules] = useState<GuardrailRule[]>([]);
  const [alerts, setAlerts] = useState<GuardrailAlert[]>([]);
  const [relays, setRelays] = useState<RelayEntry[]>([]);
  const [pendingRelayId, setPendingRelayId] = useState<string | null>(null);
  const [ruleAgent, setRuleAgent] = useState('');
  const [ruleSpend, setRuleSpend] = useState('');
  const [ruleTurns, setRuleTurns] = useState('');
  const [ruleAction, setRuleAction] = useState('Warn');
  const [ruleBusy, setRuleBusy] = useState(false);
  const [ruleStatus, setRuleStatus] = useState('');
  const [automationStatus, setAutomationStatus] = useState('');

  // Notifications state
  const [notifyApproval, setNotifyApproval] = useState(true);
  const [notifyTurnFinished, setNotifyTurnFinished] = useState(true);
  const [notifyTurnMinutes, setNotifyTurnMinutes] = useState('2');
  const [notifyLimitLow, setNotifyLimitLow] = useState(true);
  const [notifyLimitReset, setNotifyLimitReset] = useState(true);
  const [notifySaved, setNotifySaved] = useState('');

  // Pricing & local profile state
  const [displayName, setDisplayName] = useState('');
  const [openaiPlan, setOpenaiPlan] = useState('20');
  const [claudePlan, setClaudePlan] = useState('20');
  const [googlePlan, setGooglePlan] = useState('0');
  const [settingsSaved, setSettingsSaved] = useState('');
  const [customPricing, setCustomPricing] = useState<CustomPricingRule[]>([]);
  const [pattern, setPattern] = useState('');
  const [inPrice, setInPrice] = useState('');
  const [outPrice, setOutPrice] = useState('');
  const [cachePrice, setCachePrice] = useState('');
  const [dbStatsText, setDbStatsText] = useState('Loading local storage stats...');

  // Desktop startup & appearance state
  const [loginItemSupported, setLoginItemSupported] = useState(false);
  const [loginItemEnabled, setLoginItemEnabled] = useState(false);

  // Account, Data, Updates, Guide state
  const [cloudSyncStatus, setCloudSyncStatus] = useState('Checking desktop publication status...');
  // Both are on unless the person turns them off here (E3, ADR 0049).
  const [features, setFeatures] = useState<{ scheduled_messages_enabled: boolean; account_sync_enabled: boolean } | null>(
    null,
  );
  const [lastUpdatedText, setLastUpdatedText] = useState('Last successful update: not yet');
  const [dataBusy, setDataBusy] = useState(false);
  const [updateStatusText, setUpdateStatusText] = useState('Checking update status...');
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(true);
  const [updateReady, setUpdateReady] = useState(false);
  const [appBuildText, setAppBuildText] = useState('Checking installed version...');

  const loadProviders = useCallback(async () => {
    try {
      const overview = await fetchLocalJson<ProviderOverview[]>('/api/v1/providers/overview');
      if (Array.isArray(overview) && providerListRef.current) {
        updateProviderSources(providerListRef.current, overview);
      }
    } catch {
      if (providerListRef.current && !providerListRef.current.children.length) {
        providerListRef.current.textContent = 'Provider sources unavailable.';
      }
    }
  }, []);

  const loadRemote = useCallback(async () => {
    try {
      const res = await fetchLocalJson<RemoteDevicesResponse>('/api/v1/remote/devices');
      setDesktopId(res.desktop_id ?? null);
      setAccountAccess(Boolean(res.account_access));
      setRemoteEnabled(Boolean(res.remote_enabled));
      setDevices(Array.isArray(res.devices) ? res.devices : []);
      if (res.directory_error) {
        setRemoteStatus('Device discovery is unavailable. Check your connection.');
      } else if (!res.desktop_id) {
        setRemoteStatus('Sign in to your account to make this desktop available.');
      } else if (!res.account_access) {
        setRemoteStatus(
          'Confirm account access to make this desktop available to signed-in devices.',
        );
      } else if (!res.remote_enabled) {
        setRemoteStatus('Account access is disabled for this desktop.');
      } else {
        setRemoteStatus(
          'This desktop is accessible from any device signed in to your account.',
        );
      }
    } catch {
      setRemoteStatus('Devices unavailable. Reload to try again.');
    }
  }, []);

  const loadProData = useCallback(async () => {
    try {
      const [qRes, rulesRes, alertsRes, relayRes] = await Promise.all([
        fetchLocalJson<ResumeQueueEntry[]>('/api/v1/resume-queue'),
        fetchLocalJson<GuardrailRule[]>('/api/v1/rules'),
        fetchLocalJson<GuardrailAlert[]>('/api/v1/alerts'),
        fetchLocalJson<{ relays?: RelayEntry[] }>('/api/v1/pro/relay'),
      ]);
      setResumeQueue(Array.isArray(qRes) ? qRes : []);
      setRules(Array.isArray(rulesRes) ? rulesRes : []);
      setAlerts(Array.isArray(alertsRes) ? alertsRes : []);
      setRelays(Array.isArray(relayRes?.relays) ? relayRes.relays : []);
      setAutomationStatus('');
    } catch {
      setAutomationStatus(
        'Automation and guardrail data is unavailable. Retry Reload view when the daemon is reachable.',
      );
    }
  }, []);

  const loadFeatures = useCallback(async () => {
    try {
      setFeatures(
        await fetchLocalJson<{ scheduled_messages_enabled: boolean; account_sync_enabled: boolean }>(
          '/api/v1/settings/features',
        ),
      );
    } catch {
      setFeatures(null);
    }
  }, []);

  const saveFeature = async (key: 'scheduled_messages_enabled' | 'account_sync_enabled', on: boolean) => {
    const next = { ...(features ?? { scheduled_messages_enabled: true, account_sync_enabled: true }), [key]: on };
    setFeatures(next);
    try {
      await fetchLocalMutation('/api/v1/settings/features', { method: 'POST', body: next });
    } catch {
      await loadFeatures();
    }
  };

  const loadCoreSettings = useCallback(async () => {
    void fetchLocalJson<{ version?: string }>('/health')
      .then((health) => {
        setAppBuildText(`Gnomcode ${health?.version || 'version unavailable'}`);
      })
      .catch(() => {
        setAppBuildText(
          'Installed version unavailable. Reload when the daemon is connected.',
        );
      });

    try {
      const [user, stats, pricingRes, notifyCfg, updateStatus, syncResult] =
        await Promise.all([
          fetchLocalJson<{
            display_name?: string;
            openai_plan_usd?: number;
            claude_plan_usd?: number;
            google_plan_usd?: number;
          }>('/api/v1/user'),
          fetchLocalJson<{
            total_turns?: number;
            total_projects?: number;
            db_size_bytes?: number;
          }>('/api/v1/stats'),
          fetchLocalJson<{ custom_pricing?: CustomPricingRule[] }>('/api/v1/pricing/custom').catch(
            () => ({ custom_pricing: [] }),
          ),
          fetchLocalJson<{
            approval_waiting?: boolean;
            turn_finished?: boolean;
            turn_finished_min_minutes?: number;
            limit_low?: boolean;
            limit_reset?: boolean;
          }>('/api/v1/settings/notifications').catch(() => null),
          fetchLocalJson<UpdateStatusResponse>('/api/v1/update/status').catch(() => null),
          fetchLocalJson<{ message?: string; last_success_at?: string }>('/api/v1/sync/status').catch(
            () => null,
          ),
        ]);

      if (user) {
        setDisplayName(user.display_name || '');
        setOpenaiPlan(String(user.openai_plan_usd ?? 20));
        setClaudePlan(String(user.claude_plan_usd ?? 20));
        setGooglePlan(String(user.google_plan_usd ?? 0));
      }
      if (
        stats &&
        [stats.total_turns, stats.total_projects, stats.db_size_bytes].every(Number.isFinite)
      ) {
        setDbStatsText(
          `Local Storage: ${stats.total_turns} turns EXACT across ${stats.total_projects} projects EXACT (${((stats.db_size_bytes ?? 0) / 1024 / 1024).toFixed(2)} MB EXACT)`,
        );
      } else {
        setDbStatsText('Storage measurements unavailable · UNKNOWN');
      }

      setCustomPricing(Array.isArray(pricingRes?.custom_pricing) ? pricingRes.custom_pricing : []);

      if (notifyCfg) {
        setNotifyApproval(Boolean(notifyCfg.approval_waiting));
        setNotifyTurnFinished(Boolean(notifyCfg.turn_finished));
        setNotifyTurnMinutes(
          Number.isFinite(notifyCfg.turn_finished_min_minutes)
            ? String(notifyCfg.turn_finished_min_minutes)
            : '2',
        );
        setNotifyLimitLow(Boolean(notifyCfg.limit_low));
        setNotifyLimitReset(Boolean(notifyCfg.limit_reset));
      }

      if (updateStatus) {
        setAutoUpdateEnabled(updateStatus.auto_update_enabled !== false);
        setUpdateReady(updateStatus.state === 'ready');
        const version = updateStatus.available_version
          ? `v${updateStatus.available_version}`
          : 'new version';
        if (updateStatus.state === 'ready') {
          setUpdateStatusText(`Update available: ${version} ready to install on quit or restart.`);
        } else if (updateStatus.state === 'downloading') {
          setUpdateStatusText(`Update available: ${version} downloading in background.`);
        } else if (updateStatus.state === 'available') {
          setUpdateStatusText(`Update available: ${version} queued for download.`);
        } else if (updateStatus.state === 'disabled') {
          setUpdateStatusText('Automatic update checks off.');
        } else if (updateStatus.state === 'error') {
          setUpdateStatusText(`Update check failed (${updateStatus.last_error || 'offline'}).`);
        } else {
          setUpdateStatusText(`Up to date (v${updateStatus.current_version || 'unknown'}).`);
        }
      } else {
        setUpdateStatusText('Update status unavailable.');
      }

      if (syncResult?.message) {
        setCloudSyncStatus(
          `${syncResult.message} Last published: ${syncResult.last_success_at ? new Date(syncResult.last_success_at).toLocaleString() : 'not yet'}.`,
        );
      } else {
        setCloudSyncStatus('Local monitoring does not require an account connection.');
      }
      setLastUpdatedText(`Last updated: ${new Date().toLocaleTimeString()}`);
    } catch {
      setDbStatsText(
        'Local settings are unavailable. Retry Reload view when the daemon is reachable.',
      );
    }
  }, []);

  const refreshAll = useCallback(async () => {
    await Promise.all([
      loadProviders(),
      loadRemote(),
      loadProData(),
      loadCoreSettings(),
      loadFeatures(),
    ]);
  }, [loadCoreSettings, loadFeatures, loadProData, loadProviders, loadRemote]);

  useEffect(() => {
    void refreshAll();
    const timer = window.setInterval(() => {
      if (isSettingsVisible()) {
        void loadProviders();
        void loadRemote();
      }
    }, POLL_INTERVAL_MS);

    const onViewChange = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'settings') {
        void refreshAll();
      }
    };
    const onReload = (event: Event) => {
      const detail = (event as CustomEvent<{ view?: string }>).detail;
      if (detail?.view === 'settings') {
        void refreshAll();
      }
    };

    window.addEventListener('gnomcode:view-change', onViewChange);
    window.addEventListener('gnomcode:reload-view', onReload);
    return () => {
      window.clearInterval(timer);
      window.removeEventListener('gnomcode:view-change', onViewChange);
      window.removeEventListener('gnomcode:reload-view', onReload);
    };
  }, [loadProviders, loadRemote, refreshAll]);

  const handleCycleTheme = () => {
    const next: ThemeMode =
      themeMode === 'system' ? 'light' : themeMode === 'light' ? 'dark' : 'system';
    applyThemeMode(next);
    setThemeMode(next);
  };

  const executePendingDeviceAction = async () => {
    if (!pendingDeviceAction) return;
    const action = pendingDeviceAction;
    setPendingDeviceAction(null);
    setRemoteBusy(true);
    try {
      if (action.kind === 'toggle_access') {
        await fetchLocalMutation('/api/v1/remote/devices', {
          method: 'POST',
          body: {
            type: action.enable ? 'enable_account_access' : 'disable_account_access',
            remote_access_confirmed: action.enable,
          },
        });
      } else {
        await fetchLocalMutation('/api/v1/remote/devices', {
          method: 'POST',
          body: { type: 'remove_device', device_id: action.id },
        });
      }
      await loadRemote();
    } catch {
      setRemoteStatus('Could not update device access. Refresh and retry.');
    } finally {
      setRemoteBusy(false);
    }
  };

  const handleSaveRule = async (event: FormEvent) => {
    event.preventDefault();
    setRuleBusy(true);
    setRuleStatus('Saving rule...');
    try {
      await fetchLocalMutation('/api/v1/rules', {
        method: 'POST',
        body: {
          id: `rule-${Date.now()}`,
          project_id: null,
          agent: ruleAgent || null,
          max_spend_usd: ruleSpend ? Number(ruleSpend) : null,
          max_turns: ruleTurns ? Number(ruleTurns) : null,
          action: ruleAction,
        },
      });
      setRuleStatus('Rule saved. It applies to each matching session. Alerts appear below.');
      await loadProData();
    } catch (error) {
      setRuleStatus(
        `Rule could not be saved. ${error instanceof Error ? error.message : 'Request failed.'}`,
      );
    } finally {
      setRuleBusy(false);
    }
  };

  const handleSaveNotifications = async () => {
    const minutesRaw = parseInt(notifyTurnMinutes, 10);
    await fetchLocalMutation('/api/v1/settings/notifications', {
      method: 'POST',
      body: {
        approval_waiting: notifyApproval,
        turn_finished: notifyTurnFinished,
        turn_finished_min_minutes: Number.isFinite(minutesRaw)
          ? Math.max(0, Math.min(1440, minutesRaw))
          : 2,
        limit_low: notifyLimitLow,
        limit_reset: notifyLimitReset,
      },
    });
    setNotifySaved('Saved notification preferences.');
    window.setTimeout(() => setNotifySaved(''), 3000);
  };

  const handleSaveProfile = async () => {
    await fetchLocalMutation('/api/v1/user', {
      method: 'POST',
      body: {
        id: 'local_user',
        username: 'developer',
        display_name: displayName || 'Developer',
        openai_plan_usd: parseFloat(openaiPlan) || 0,
        claude_plan_usd: parseFloat(claudePlan) || 0,
        google_plan_usd: parseFloat(googlePlan) || 0,
        other_plan_usd: 0,
        updated_at: new Date().toISOString(),
      },
    });
    setSettingsSaved('Saved successfully!');
    window.setTimeout(() => setSettingsSaved(''), 3000);
  };

  const handleAddPricing = async () => {
    const trimmed = pattern.trim();
    if (!trimmed) return;
    await fetchLocalMutation('/api/v1/pricing/custom', {
      method: 'POST',
      body: {
        model_pattern: trimmed,
        input_per_mtok_usd: parseFloat(inPrice) || 0,
        output_per_mtok_usd: parseFloat(outPrice) || 0,
        cache_read_per_mtok_usd: parseFloat(cachePrice) || 0,
        updated_at: new Date().toISOString(),
      },
    });
    setPattern('');
    setInPrice('');
    setOutPrice('');
    setCachePrice('');
    await loadCoreSettings();
  };

  const handleResyncData = async () => {
    setDataBusy(true);
    setLastUpdatedText('Reconciling local data...');
    try {
      await fetchLocalMutation('/api/v1/reconcile', { method: 'POST' });
      await refreshAll();
    } catch {
      setLastUpdatedText('Reconciliation failed');
    } finally {
      setDataBusy(false);
    }
  };

  const handleCheckUpdate = async () => {
    setUpdateStatusText('Checking for updates...');
    try {
      await fetchLocalMutation('/api/v1/update/check', { method: 'POST' });
      await loadCoreSettings();
    } catch {
      setUpdateStatusText('Update check failed.');
    }
  };

  const handleToggleAutoUpdate = async (enabled: boolean) => {
    setAutoUpdateEnabled(enabled);
    try {
      await fetchLocalMutation('/api/v1/settings/update', {
        method: 'POST',
        body: { enabled },
      });
      await loadCoreSettings();
    } catch {
      await loadCoreSettings();
    }
  };

  const trimmedQuery = query.trim();
  const visible = SETTINGS_SECTIONS.filter((section) => sectionMatches(section, query));
  const activeId = visible.some((section) => section.id === selectedId) ? selectedId : (visible[0]?.id ?? selectedId);
  const selectCategory = (id: string) => {
    setSelectedId(id);
    if (typeof window !== 'undefined' && window.location.hash !== `#${id}`) {
      window.history.replaceState(null, '', `#${id}`);
    }
    revealSection(id);
  };
  const shown = (id: string) => (trimmedQuery ? visible.some((section) => section.id === id) : id === activeId);

  return (
    <div className="sp-settings-layout">
      <nav className="sp-settings-nav" aria-label="Settings sections">
        <input
          id="settings-search"
          type="search"
          className="sp-settings-search"
          placeholder="Search settings"
          aria-label="Search settings"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        <div className="sp-settings-mobile-bar">
          <label htmlFor="settings-mobile-category" className="sr-only">
            Settings category
          </label>
          <select
            id="settings-mobile-category"
            className="sp-settings-mobile-select"
            value={activeId}
            onChange={(event) => selectCategory(event.target.value)}
          >
            {visible.map((section) => (
              <option key={section.id} value={section.id}>
                {section.title}
              </option>
            ))}
          </select>
        </div>
        <ul>
          {visible.map((section) => {
            const isActive = !trimmedQuery && section.id === activeId;
            return (
              <li key={section.id}>
                <a
                  href={`#${section.id}`}
                  aria-current={isActive ? 'page' : undefined}
                  className={isActive ? 'is-active' : undefined}
                  onClick={(event) => {
                    event.preventDefault();
                    selectCategory(section.id);
                  }}
                >
                  {section.title}
                </a>
              </li>
            );
          })}
        </ul>
        {visible.length === 0 ? <p role="status">No settings match.</p> : null}
      </nav>
    <div className="sp-settings-root">
      <section id="settings-account" className="sp-settings-section" hidden={!shown('settings-account')}>
        <h2>Account</h2>
        <div className="sp-setting-row">
          <div>
            <p className="sp-setting-label">{accountState?.paired ? accountState.email || 'Signed in' : 'Not signed in'}</p>
            <p className="sp-setting-desc">{cloudSyncStatus}</p>
          </div>
          <button className="btn-control" type="button" onClick={() => navigateTo('auth')}>
            Manage account
          </button>
        </div>
        <div className="sp-setting-row">
          <div>
            <p className="sp-setting-label">Share this desktop's snapshot with your account</p>
            <p className="sp-setting-desc">
              Publishes session names, project names and usage totals so the web console can show this desktop while you
              are away. Chat content is never published. Off means the web console shows nothing for this desktop.
            </p>
          </div>
          <label className="sp-notify-toggle" htmlFor="settings-account-sync">
            <input
              id="settings-account-sync"
              type="checkbox"
              checked={features?.account_sync_enabled ?? true}
              disabled={!features}
              onChange={(event) => void saveFeature('account_sync_enabled', event.target.checked)}
            />
            <span>Publish snapshot</span>
          </label>
        </div>
        <div className="sp-setting-row">
          <div>
            <p className="sp-setting-label">Scheduled messages</p>
            <p className="sp-setting-desc">
              Lets you queue a message for a later time. It is sent from this computer while Gnomcode is running.
            </p>
          </div>
          <label className="sp-notify-toggle" htmlFor="settings-scheduled-messages">
            <input
              id="settings-scheduled-messages"
              type="checkbox"
              checked={features?.scheduled_messages_enabled ?? true}
              disabled={!features}
              onChange={(event) => void saveFeature('scheduled_messages_enabled', event.target.checked)}
            />
            <span>Send later</span>
          </label>
        </div>
        {accountState?.paired ? (
          <div className="sp-setting-row sp-danger-zone">
            <div>
              <p className="sp-setting-label">Sign out</p>
              <p className="sp-setting-desc">Disconnects this desktop and stops Remote. Local history stays.</p>
            </div>
            <button
              id="settings-sign-out"
              className="btn-control sp-btn-danger"
              type="button"
              onClick={async () => {
                if (await signOutDesktop()) setAccountState({ paired: false });
              }}
            >
              Sign out
            </button>
          </div>
        ) : null}
      </section>

      <section id="settings-providers" className="sp-settings-section" hidden={!shown('settings-providers')}>
        <h2>Providers</h2>
        <ProviderSetupPanel />
        <h3>Limit sources</h3>
        <div ref={providerListRef} className="sp-provider-sources">
          Loading providers...
        </div>
      </section>

      <section id="settings-appearance" className="profile-appearance sp-settings-section" hidden={!shown('settings-appearance')}>
        <h2>Appearance</h2>
        <button
          className="theme-control"
          type="button"
          onClick={handleCycleTheme}
          aria-label={`Theme: ${themeMode}. Switch mode`}
        >
          {`Theme / ${themeMode}`}
        </button>
        {loginItemSupported ? (
          <div className="sp-setting-row">
            <label className="sp-notify-toggle" htmlFor="settings-start-at-login">
              <input
                id="settings-start-at-login"
                type="checkbox"
                checked={loginItemEnabled}
                onChange={(event) => {
                  const next = event.target.checked;
                  setLoginItemEnabled(next);
                  void window.gnomcodeDesktop?.setLoginItem?.(next).then(
                    (saved) => setLoginItemEnabled(Boolean(saved)),
                    () => setLoginItemEnabled(!next),
                  );
                }}
              />
              <span>Start Gnomcode when I sign in</span>
            </label>
          </div>
        ) : null}
      </section>

      <section id="settings-remote" className="profile-devices sp-settings-section" aria-label="Remote and devices" hidden={!shown('settings-remote')}>
        <h2>Remote and devices</h2>
        <p role="status">{remoteStatus}</p>
        {desktopId ? (
          <p>
            <button
              type="button"
              className="btn-control"
              disabled={remoteBusy}
              onClick={() =>
                setPendingDeviceAction({
                  kind: 'toggle_access',
                  enable: !(accountAccess && remoteEnabled),
                })
              }
            >
              {accountAccess && remoteEnabled
                ? 'Disable account access'
                : 'Enable account access'}
            </button>
          </p>
        ) : null}

        {pendingDeviceAction ? (
          <div className="sp-confirm-inline" role="region" aria-label="Confirm device change">
            <p>
              {pendingDeviceAction.kind === 'remove'
                ? `Remove ${pendingDeviceAction.name}? It will be disconnected and cannot reconnect without pairing again.`
                : pendingDeviceAction.enable
                  ? 'Linking this native app makes its available conversations and supported session controls accessible from any device signed in to your account.'
                  : 'Disable access to this desktop for every device signed in to your account?'}
            </p>
            <div className="dialog-actions">
              <button
                type="button"
                className="btn-control"
                onClick={() => setPendingDeviceAction(null)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn-primary"
                onClick={() => void executePendingDeviceAction()}
              >
                Continue
              </button>
            </div>
          </div>
        ) : null}

        <div>
          {devices.map((device) => (
            <div key={device.id} className="sp-device-row">
              <div>
                <strong>{device.name}</strong>
                <p>{device.connected ? 'Connected' : 'Saved connection'}</p>
              </div>
              <button
                type="button"
                className="btn-control sp-tone-red"
                disabled={remoteBusy}
                onClick={() =>
                  setPendingDeviceAction({
                    kind: 'remove',
                    id: device.id,
                    name: device.name,
                  })
                }
              >
                Remove
              </button>
            </div>
          ))}
        </div>
      </section>

      <section id="settings-automations" className="sp-settings-section" hidden={!shown('settings-automations')}>
        <h2>Automations</h2>
        {automationStatus ? <p role="status">{automationStatus}</p> : null}

        <RoutingPolicySetting />

        <div className="table-container">
          <div className="table-header-controls">
            <h3>Reconnect after a limit</h3>
          </div>
          <p>Set up reconnection or scheduling from a conversation.</p>
          <details className="sp-help-details">
            <summary>How it works</summary>
            <p>
              Open Schedule and automation in your conversation. Gnomcode can reconnect when fresh
              quota confirms a reset; send your next message to continue. Your laptop must stay
              awake and the agent must be signed in.
            </p>
          </details>
          <button
            type="button"
            className="btn-control"
            onClick={() => navigateTo('sessions')}
          >
            Open conversations
          </button>
          <table>
            <thead>
              <tr>
                <th>Session ID</th>
                <th>Agent</th>
                <th>Working Dir</th>
                <th>Prompt</th>
                <th>Max Runs</th>
                <th>Runs Done</th>
                <th>Eligible After</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {resumeQueue.length ? (
                resumeQueue.map((entry) => (
                  <tr key={entry.session_id}>
                    <td className="sp-mono-meta sp-mono-meta--text">{entry.session_id}</td>
                    <td>
                      <span className="badge">{entry.agent}</span>
                    </td>
                    <td className="sp-mono-meta sp-mono-meta--sans">{entry.cwd || ''}</td>
                    <td>{entry.resume_prompt || ''}</td>
                    <td>{entry.max_runs}</td>
                    <td>{entry.runs_completed}</td>
                    <td className="sp-mono-meta sp-mono-meta--sans">
                      {formatAgeLabel(entry.eligible_after)}
                    </td>
                    <td>
                      <button
                        type="button"
                        className="btn-control"
                        onClick={() =>
                          void fetchLocalMutation('/api/v1/resume-queue/disarm', {
                            method: 'POST',
                            body: { session_id: entry.session_id },
                          }).then(() => loadProData())
                        }
                      >
                        Disarm
                      </button>
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={8} className="sp-muted-cell">
                    No reconnections scheduled.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <div className="table-container">
          <div className="table-header-controls">
            <div>
              <h3>Usage rules</h3>
              <p>Get an alert when a session reaches a spend or turn limit.</p>
            </div>
          </div>
          <form className="sp-rule-form" onSubmit={(event) => void handleSaveRule(event)}>
            <label>
              Agent
              <select value={ruleAgent} onChange={(event) => setRuleAgent(event.target.value)}>
                <option value="">All agents</option>
                <option value="claude-code">Claude Code</option>
                <option value="codex">Codex</option>
                <option value="antigravity">Antigravity</option>
              </select>
            </label>
            <label>
              Spend limit (USD)
              <input
                type="number"
                min="0.01"
                step="0.01"
                placeholder="e.g. 5"
                value={ruleSpend}
                onChange={(event) => setRuleSpend(event.target.value)}
              />
            </label>
            <label>
              Turn limit
              <input
                type="number"
                min="1"
                step="1"
                placeholder="e.g. 30"
                value={ruleTurns}
                onChange={(event) => setRuleTurns(event.target.value)}
              />
            </label>
            <label>
              When reached
              <select value={ruleAction} onChange={(event) => setRuleAction(event.target.value)}>
                <option value="Warn">Send an alert</option>
                <option value="HardStop">Stop via Claude hook; alert other agents</option>
              </select>
            </label>
            <button className="btn-control" type="submit" disabled={ruleBusy}>
              Add rule
            </button>
            {ruleStatus ? <p role="status">{ruleStatus}</p> : null}
          </form>
          <details className="sp-help-details">
            <summary>About usage rules</summary>
            <p>
              Spend is estimated, not a billing cap. Stops require the Claude Code hook; other
              agents receive alerts. If Gnomcode is unavailable, hooks allow the agent to continue.
            </p>
          </details>
          <table>
            <thead>
              <tr>
                <th>Rule ID</th>
                <th>Agent Scope</th>
                <th>Max Spend (USD)</th>
                <th>Max Turns</th>
                <th>Enforcement</th>
                <th>Action</th>
              </tr>
            </thead>
            <tbody>
              {rules.length ? (
                rules.map((rule) => (
                  <tr key={rule.id}>
                    <td className="sp-mono-meta sp-mono-meta--text sp-strong-cell">{rule.id}</td>
                    <td>{rule.agent || 'All Agents'}</td>
                    <td>{rule.max_spend_usd ? `$${rule.max_spend_usd.toFixed(2)}` : 'None'}</td>
                    <td>{rule.max_turns || 'None'}</td>
                    <td>
                      <span
                        className={`badge ${rule.action === 'HardStop' ? 'sp-tone-red' : 'sp-tone-accent'}`}
                      >
                        {rule.action === 'HardStop'
                          ? 'Claude hook stop; other agents alert'
                          : rule.action}
                      </span>
                    </td>
                    <td>
                      <button
                        type="button"
                        className="btn-control"
                        onClick={() =>
                          void fetchLocalMutation(`/api/v1/rules/${encodeURIComponent(rule.id)}`, {
                            method: 'DELETE',
                          }).then(() => loadProData())
                        }
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))
              ) : (
                <tr>
                  <td colSpan={6} className="sp-muted-cell">
                    No usage rules yet.
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <details className="sp-alert-history">
          <summary>Recent alerts</summary>
          <div className="table-container">
            <table>
              <thead>
                <tr>
                  <th>Session</th>
                  <th>Rule ID</th>
                  <th>Enforcement</th>
                  <th>Timestamp</th>
                </tr>
              </thead>
              <tbody>
                {alerts.length ? (
                  alerts.map((alert, index) => (
                    <tr key={`${alert.session_id}-${alert.rule_id}-${index}`}>
                      <td className="sp-mono-meta sp-mono-meta--text">{alert.session_id}</td>
                      <td className="sp-strong-cell">{alert.rule_id}</td>
                      <td>
                        <span
                          className={`badge ${alert.action === 'hard_stop' ? 'sp-tone-red' : 'sp-tone-accent'}`}
                        >
                          {alert.action === 'hard_stop'
                            ? 'Stop requested (Claude hook only)'
                            : alert.action}
                        </span>
                      </td>
                      <td className="sp-mono-meta sp-mono-meta--sans">
                        {formatAgeLabel(alert.created_at)}
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={4} className="sp-muted-cell">
                      No alerts yet.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </details>

        <details className="sp-help-details">
          <summary>Rotation history</summary>
          <p>
            Providers manage their own context compaction. To change model or provider, use the
            selector in your Gnomcode conversation.
          </p>
          <div className="sp-table-caption">Rotation history</div>
          <div className="table-container">
            <table>
              <thead>
                <tr>
                  <th>Session ID</th>
                  <th>Project</th>
                  <th>Source -&gt; Target</th>
                  <th>Context Fill</th>
                  <th>Status</th>
                  <th>Triggered</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {relays.length ? (
                  relays.map((relay) => (
                    <tr key={relay.id}>
                      <td className="sp-mono-meta sp-mono-meta--text">{relay.session_id}</td>
                      <td className="sp-strong-cell">{relay.project_name || ''}</td>
                      <td>{`${relay.source_agent} -> ${relay.target_agent}`}</td>
                      <td
                        className={`sp-strong-cell ${relay.fill_pct >= 90 ? 'sp-tone-red' : 'sp-tone-yellow'}`}
                      >
                        {`${relay.fill_pct}% `}
                        <span className="trust-mark">EST</span>
                      </td>
                      <td>
                        <span className="badge">{relay.status.toUpperCase()}</span>
                      </td>
                      <td className="sp-mono-meta sp-mono-meta--sans">
                        {formatAgeLabel(relay.created_at)}
                      </td>
                      <td>
                        {relay.status === 'pending' ? (
                          pendingRelayId === relay.id ? (
                            <>
                              <button
                                type="button"
                                className="btn-primary sp-row-action sp-row-action--gap"
                                onClick={() => {
                                  setPendingRelayId(null);
                                  void fetchLocalMutation('/api/v1/pro/relay/execute', {
                                    method: 'POST',
                                    body: { id: relay.id },
                                  }).then(() => loadProData());
                                }}
                              >
                                Confirm rotate
                              </button>
                              <button
                                type="button"
                                className="btn-control sp-row-action"
                                onClick={() => setPendingRelayId(null)}
                              >
                                Cancel
                              </button>
                            </>
                          ) : (
                            <>
                              <button
                                type="button"
                                className="btn-primary sp-row-action sp-row-action--gap"
                                onClick={() => setPendingRelayId(relay.id)}
                              >
                                Handoff &amp; Rotate
                              </button>
                              <button
                                type="button"
                                className="btn-control sp-row-action"
                                onClick={() =>
                                  void fetchLocalMutation('/api/v1/pro/relay/dismiss', {
                                    method: 'POST',
                                    body: { id: relay.id },
                                  }).then(() => loadProData())
                                }
                              >
                                Dismiss
                              </button>
                            </>
                          )
                        ) : (
                          <span className="sp-mono-meta">{relay.status}</span>
                        )}
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={7} className="sp-muted-cell">
                      No context handoffs recorded. Open a supported session to switch agents.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </details>
      </section>

      <section id="settings-notifications" className="sp-settings-section" aria-label="Notifications" hidden={!shown('settings-notifications')}>
        <h2>Notifications</h2>
        <p className="sp-field-note">
          Choose which local desktop and paired phone alerts are enabled. Alerts include only agent
          name, session title, and event.
        </p>
        <div className="sp-notify-list">
          <label className="sp-notify-toggle">
            <input
              type="checkbox"
              checked={notifyApproval}
              onChange={(event) => setNotifyApproval(event.target.checked)}
            />
            <span>Needs approval (agent waiting on permission)</span>
          </label>
          <label className="sp-notify-toggle">
            <input
              type="checkbox"
              checked={notifyTurnFinished}
              onChange={(event) => setNotifyTurnFinished(event.target.checked)}
            />
            <span>Turn finished after long work</span>
          </label>
          <div className="sp-notify-minutes">
            <label>
              Minimum turn duration (minutes)
              <input
                type="number"
                min="0"
                max="1440"
                step="1"
                value={notifyTurnMinutes}
                onChange={(event) => setNotifyTurnMinutes(event.target.value)}
              />
            </label>
          </div>
          <label className="sp-notify-toggle">
            <input
              type="checkbox"
              checked={notifyLimitLow}
              onChange={(event) => setNotifyLimitLow(event.target.checked)}
            />
            <span>Limit low (under 20% capacity left)</span>
          </label>
          <label className="sp-notify-toggle">
            <input
              type="checkbox"
              checked={notifyLimitReset}
              onChange={(event) => setNotifyLimitReset(event.target.checked)}
            />
            <span>Limit reset (fresh capacity after reset)</span>
          </label>
        </div>
        <div className="sp-form-actions">
          <button
            type="button"
            className="btn-primary"
            onClick={() => void handleSaveNotifications()}
          >
            Save notifications
          </button>
          {notifySaved ? <span role="status">{notifySaved}</span> : null}
        </div>
      </section>

      <section id="settings-pricing" className="sp-settings-section" hidden={!shown('settings-pricing')}>
        <h2>Pricing and subscription costs</h2>
        <div className="settings-grid">
          <div className="card">
            <div className="section-title sp-title-gap">Local profile and subscriptions</div>
            <div className="form-group">
              <label>
                Display Name
                <input
                  type="text"
                  placeholder="Developer"
                  value={displayName}
                  onChange={(event) => setDisplayName(event.target.value)}
                />
              </label>
            </div>
            <div className="form-group">
              <label>
                OpenAI Monthly Plan ($/mo)
                <input
                  type="number"
                  step="1"
                  value={openaiPlan}
                  onChange={(event) => setOpenaiPlan(event.target.value)}
                />
              </label>
            </div>
            <div className="form-group">
              <label>
                Anthropic / Claude Monthly Plan ($/mo)
                <input
                  type="number"
                  step="1"
                  value={claudePlan}
                  onChange={(event) => setClaudePlan(event.target.value)}
                />
              </label>
            </div>
            <div className="form-group">
              <label>
                Google Monthly Plan ($/mo)
                <input
                  type="number"
                  step="1"
                  value={googlePlan}
                  onChange={(event) => setGooglePlan(event.target.value)}
                />
              </label>
            </div>
            <div className="sp-form-actions">
              <button
                type="button"
                className="btn-primary"
                onClick={() => void handleSaveProfile()}
              >
                Save Preferences
              </button>
              {settingsSaved ? <span role="status">{settingsSaved}</span> : null}
            </div>
          </div>

          <div className="card">
            <div className="section-title sp-title-gap">Custom Model Pricing Overrides</div>
            <div className="sp-pricing-list">
              {customPricing.length ? (
                <div className="sp-pricing-rows">
                  {customPricing.map((cp) => (
                    <div key={cp.model_pattern} className="sp-pricing-row">
                      <span>
                        <strong>{cp.model_pattern}</strong>
                        {`: In $${cp.input_per_mtok_usd.toFixed(2)} | Out $${cp.output_per_mtok_usd.toFixed(2)} | Cache $${cp.cache_read_per_mtok_usd.toFixed(2)}`}
                      </span>
                      <button
                        type="button"
                        className="btn-control sp-row-action sp-tone-red"
                        onClick={() =>
                          void fetchLocalMutation(
                            `/api/v1/pricing/custom?pattern=${encodeURIComponent(cp.model_pattern)}`,
                            { method: 'DELETE' },
                          ).then(() => loadCoreSettings())
                        }
                      >
                        Remove
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <span className="sp-tone-muted">
                  No custom rate overrides configured (using default catalog).
                </span>
              )}
            </div>

            <div className="sp-field-note">Add New Model Rate ($/1M tokens):</div>
            <div className="profile-pricing-fields">
              <input
                type="text"
                aria-label="Model name or pattern"
                placeholder="Model (e.g. claude-3-5)"
                value={pattern}
                onChange={(event) => setPattern(event.target.value)}
              />
              <input
                type="number"
                aria-label="Input price per million tokens"
                placeholder="In $"
                step="0.1"
                value={inPrice}
                onChange={(event) => setInPrice(event.target.value)}
              />
              <input
                type="number"
                aria-label="Output price per million tokens"
                placeholder="Out $"
                step="0.1"
                value={outPrice}
                onChange={(event) => setOutPrice(event.target.value)}
              />
              <input
                type="number"
                aria-label="Cached input price per million tokens"
                placeholder="Cache $"
                step="0.1"
                value={cachePrice}
                onChange={(event) => setCachePrice(event.target.value)}
              />
              <button
                type="button"
                className="btn-control sp-add-rate"
                onClick={() => void handleAddPricing()}
              >
                + Add
              </button>
            </div>

            <div className="sp-storage-note">
              <div className="section-title sp-title-gap-sm">Local Flight Recorder Storage</div>
              <div>Database: ~/.gnomcode/state.db</div>
              <div className="sp-storage-stats">{dbStatsText}</div>
            </div>
          </div>
        </div>
      </section>

      <section id="settings-privacy" className="profile-data sp-settings-section" hidden={!shown('settings-privacy')}>
        <h2>Privacy and data</h2>
        <p aria-live="polite">{lastUpdatedText}</p>
        <div className="console-hero-actions">
          <button
            className="btn-control"
            type="button"
            disabled={dataBusy}
            onClick={() => void refreshAll()}
          >
            Reload view
          </button>
          <button
            className="btn-control"
            type="button"
            disabled={dataBusy}
            onClick={() => void handleResyncData()}
          >
            Resync local data
          </button>
        </div>
      </section>

      <section id="settings-updates" className="sp-settings-section" hidden={!shown('settings-updates')}>
        <h2>Updates</h2>
        <p role="status">{updateStatusText}</p>
        <label className="sp-notify-toggle">
          <input
            type="checkbox"
            checked={autoUpdateEnabled}
            onChange={(event) => void handleToggleAutoUpdate(event.target.checked)}
          />
          <span>Check for desktop updates automatically</span>
        </label>
        <div className="sp-settings-resources">
          <button type="button" className="btn-control" onClick={() => void handleCheckUpdate()}>
            Check now
          </button>
          {updateReady && window.gnomcodeDesktop?.restartToUpdate ? (
            <button
              type="button"
              className="btn-control"
              onClick={() => void window.gnomcodeDesktop?.restartToUpdate?.()}
            >
              Restart to update
            </button>
          ) : null}
        </div>
      </section>

      <section id="settings-about" className="sp-settings-section" hidden={!shown('settings-about')}>
        <h2>About</h2>
        <p role="status">{appBuildText}</p>
        <div className="sp-settings-resources">
          <button
            type="button"
            className="btn-control"
            onClick={startOnboarding}
          >
            Getting started
          </button>
          <a href={`${baseUrl}/downloads`} target="_blank" rel="noopener noreferrer">
            Download desktop app
          </a>
          <a href={`${baseUrl}/docs`} target="_blank" rel="noopener noreferrer">
            Documentation
          </a>
          <a href={`${baseUrl}/pricing`} target="_blank" rel="noopener noreferrer">
            Pricing
          </a>
        </div>
      </section>
    </div>
    </div>
  );
}

/** Global limit-reached routing policy (`/api/v1/routing/policy`); the daemon decides and switches. */
function RoutingPolicySetting() {
  const [policy, setPolicy] = useState('');
  const [status, setStatus] = useState('');
  useEffect(() => {
    fetchLocalJson<{ policy?: string }>('/api/v1/routing/policy')
      .then((value) => setPolicy(value.policy || 'ask_first'))
      .catch(() => setStatus('Routing policy is unavailable while the daemon is unreachable.'));
  }, []);
  const save = async (next: string) => {
    const previous = policy;
    setPolicy(next);
    try {
      await fetchLocalMutation('/api/v1/routing/policy', { method: 'POST', body: { policy: next } });
      setStatus('Saved on this laptop.');
    } catch (error) {
      setPolicy(previous);
      setStatus(error instanceof Error ? error.message : 'Could not save routing policy.');
    }
  };
  return (
    <div className="table-container" id="settings-auto-switch">
      <div className="table-header-controls">
        <h3>Switch when a limit is reached</h3>
      </div>
      <p>
        When an agent hits its usage limit, Gnomcode can move the conversation to another model with
        fresh known quota. Send your next message to continue; tasks are not replayed.
      </p>
      <label>
        When an agent reaches a limit
        <select value={policy} disabled={!policy} onChange={(event) => void save(event.target.value)}>
          <option value="ask_first">Let me choose</option>
          <option value="automatic">Automatically switch model or provider</option>
          <option value="deny_cross_provider">Automatically switch within provider</option>
        </select>
      </label>
      {status ? <p role="status">{status}</p> : null}
    </div>
  );
}
