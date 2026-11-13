// First-run guide
    let onboardingStep = 0;
    let onboardingReturnFocus = null;
    function renderOnboarding() {
      const pages = [...document.querySelectorAll('[data-onboarding-step]')];
      pages.forEach((page, index) => { page.hidden = index !== onboardingStep; });
      document.getElementById('onboarding-progress').textContent = `Step ${onboardingStep + 1} of ${pages.length}`;
      document.getElementById('onboarding-back').disabled = onboardingStep === 0;
      document.getElementById('onboarding-next').textContent = onboardingStep === pages.length - 1 ? 'Open session launcher' : 'Continue';
      pages[onboardingStep].querySelector?.('h2')?.focus();
    }
    function startOnboarding() {
      const dialog = document.getElementById('onboarding-dialog');
      const tip = document.getElementById('tour-tip');
      if (tip) tip.hidden = true;
      if (dialog.open) return;
      onboardingReturnFocus = document.activeElement;
      onboardingStep = 0;
      dialog.showModal();
      renderOnboarding();
    }
    function dismissTourTip() {
      try { localStorage.setItem('gnomcode-onboarding-v1', 'dismissed'); } catch (_) {}
      const tip = document.getElementById('tour-tip');
      if (tip) tip.hidden = true;
    }
    function finishOnboarding(launch) {
      try { localStorage.setItem('gnomcode-onboarding-v1', 'dismissed'); } catch (_) {}
      const tip = document.getElementById('tour-tip');
      if (tip) tip.hidden = true;
      document.getElementById('onboarding-dialog').close();
      if (launch) {
        setView('sessions');
        showLauncher();
        document.getElementById('launch-agent-select').focus();
      } else onboardingReturnFocus?.focus();
    }
    function moveOnboarding(direction) {
      const count = document.querySelectorAll('[data-onboarding-step]').length;
      if (onboardingStep + direction >= count) { finishOnboarding(true); return; }
      onboardingStep = Math.max(0, onboardingStep + direction);
      renderOnboarding();
    }
    document.getElementById('onboarding-dialog').addEventListener('cancel', event => {
      event.preventDefault();
      finishOnboarding(false);
    });
    function offerOnboarding(sessionCount) {
      try { if (localStorage.getItem('gnomcode-onboarding-v1')) return; } catch (_) {}
      if (sessionCount === 0) {
        startOnboarding();
      } else if (typeof sessionCount === 'number' && sessionCount > 0) {
        const tip = document.getElementById('tour-tip');
        if (tip) tip.hidden = false;
      }
    }
    // End first-run guide

    async function checkOnboardingOffer() {
      try { if (localStorage.getItem('gnomcode-onboarding-v1')) return; } catch (_) {}
      try {
        const res = await apiJson('/api/v1/sessions?page=1&limit=1', { headers });
        const count = typeof res?.total_count === 'number'
          ? res.total_count
          : Array.isArray(res?.sessions)
            ? res.sessions.length
            : null;
        if (typeof count === 'number') offerOnboarding(count);
      } catch {}
    }

function showNotice(message) {
      document.getElementById('app-notice-message').textContent = String(message);
      document.getElementById('app-notice').hidden = false;
    }
    function confirmAction(message, actionLabel = 'Continue') {
      const dialog = document.getElementById('confirm-dialog');
      if (dialog.open) return Promise.resolve(false);
      document.getElementById('confirm-heading').textContent = actionLabel;
      document.getElementById('confirm-message').textContent = message;
      document.getElementById('confirm-accept').textContent = actionLabel;
      dialog.returnValue = 'cancel';
      return new Promise(resolve => {
        dialog.addEventListener('close', () => resolve(dialog.returnValue === 'confirm'), {once:true});
        dialog.showModal();
      });
    }

function cycleTheme() {
      const current = document.documentElement.dataset.theme || 'system';
      const next = current === 'system' ? 'light' : current === 'light' ? 'dark' : 'system';
      if (next === 'system') delete document.documentElement.dataset.theme;
      else document.documentElement.dataset.theme = next;
      try {
        if (next === 'system') localStorage.removeItem('gnomcode-theme');
        else localStorage.setItem('gnomcode-theme', next);
      } catch (_) {}
      const button = document.getElementById('theme-control');
      button.textContent = `Theme / ${next}`;
      button.setAttribute('aria-label', `Theme: ${next}. Switch mode`);
    }

    const themeButton = document.getElementById('theme-control');
    if (themeButton) {
      const savedTheme = document.documentElement.dataset.theme || 'system';
      themeButton.textContent = `Theme / ${savedTheme}`;
      themeButton.setAttribute('aria-label', `Theme: ${savedTheme}. Switch mode`);
    }

    function escapeHtml(value) { return String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char])); }
    function jsArg(value) { return escapeHtml(JSON.stringify(String(value ?? ''))); }
    const consoleNavigation = document.querySelector('.sp-console-nav');
    const updateNavOrientation = () => consoleNavigation.querySelector('[role=tablist]').setAttribute('aria-orientation', 'vertical');
    updateNavOrientation(); window.addEventListener('resize', updateNavOrientation);
    let currentTimeline = 'all';
    let currentPage = 1;
    const pageLimit = 10;
    const token = new URLSearchParams(window.location.search).get('token') || '';
    const headers = token ? { 'Authorization': `Bearer ${token}` } : {};
    let refreshBusy = false;
    let lastSuccessfulUpdate = null;
    let activeView = 'overview';
    let projectsLoaded = false;
    let lastReportJson = '';
    let lastSessionsJson = '';
    let lastQuotaJson = '';

    function consoleViewUrl(view) { const url = new URL(window.location.href); url.hash = view === 'overview' ? '' : view; return url.pathname + url.search + url.hash; }
    document.querySelector('.app-brand').href = consoleViewUrl('overview');

    // Old links keep working: Limits and Projects moved into Usage, Automations into Settings.
    const VIEW_ALIASES = { limits: 'usage', projects: 'usage', automations: 'settings' };
    const VIEWS = ['overview', 'sessions', 'usage', 'settings', 'auth'];
    const CONSOLE_VIEWS = ['overview', 'sessions', 'usage'];
    const resolveView = name => VIEW_ALIASES[name] || name;

    function setView(view, updateUrl = true) {
      view = resolveView(view);
      activeView = view;
      try { sessionStorage.setItem('gnomcode-active-view', view); } catch (_) {}
      document.body.dataset.consoleView = view;
      document.getElementById('chat-list-toggle').hidden = view !== 'sessions';
      const titles = { overview: 'Home', sessions: 'Workspace', usage: 'Usage', settings: 'Settings', auth: 'Account' };
      document.getElementById('console-title').textContent = titles[view] || 'Home';
      if (CONSOLE_VIEWS.includes(view)) {
        document.getElementById('view-console').setAttribute('aria-labelledby', `tab-btn-${view}`);
      }
      document.querySelectorAll('.tab-btn').forEach(button => {
        const selected = button.dataset.view === view;
        button.classList.toggle('active', selected);
        button.setAttribute('aria-selected', String(selected));
        button.tabIndex = selected || (view === 'auth' && button.dataset.view === 'settings') ? 0 : -1;
      });
      document.querySelectorAll('[data-local-view]').forEach(section => { section.hidden = !section.dataset.localView.split(',').includes(view); });
      document.getElementById('session-chat-panel').hidden = view !== 'sessions' || !activeChatSession;
      document.querySelectorAll('.view-content').forEach(section => section.classList.toggle('active', section.id === `view-${view}` || (section.id === 'view-console' && CONSOLE_VIEWS.includes(view))));
      if (updateUrl && window.location.hash.slice(1) !== view) history.pushState(null, '', consoleViewUrl(view));
      if (view === 'usage') { scrollHeatmapToToday(); void loadHeatmap(); void loadProjects(); }
      if (view === 'settings') { void loadProData(); }
      if (view === 'overview') { void loadHeatmap(); void loadToday(true); }
    }

    document.querySelector('.nav-tabs').addEventListener('keydown', event => {
      const tabs = [...document.querySelectorAll('.nav-tabs [role="tab"]')];
      const current = tabs.indexOf(document.activeElement);
      if (current < 0) return;
      let next = current;
      if (event.key === 'ArrowRight' || event.key === 'ArrowDown') next = (current + 1) % tabs.length;
      else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') next = (current - 1 + tabs.length) % tabs.length;
      else if (event.key === 'Home') next = 0;
      else if (event.key === 'End') next = tabs.length - 1;
      else return;
      event.preventDefault();
      const tab = tabs[next];
      tab.focus();
      navigateView(tab.dataset.view);
    });

    function navigateView(view) { setView(view); refreshCurrentView(); }
    // Rewrites an old #limits/#projects/#automations hash in place, so Back does not bounce through the old name.
    function normalizeHash() {
      const target = VIEW_ALIASES[window.location.hash.slice(1)];
      if (target) history.replaceState(null, '', consoleViewUrl(target));
    }
    window.addEventListener('hashchange', () => {
      normalizeHash();
      const view = resolveView(window.location.hash.slice(1) || 'overview');
      if (VIEWS.includes(view)) { setView(view, false); refreshCurrentView(); }
    });

    async function refreshCurrentView() {
      if (refreshBusy) return;
      refreshBusy = true;
      const button = document.getElementById('reload-view');
      button.disabled = true;
      document.getElementById('resync-data').disabled = true;
      button.textContent = 'Refreshing...';
      try {
        while (pollBusy) await new Promise(resolve => setTimeout(resolve, 25));
        if (CONSOLE_VIEWS.includes(activeView)) {
          const results = await Promise.all([loadReport(), loadSessions(), loadQuota(), loadOngoing()]);
          if (results.some(result => !result)) throw new Error('One or more local data sources failed');
          if (['usage', 'overview'].includes(activeView) && !await loadHeatmap(true)) throw new Error('Activity could not be loaded');
          if (activeView === 'usage' && !await loadProjects()) throw new Error('Projects could not be loaded');
        } else if (activeView === 'settings') {
          const results = await Promise.all([loadSettingsData(), loadProData(), loadQuota()]);
          if (results.some(result => !result)) throw new Error('Settings could not be loaded');
        } else if (activeView === 'auth') {
          if (!await checkAuthSession()) throw new Error('Account status could not be loaded');
        }
        lastSuccessfulUpdate = new Date();
        document.getElementById('last-updated').textContent = `Last updated: ${lastSuccessfulUpdate.toLocaleTimeString()}`;
        document.getElementById('sync-state').textContent = 'View refreshed';
        document.getElementById('sync-state').classList.remove('local-status-error');
      } catch {
        document.getElementById('sync-state').textContent = 'Refresh failed';
        document.getElementById('sync-state').classList.add('local-status-error');
      } finally {
        button.disabled = false;
        document.getElementById('resync-data').disabled = false;
        button.textContent = 'Reload view';
        refreshBusy = false;
      }
    }

    let pollBusy = false;
    async function pollLocalData() {
      if (refreshBusy || pollBusy || document.hidden) return;
      if (!CONSOLE_VIEWS.includes(activeView) && activeView !== 'settings') return;
      pollBusy = true;
      try {
      // Settings only needs the provider overview; the other sources are not on screen.
      const results = activeView === 'settings' ? [await loadQuota()] : await Promise.all([loadReport(), loadSessions(), loadQuota(), loadOngoing()]);
      if (['usage', 'overview'].includes(activeView)) void loadHeatmap();
      const state = document.getElementById('sync-state');
      const updated = document.getElementById('last-updated');
      if (results.every(Boolean)) {
        lastSuccessfulUpdate = new Date();
        updated.textContent = `Last successful update: ${lastSuccessfulUpdate.toLocaleTimeString()}`;
        state.textContent = 'Local daemon connected';
        state.classList.remove('local-status-error');
      } else {
        state.textContent = lastSuccessfulUpdate ? 'Local data stale' : 'Local data unavailable';
        state.classList.add('local-status-error');
      }
      } finally { pollBusy = false; }
    }

    async function resyncLocal() {
      if (refreshBusy) return;
      refreshBusy = true;
      const button = document.getElementById('resync-data');
      button.disabled = true;
      document.getElementById('reload-view').disabled = true;
      button.textContent = 'Reconciling...';
      document.getElementById('sync-state').textContent = 'Reconciling local data';
      try {
        while (pollBusy) await new Promise(resolve => setTimeout(resolve, 25));
        const response = await fetch('/api/v1/reconcile', { method: 'POST', headers });
        if (response.status === 409) {
          document.getElementById('sync-state').textContent = 'Reconciliation in progress';
        } else if (!response.ok) throw new Error('Reconciliation failed');
        const refreshed = await Promise.all([loadReport(), loadSessions(), loadQuota(), loadOngoing()]);
        if (refreshed.some(result => !result)) throw new Error('Local data refresh failed');
        if (response.status === 409) {
          document.getElementById('sync-state').textContent = 'Reconciliation in progress';
        } else {
          lastSuccessfulUpdate = new Date();
          document.getElementById('last-updated').textContent = `Last successful update: ${lastSuccessfulUpdate.toLocaleTimeString()}`;
          document.getElementById('sync-state').textContent = 'Local data reconciled';
        }
        document.getElementById('sync-state').classList.remove('local-status-error');
      } catch {
        document.getElementById('sync-state').textContent = 'Reconciliation failed';
        document.getElementById('sync-state').classList.add('local-status-error');
      } finally {
        button.disabled = false;
        document.getElementById('reload-view').disabled = false;
        button.textContent = 'Resync local data';
        refreshBusy = false;
      }
    }

    async function apiJson(url, options = { headers }) {
      const response = await fetch(url, options);
      if (!response.ok) {const error=await response.json().catch(()=>null);throw new Error(response.status === 401 ? 'Local authorization is missing or expired. Close this window and reopen gnomcode from its tray icon or shortcut.' : typeof error?.message === 'string' ? error.message : `gnomcode request failed (${response.status}). Retry Reload view or restart gnomcode if this persists.`);}
      return response.json();
    }
    function escapeHtml(value) {
      return String(value || '').replace(/[&<>"']/g, character => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[character]));
    }
    function showConnectionError(error) {
      const status = document.getElementById('connection-status');
      status.textContent = error.message;
      status.hidden = false;
    }

    function switchTab(tabName) {
      setView(tabName);
      refreshCurrentView();
      if (tabName === 'sessions') {
        loadInstalledAgents();
      }
    }

    async function exportData(fmt) {
      try {
        let url = `/api/v1/export?range=${currentTimeline}&format=${fmt}`;
        if (currentTimeline === 'today' || currentTimeline === 'month') {
          const d = new Date();
          d.setHours(0, 0, 0, 0); if (currentTimeline === 'month') d.setDate(1);
          url += `&from=${encodeURIComponent(d.toISOString())}`;
        }
        const res = await fetch(url, { headers, signal: AbortSignal.timeout(15000) });
        if (res.status === 402) {
          showNotice("Export is unavailable. Reopen the local console and try again.");
          return;
        }
        if (!res.ok) {
          showNotice("Export failed. Reload the view and try again.");
          return;
        }
        const blob = await res.blob();
        const downloadUrl = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = downloadUrl;
        a.download = `gnomcode-report-${currentTimeline}.${fmt}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        window.URL.revokeObjectURL(downloadUrl);
      } catch (err) {
        showNotice("Export failed. Reload the view and retry. Details: " + err);
      }
    }

    async function saveGuardrail() {
      const button=document.getElementById('rule-save'),status=document.getElementById('rule-status');
      button.disabled=true;
      status.textContent='Saving rule...';
      try {
        await apiJson('/api/v1/rules',{method:'POST',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify({
          id:`rule-${Date.now()}`,project_id:null,agent:document.getElementById('rule-agent').value || null,
          max_spend_usd:document.getElementById('rule-spend').value ? Number(document.getElementById('rule-spend').value) : null,
          max_turns:document.getElementById('rule-turns').value ? Number(document.getElementById('rule-turns').value) : null,
          action:document.getElementById('rule-action').value
        })});
        status.textContent='Rule saved. It applies to each matching session. Alerts appear below.';
        await loadProData();
      } catch(error) {status.textContent=`Rule could not be saved. ${error.message}`;}
      finally {button.disabled=false;}
    }
    document.getElementById('guardrail-form').addEventListener('submit',event=>{event.preventDefault();void saveGuardrail();});
    async function loadProData() {
      try {
        const licRes = await fetch('/api/v1/license', { headers, signal: AbortSignal.timeout(15000) });
        if (!licRes.ok) throw new Error('License status request failed');
        if (licRes.ok) {
          const lic = await licRes.json();
          const badge = document.getElementById('pro-tier-badge');
          const details = document.getElementById('pro-license-details');
          document.getElementById('license-entry').hidden = lic.licensed;
          document.getElementById('replace-license').hidden = !lic.licensed;
          document.getElementById('remove-license').hidden = !lic.licensed;
          if (lic.licensed) {
            badge.innerText = lic.tier.toUpperCase();
            badge.style.color = 'var(--green)';
            details.innerText = `License active for ${lic.customer_name || 'user'}. ${lic.expires_at ? 'Expires ' + lic.expires_at : 'Perpetual license.'}`;
          } else {
            badge.innerText = 'FREE';
            badge.style.color = 'var(--text-muted)';
            details.innerText = 'All local features are free. Existing offline licenses can still be managed here.';
          }
        }
        const qRes = await fetch('/api/v1/resume-queue', { headers, signal: AbortSignal.timeout(15000) });
        if (!qRes.ok) throw new Error('Resume queue request failed');
        if (qRes.ok) {
          const queue = await qRes.json();
          const qTable = document.getElementById('resume-queue-table');
          if (!queue || queue.length === 0) {
            qTable.innerHTML = '<tr><td colspan="8" class="sp-muted-cell">No reconnections scheduled.</td></tr>';
          } else {
            qTable.innerHTML = queue.map(e => `
              <tr>
                <td class="sp-mono-meta sp-mono-meta--text">${escapeHtml(e.session_id)}</td>
                <td><span class="badge">${escapeHtml(e.agent)}</span></td>
                <td class="sp-mono-meta sp-mono-meta--sans">${escapeHtml(e.cwd)}</td>
                <td>${escapeHtml(e.resume_prompt)}</td>
                <td>${e.max_runs}</td>
                <td>${e.runs_completed}</td>
                <td class="sp-mono-meta sp-mono-meta--sans">${formatRelativeTime(e.eligible_after)}</td>
                <td><button class="btn-control" data-action="disarm" data-arg="${escapeHtml(e.session_id)}">Disarm</button></td>
              </tr>
            `).join('');
          }
        }
        const rulesRes = await fetch('/api/v1/rules', { headers, signal: AbortSignal.timeout(15000) });
        if (!rulesRes.ok) throw new Error('Guardrail rules request failed');
        if (rulesRes.ok) {
          const rules = await rulesRes.json();
          const rTable = document.getElementById('rules-table');
          if (!rules || rules.length === 0) {
            rTable.innerHTML = '<tr><td colspan="6" class="sp-muted-cell">No usage rules yet.</td></tr>';
          } else {
            rTable.innerHTML = rules.map(r => `
              <tr>
                <td class="sp-mono-meta sp-mono-meta--text sp-strong-cell">${escapeHtml(r.id)}</td>
                <td>${escapeHtml(r.agent || 'All Agents')}</td>
                <td>${r.max_spend_usd ? '$' + r.max_spend_usd.toFixed(2) : 'None'}</td>
                <td>${r.max_turns ? r.max_turns : 'None'}</td>
                <td><span class="badge ${r.action === 'HardStop' ? 'sp-tone-red' : 'sp-tone-accent'}">${r.action === 'HardStop' ? 'Claude hook stop; other agents alert' : escapeHtml(r.action)}</span></td>
                <td><button class="btn-control" data-action="delete-rule" data-arg="${escapeHtml(r.id)}">Delete</button></td>
              </tr>
            `).join('');
          }
        }
        const alertsRes = await fetch('/api/v1/alerts', { headers, signal: AbortSignal.timeout(15000) });
        if (!alertsRes.ok) throw new Error('Guardrail alerts request failed');
        if (alertsRes.ok) {
          const alerts = await alertsRes.json();
          const aTable = document.getElementById('alerts-table');
          if (!alerts || alerts.length === 0) {
            aTable.innerHTML = '<tr><td colspan="4" class="sp-muted-cell">No alerts yet.</td></tr>';
          } else {
            aTable.innerHTML = alerts.map(a => `
              <tr>
                <td class="sp-mono-meta sp-mono-meta--text">${escapeHtml(a.session_id)}</td>
                <td class="sp-strong-cell">${escapeHtml(a.rule_id)}</td>
                <td><span class="badge ${a.action === 'hard_stop' ? 'sp-tone-red' : 'sp-tone-accent'}">${a.action === 'hard_stop' ? 'Stop requested (Claude hook only)' : escapeHtml(a.action)}</span></td>
                <td class="sp-mono-meta sp-mono-meta--sans">${formatRelativeTime(a.created_at)}</td>
              </tr>
            `).join('');
          }
        }
        if (!await loadRelayData()) throw new Error('Relay data request failed');
        return true;
      } catch (e) {
        console.error(e);
        document.getElementById('pro-license-details').textContent = 'Automation and guardrail data is unavailable. Retry Reload view when the daemon is reachable.';
        return false;
      }
    }

    async function loadRelayData() {
      try {
        const res = await fetch('/api/v1/pro/relay', { headers, signal: AbortSignal.timeout(15000) });
        if (!res.ok) return false;
        const data = await res.json();
        if (data.config) {
          const pol = document.getElementById('relay-policy-select');
          const str = document.getElementById('relay-strategy-select');
          const thr = document.getElementById('relay-threshold-select');
          for (const [select, value] of [[pol, data.config.policy], [str, data.config.strategy], [thr, data.config.threshold_pct]]) {
            if (select && value != null && [...select.options].some(option => option.value === String(value))) select.value = String(value);
          }
        }
        const table = document.getElementById('relay-table');
        if (table) {
          if (!data.relays || data.relays.length === 0) {
            table.innerHTML = '<tr><td colspan="7" class="sp-muted-cell">No context handoffs recorded. Open a supported session to switch agents.</td></tr>';
          } else {
            table.innerHTML = data.relays.map(r => {
              const statusTone = r.status === 'executed' ? 'sp-tone-green' : (r.status === 'pending' ? 'sp-tone-yellow' : 'sp-tone-muted');
              const actionBtn = r.status === 'pending'
                ? `<button class="btn-primary sp-row-action sp-row-action--gap" data-action="execute-relay" data-arg="${escapeHtml(r.id)}">Handoff & Rotate</button><button class="btn-control sp-row-action" data-action="dismiss-relay" data-arg="${escapeHtml(r.id)}">Dismiss</button>`
                : `<span class="sp-mono-meta">${escapeHtml(r.status)}</span>`;
              return `<tr>
                <td class="sp-mono-meta sp-mono-meta--text">${escapeHtml(r.session_id)}</td>
                <td class="sp-strong-cell">${escapeHtml(r.project_name)}</td>
                <td>${escapeHtml(r.source_agent)} &rarr; ${escapeHtml(r.target_agent)}</td>
                <td class="sp-strong-cell ${r.fill_pct >= 90 ? 'sp-tone-red' : 'sp-tone-yellow'}">${r.fill_pct}% <span class="trust-mark">EST</span></td>
                <td><span class="badge ${statusTone}">${r.status.toUpperCase()}</span></td>
                <td class="sp-mono-meta sp-mono-meta--sans">${formatRelativeTime(r.created_at)}</td>
                <td>${actionBtn}</td>
              </tr>`;
            }).join('');
          }
        }
        return true;
      } catch (err) {
        console.error('Failed to load relay data', err);
        return false;
      }
    }

    async function saveRelayConfig() {
      const pol = document.getElementById('relay-policy-select').value;
      const str = document.getElementById('relay-strategy-select').value;
      const thr = parseInt(document.getElementById('relay-threshold-select').value, 10);
      try {
        const res = await fetch('/api/v1/pro/relay/config', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ policy: pol, strategy: str, threshold_pct: thr })
        });
        if (res.status === 402) {
          showNotice('Could not save handoff settings. Reopen the local console and try again.');
          return;
        }
        if (res.ok) {
          loadRelayData();
        }
      } catch (err) {
        showNotice('Settings could not be saved. Check the connection and retry. Details: ' + err.message);
      }
    }

    async function executeRelay(id) {
      if (!await confirmAction('Rotate session now? gnomcode will create .gnomcode/HANDOFF.md and spawn a clean session.', 'Rotate session')) return;
      try {
        const res = await fetch('/api/v1/pro/relay/execute', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ id })
        });
        if (res.status === 402) {
          showNotice('Could not start the handoff. Reopen the local console and check the agent session.');
          return;
        }
        if (res.ok) {
          showNotice('Handoff launched a fresh session. Check the target conversation and reported model.');
          loadRelayData();
        } else {
          showNotice('Handoff failed. Check the target CLI in Agent terminal and try again.');
        }
      } catch (err) {
        showNotice('Error: ' + err.message);
      }
    }

    async function dismissRelay(id) {
      try {
        const res = await fetch('/api/v1/pro/relay/dismiss', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ id })
        });
        if (res.ok) {
          loadRelayData();
        }
      } catch (err) {
        showNotice('Error: ' + err.message);
      }
    }

    async function activateLicenseKey() {
      const input = document.getElementById('license-key-input');
      const feedback = document.getElementById('license-feedback');
      const key = input.value.trim();
      if (!key) return;
      try {
        const res = await fetch('/api/v1/license', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ license_key: key })
        });
        if (res.ok) {
          feedback.style.color = 'var(--green)';
          feedback.innerText = 'License activated successfully!';
          input.value = '';
          loadProData();
        } else {
          feedback.style.color = 'var(--red)';
          feedback.innerText = 'Activation failed. Invalid or expired signature.';
        }
      } catch (err) {
        feedback.style.color = 'var(--red)';
        feedback.innerText = 'Error: ' + err;
      }
    }

    async function removeLicenseKey() {
      if (!await confirmAction('Remove the saved offline license? Local features remain free.', 'Remove license')) return;
      try {
        await fetch('/api/v1/license', { method: 'DELETE', headers });
        loadProData();
      } catch (err) {
        showNotice(err);
      }
    }

    async function disarmResume(sid) {
      try {
        await fetch('/api/v1/resume-queue/disarm', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ session_id: sid })
        });
        loadProData();
      } catch (err) {
        showNotice(err);
      }
    }

    async function deleteRule(ruleId) {
      try {
        await fetch(`/api/v1/rules/${encodeURIComponent(ruleId)}`, { method: 'DELETE', headers });
        loadProData();
      } catch (err) {
        showNotice(err);
      }
    }

    function formatRelativeTime(isoStr) {
      if (!isoStr) return 'N/A';
      const d = new Date(isoStr);
      const diffMs = new Date() - d;
      const diffMins = Math.floor(diffMs / 60000);
      const diffHours = Math.floor(diffMins / 60);
      const diffDays = Math.floor(diffHours / 24);

      if (diffMins < 1) return 'Just now';
      if (diffMins < 60) return `${diffMins}m ago`;
      if (diffHours < 24) return `${diffHours}h ago`;
      if (diffDays < 30) return `${diffDays}d ago`;
      return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
    }

    function formatTokens(n) {
      if (!n) return '0';
      if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
      if (n >= 1e3) return (n / 1e3).toFixed(1) + 'k';
      return n.toString();
    }

    function setTimeline(range) {
      currentTimeline = range;
      document.getElementById('timeline-select').value = range;
      lastReportJson = '';
      loadReport();
      if (activeView === 'usage') loadProjects();
    }

    function onFilterChange() {
      currentPage = 1;
      lastSessionsJson = '';
      loadSessions();
    }

    function prevPage() {
      if (currentPage > 1) {
        currentPage--;
        lastSessionsJson = '';
        loadSessions();
      }
    }

    function nextPage() {
      currentPage++;
      lastSessionsJson = '';
      loadSessions();
    }

    async function loadReport() {
      try {
        const requestedRange = currentTimeline;
        let url = `/api/v1/report?range=${currentTimeline}`;
        if (currentTimeline === 'today' || currentTimeline === 'month') {
          const d = new Date();
          d.setHours(0, 0, 0, 0); if (currentTimeline === 'month') d.setDate(1);
          url += `&from=${encodeURIComponent(d.toISOString())}`;
        }
        const res = await apiJson(url, { headers, signal: AbortSignal.timeout(15000) });
        if (!res || !res.timeline) throw new Error('The local report response is incomplete. Retry Reload view.');
        if (requestedRange !== currentTimeline) return true;

        const jsonStr = JSON.stringify(res);
        if (jsonStr === lastReportJson) return true;

        document.getElementById('connection-status').hidden = true;
        const tl = res.timeline;
        const presentation = await import("/console-presentation.js");
        presentation.updateActivity(document.getElementById("daily-activity"), tl, { totals: false });


        // Date range header
        let dateStr = tl.range_label;
        if (tl.earliest_at && tl.latest_at) {
          const e = new Date(tl.earliest_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
          const l = new Date(tl.latest_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
          dateStr = e === l ? `Showing: ${tl.range_label} (${e})` : `Showing: ${tl.range_label} (${e} — ${l})`;
        }
        document.getElementById('timeline-label').textContent = dateStr;

        // Subscription value tiles: Rust supplies every figure, the presentation module only formats.
        presentation.updateValue(document.getElementById('usage-value'), res.roi_by_provider, res.combined_roi);

        // Render the shared provider breakdown.
        presentation.updateProviders(document.getElementById('providers-container'), tl);
        lastReportJson = jsonStr;
        return true;
      } catch (e) {
        console.error('Failed to load report', e);
        lastReportJson = '';
        showConnectionError(e);
        document.getElementById('daily-activity').textContent = 'Daily activity unavailable. ' + e.message;
        document.getElementById('providers-container').innerHTML = '<p class="local-view-message">Usage data is unavailable. Retry Reload view when the local daemon is reachable.</p>';
        return false;
      }
    }

    // The trailing-year grid changes slowly, so it is fetched at most once a minute, never on the 2-second poll.
    let lastHeatmapAt = 0;
    async function loadHeatmap(force = false) {
      if (!force && Date.now() - lastHeatmapAt < 60000) return true;
      lastHeatmapAt = Date.now();
      const container = document.getElementById('usage-heatmap');
      const mini = document.getElementById('home-activity');
      try {
        const data = await apiJson('/api/v1/activity/heatmap', { headers, signal: AbortSignal.timeout(15000) });
        const presentation = await import('/console-presentation.js');
        presentation.updateHeatmap(container, data);
        presentation.updateHeatmap(mini, data, { weeks: 12, stats: false, legend: false, months: false });
        return true;
      } catch (_) {
        lastHeatmapAt = Date.now() - 50000;
        if (!container.firstChild) container.innerHTML = '<div class="sp-empty"><p>Activity could not be loaded.</p></div>';
        return false;
      }
    }

    // Today's totals feed the Home tiles; the range report is fetched at most every 10 seconds.
    let lastTodayAt = 0;
    let todayReport = null;
    async function loadToday(force = false) {
      if (!force && Date.now() - lastTodayAt < 10000) { await renderToday(); return true; }
      lastTodayAt = Date.now();
      try {
        const start = new Date(); start.setHours(0, 0, 0, 0);
        const res = await apiJson(`/api/v1/report?range=today&from=${encodeURIComponent(start.toISOString())}`, { headers, signal: AbortSignal.timeout(15000) });
        todayReport = res?.timeline || null;
      } catch (_) { todayReport = null; lastTodayAt = Date.now() - 5000; }
      await renderToday();
      return Boolean(todayReport);
    }
    async function renderToday() {
      const presentation = await import('/console-presentation.js');
      presentation.updateTodayTiles(document.getElementById('home-today'), { report: todayReport, activeCount: homeActiveCount });
    }

    function scrollHeatmapToToday() {
      const scroller = document.querySelector('#usage-heatmap .sp-heatmap-scroll');
      if (scroller) scroller.scrollLeft = scroller.scrollWidth;
    }

    document.addEventListener('click', event => {
      const cell = event.target.closest?.('.sp-heatmap-grid .sp-heat[data-date]');
      if (!cell) return;
      const day = [...document.querySelectorAll('.sp-activity-breakdown [data-date]')].find(item => item.dataset.date === cell.dataset.date);
      if (!day) return;
      day.closest('.sp-activity-breakdown').open = true;
      day.open = true;
      day.scrollIntoView({ block: 'center', behavior: window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' });
    });

    async function loadProjects() {
      const container = document.getElementById('project-rollups');
      const range = currentTimeline;
      try {
        let url = `/api/v1/projects?range=${encodeURIComponent(range)}`;
        if (range === 'today') { const start = new Date(); start.setHours(0, 0, 0, 0); url += `&from=${encodeURIComponent(start.toISOString())}`; }
        const projects = await apiJson(url, { headers, signal: AbortSignal.timeout(15000) });
        if (range !== currentTimeline) return true;
        const presentation = await import('/console-presentation.js');
        presentation.mountProjectList(container, projects, { heading: false, helper: 'Most recent first. Select a project to filter sessions.' });
        projectsLoaded = true;
        const select = document.getElementById('project-filter'); const selected = select.value; select.innerHTML = '<option value="">All projects</option>'; for (const project of projects) { const option = document.createElement('option'); option.value = project.id; option.textContent = project.name; select.append(option); } select.value = selected;
        return true;
      } catch (_) {
        container.textContent = 'Project usage could not be refreshed. Retry Reload view.';
        return false;
      }
    }

    let activeChatSession = '';
    let chatEvents = [];
    let chatWs = null;
    let installedAgents = [];
    let ideSessions = [];
    let conversationRows = [];
    let conversationRequest = 0;
    let hostedSessions = [];
    let chatRequest = 0;
    const chatDrafts = new Map();
    const pendingPrompts = new Set();
    const optimisticPrompts = new Map();
    const chatDeletionGenerations = new Map();
    const handoffNotices = new Map();
    async function renderCurrentChat() {
      const sessionId = activeChatSession;
      const presentation = await import('/console-presentation.js');
      if (sessionId !== activeChatSession) return;
      const state = ideSessions.find(session => session.id === activeChatSession)?.state;
      const result = presentation.reconcileChat([...(handoffNotices.get(activeChatSession) || []),...chatEvents], optimisticPrompts.get(activeChatSession) || [], state);
      optimisticPrompts.set(activeChatSession, result.pending);
      presentation.updateChat(document.getElementById('chat-stream'), result.events);
    }

    function selectChatSession(sessionId) {
      const input = document.getElementById('chat-prompt-text');
      if (activeChatSession) chatDrafts.set(activeChatSession, input.value);
      activeChatSession = sessionId;
      document.getElementById('workspace-terminal-pane').hidden = true;
      input.value = chatDrafts.get(sessionId) || '';
      requestAnimationFrame(resizePrompt);
      document.getElementById('chat-send-btn').disabled = pendingPrompts.has(sessionId);
      document.getElementById('chat-delivery-status').textContent = '';
    }

    async function renderIdeSessions() {
      const presentation = await import('/console-presentation.js');
      document.getElementById('ide-session-list').innerHTML = presentation.renderSessionList(conversationRows.map(row => ({ ...row, home_group: row.home_group || homeGroups.get(row.id) })), activeChatSession);
    }

    // Workspace lands on the last conversation opened in the past 24 hours, once per visit; otherwise on the empty state.
    let reopenTried = false;
    function reopenLastChat() {
      if (reopenTried || activeView !== 'sessions') return;
      reopenTried = true;
      if (activeChatSession) return;
      try {
        const last = JSON.parse(storedFlag('gnomcode-last-chat'));
        if (last && Date.now() - last.at < 864e5 && ideSessions.some(session => session.id === last.id)) void loadChat(last.id);
      } catch (_) { /* no remembered conversation */ }
    }

    // The launcher opens only from "New"; closing a chat leaves the empty state (CSS shows it when nothing else is open).
    function showLauncher(open = true) {
      reopenTried = true;
      selectChatSession(''); chatEvents = []; chatRequest++;
      if (chatWs) { chatWs.close(); chatWs = null; }
      document.getElementById('session-chat-panel').hidden = true;
      document.getElementById('ide-launcher').hidden = !open;
      document.getElementById('launch-status').textContent = '';
      renderIdeSessions(); if (open) loadInstalledAgents();
    }

    function openExplorerWorkspace() {
      const url = new URL(window.location.href);
      if (!url.hash.startsWith('#sessions?')) return;
      const folder = new URLSearchParams(url.hash.slice('#sessions?'.length)).get('folder');
      if (!folder) return;
      url.hash = 'sessions';
      history.replaceState(null, '', url.toString());
      setView('sessions', false);
      showLauncher();
      document.getElementById('launch-repo-input').value = folder;
    }

    function showCurrentChatModel() {
      if(modelHandoffBusy)return;
      const select = document.getElementById('chat-model-select');
      const session = ideSessions.find(session => session.id === activeChatSession);
      const hosted = hostedSessions.find(session => session.session_id === activeChatSession);
      const model = hosted?.model || session?.model || '';
      const agent = hosted?.agent || session?.agent || '';
      const cliAgent = {'claude-code':'claude','gemini-cli':'gemini'}[agent] || agent;
      const observed = hosted ? hosted.model_observed : Boolean(session?.model);
      const status = document.getElementById('chat-model-status');
      const message = select.disabled ? 'Model selection unavailable' : observed ? '' : model ? 'Switching model...' : 'Agent default';
      if (status.textContent !== message) status.textContent = message;
      if (select.dataset.sessionId !== activeChatSession || select.dataset.currentModel !== model) {
        select.querySelectorAll('[data-current-only]').forEach(option => option.remove());
        const choice = [...select.options].find(option => option.dataset.model === model && option.dataset.agent === cliAgent);
        if (model && !choice) {
          const option = document.createElement('option'); option.value = model; option.textContent = `${model} (current)`; option.disabled = true; option.dataset.currentOnly = ''; select.append(option);
        }
        select.value = choice?.value || model;
        select.dataset.currentSelection = select.value;
        select.dataset.sessionId = activeChatSession;
        select.dataset.currentModel = model;
      }
      document.getElementById('chat-switch-btn').disabled = select.disabled || !select.value || select.selectedOptions[0]?.disabled || select.value === select.dataset.currentSelection;
    }

    async function loadChatModels() {
      const select = document.getElementById('chat-model-select');
      const button = document.getElementById('chat-switch-btn');
      const sessionId = activeChatSession;
      try {
        const catalog = await apiJson('/api/v1/model-catalog', { headers });
        if (sessionId !== activeChatSession) return;
        select.innerHTML = '<option value="">Agent default</option>';
        for (const provider of catalog.providers || []) {
          const group = document.createElement('optgroup'); group.label = provider.name;
          for (const model of provider.models || []) {
            const option = document.createElement('option'); option.value = `${model.agent}:${model.model}`;
            option.dataset.agent = model.agent; option.dataset.model = model.model; option.textContent = model.display_name || model.model; group.append(option);
          }
          if (!group.children.length) { const option=document.createElement('option');option.disabled=true;option.textContent=provider.message;group.append(option); }
          select.append(group);
        }
        select.disabled = ![...select.options].some(option=>!option.disabled && option.dataset.model); button.disabled = select.disabled;
        delete select.dataset.sessionId;
        showCurrentChatModel();
      } catch {
        if (sessionId !== activeChatSession) return;
        select.innerHTML = '<option value="">Model catalog unavailable</option>'; select.disabled = true; button.disabled = true;
        delete select.dataset.sessionId;
        showCurrentChatModel();
      }
    }

    let modelHandoffBusy = false;
    async function showChatLimits() {
      const sessionId = activeChatSession;
      const presentation = await import('/console-presentation.js');
      if (sessionId !== activeChatSession) return;
      const element = document.getElementById('chat-header-limits');
      const markup = presentation.renderConversationLimits(ideSessions.find(session => session.id === sessionId)?.limits || []);
      if (element.innerHTML !== markup) element.innerHTML = markup;
    }
    async function switchChatModel() {
      const select = document.getElementById('chat-model-select');
      if (!activeChatSession || !select.value || modelHandoffBusy || select.value === select.dataset.currentSelection) return;
      const sourceSession = activeChatSession;
      const previousModel = select.dataset.currentModel || 'CLI default';
      const targetModel = select.selectedOptions[0].dataset.model || select.value;
      const targetAgent = select.selectedOptions[0].dataset.agent;
      const button = document.getElementById('chat-switch-btn'); button.disabled = true;
      const draft=pendingPrompts.has(sourceSession) ? '' : document.getElementById('chat-prompt-text').value;
      modelHandoffBusy = true;select.disabled=true;pendingPrompts.add(sourceSession);document.getElementById('chat-send-btn').disabled=true;
      try {
        const result = await apiJson(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/switch`, { method:'POST', headers:{ ...headers, 'Content-Type':'application/json' }, body:JSON.stringify({ target_model:targetModel, agent:targetAgent }) });
        await loadSessions();
        if (activeChatSession === sourceSession) {
          await loadChat(result.session_id);
          if(draft){chatDrafts.set(result.session_id,draft);chatDrafts.delete(sourceSession);document.getElementById('chat-prompt-text').value=draft;resizePrompt();}
          document.getElementById('chat-delivery-status').textContent = '';
          void renderCurrentChat();
        }
      } catch (error) { select.value=select.dataset.currentSelection || ''; if (activeChatSession === sourceSession) showNotice(error.message || 'Model change could not be confirmed. Your draft is saved. Check the selected model and try again.'); }
      finally { modelHandoffBusy=false;pendingPrompts.delete(sourceSession);select.disabled=false;document.getElementById('chat-send-btn').disabled=pendingPrompts.has(activeChatSession);showCurrentChatModel(); }
    }

    async function focusChat() {
      if (!activeChatSession) return;
      try {
        const result = await apiJson(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/focus`, { method:'POST', headers });
        document.getElementById('session-action-status').textContent = result.message;
        const copy = document.getElementById('session-action-copy'); copy.hidden = result.focused || !result.repo_path; copy.dataset.path = result.repo_path || '';
      } catch { document.getElementById('session-action-status').textContent = 'Window unavailable. Use the terminal stream for a hosted session.'; }
    }

    async function resumeChat() {
      const source = activeChatSession;
      const button = document.getElementById('chat-resume');
      button.disabled = true;
      document.getElementById('chat-delivery-status').textContent = 'Resuming conversation...';
      try {
        await apiJson(`/api/v1/sessions/${encodeURIComponent(source)}/continue`, {method:'POST',headers});
        await loadSessions();
        if (source === activeChatSession) {
          await loadChat(source);
          document.getElementById('chat-delivery-status').textContent = 'Ready to continue.';
        }
      } catch { document.getElementById('chat-delivery-status').textContent = 'Could not resume. Check that the agent is installed and signed in, then try again.'; }
      finally { button.disabled = false; }
    }

    async function loadInstalledAgents() {
      try {
        const data = await apiJson('/api/v1/agents/installed', { headers });
        if (data?.agents) {
          installedAgents = data.agents;
          const select = document.getElementById('launch-agent-select');
          if (select) {
            const selected = select.value;
            select.innerHTML = installedAgents.map(a =>
              `<option value="${escapeHtml(a.id)}">${escapeHtml(a.name)}${a.installed ? '' : ' [Not installed]'}</option>`
            ).join('');
            if (installedAgents.some(agent => agent.id === selected)) select.value = selected;
            updateAgentInstallHint();
          }
        }
      } catch (e) {
        document.getElementById('launch-agent-hint').textContent = 'Agent detection failed. Reopen New Session to retry, or launch Terminal Shell.';
      }
    }

    function updateAgentInstallHint() {
      const select = document.getElementById('launch-agent-select');
      const hint = document.getElementById('launch-agent-hint');
      if (!select || !hint) return;
      const agent = installedAgents.find(a => a.id === select.value);
      if (agent && !agent.installed) {
        hint.innerHTML = `<span style="color:var(--text-muted);">${escapeHtml(agent.name)} is not found on PATH. Run: <code>${escapeHtml(agent.install_command)}</code> then <code>${escapeHtml(agent.login_command)}</code>. Or launch <strong>Terminal Shell</strong> to set it up.</span>`;
      } else {
        hint.textContent = agent ? `${agent.name} is available on PATH. Provider login is checked by the CLI when it starts.` : 'Agent detection unavailable. Try launching the terminal shell.';
      }
      document.getElementById('launch-model-input').disabled = select.value === 'terminal';
      document.getElementById('launch-agent-btn').disabled = Boolean(agent && !agent.installed);
    }

    async function launchNewAgent(event) {
      if (event) event.preventDefault();
      const agent = document.getElementById('launch-agent-select')?.value;
      const repoPath = document.getElementById('launch-repo-input')?.value.trim();
      const model = document.getElementById('launch-model-input')?.value.trim() || undefined;
      const status = document.getElementById('launch-status');
      const btn = document.getElementById('launch-agent-btn');
      if (!agent || !repoPath) {
        if (status) status.textContent = 'Please provide an agent and a workspace directory.';
        return;
      }
      if (btn) btn.disabled = true;
      if (status) status.textContent = `Launching ${agent}...`;
      try {
        const res = await fetch('/api/v1/sessions/launch', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ agent, repo_path: repoPath, model })
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.message || `Launch failed (status ${res.status})`);
        }
        const record = await res.json();
        if (status) status.textContent = `Session ${record.session_id} launched! Opening conversation...`;
        await loadSessions();
        await loadChat(record.session_id);
        if (agent === 'terminal') toggleStream();
      } catch (err) {
        if (status) status.textContent = `Could not launch: ${err.message}`;
      } finally {
        updateAgentInstallHint();
      }
    }

    const workspaceBrowser = document.getElementById('workspace-browser');
    let workspacePath = '';
    let workspaceRequest = 0;
    async function browseWorkspace(path) {
      const request = ++workspaceRequest;
      workspacePath = '';
      document.getElementById('workspace-browser-select').disabled = true;
      const status = document.getElementById('workspace-browser-status');
      status.textContent = 'Loading folders...';
      try {
        const data = await apiJson('/api/v1/workspaces', {method:'POST', headers:{...headers,'Content-Type':'application/json'}, body:JSON.stringify({path})});
        if (request !== workspaceRequest) return;
        if (data.type === 'error') throw new Error(data.message);
        workspacePath = data.path;
        document.getElementById('workspace-browser-select').disabled = false;
        document.getElementById('workspace-browser-path').textContent = data.path;
        const roots = document.getElementById('workspace-browser-roots');
        const list = document.getElementById('workspace-browser-list');
        roots.replaceChildren(); list.replaceChildren();
        function add(container, label, target) {
          const button = document.createElement('button'); button.type = 'button';
          button.className = 'btn-control'; button.textContent = label;
          button.addEventListener('click', () => void browseWorkspace(target)); container.append(button);
        }
        for (const root of data.roots) add(roots, root, root);
        if (data.parent) add(list, 'Up one folder', data.parent);
        for (const folder of data.folders) add(list, folder.name, folder.path);
        status.textContent = data.truncated ? 'More folders exist. Enter a full path if needed.' : data.folders.length ? '' : 'No subfolders.';
      } catch (error) { if (request === workspaceRequest) status.textContent = error.message || 'Cannot open this folder.'; }
    }
    async function chooseWorkspace() {
      try {
        if (window.gnomcodeDesktop?.chooseWorkspace) {
          const folder = await window.gnomcodeDesktop.chooseWorkspace();
          if (folder) document.getElementById('launch-repo-input').value = folder;
        } else { workspaceBrowser.showModal(); await browseWorkspace(); }
      } catch { document.getElementById('launch-status').textContent = 'Folder picker unavailable. Enter the workspace path and retry.'; }
    }
    document.getElementById('workspace-browser-cancel').addEventListener('click', () => workspaceBrowser.close());
    document.getElementById('workspace-browser-select').addEventListener('click', () => {
      if (workspacePath) { document.getElementById('launch-repo-input').value = workspacePath; workspaceBrowser.close(); }
    });

    function resizePrompt() {
      const input = document.getElementById('chat-prompt-text');
      input.style.height = 'auto';
      input.style.height = `${Math.min(160, Math.max(36, input.scrollHeight))}px`;
    }
    document.getElementById('chat-prompt-text').addEventListener('input', resizePrompt);

    async function automationRequest(request) {
      const result = await apiJson('/api/v1/automation', {method:'POST',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify(request),signal:AbortSignal.timeout(15000)});
      if(result.type==='error')throw new Error(result.message);
      return result;
    }
    async function refreshAutomation() {
      const session=activeChatSession;
      if(!session)return;
      try {
        const [schedules,routing,notices]=await Promise.all([automationRequest({type:'schedules',session_id:session}),automationRequest({type:'routing'}),automationRequest({type:'notices'})]);
        if(session!==activeChatSession)return;
        const list=document.getElementById('schedule-list');
        const markup=schedules.messages.map(message=>`<p>${escapeHtml(new Date(message.due_at*1000).toLocaleString())} - ${escapeHtml(message.status)}${message.text?`<br>${escapeHtml(message.text.slice(0,120))}`:''}${message.status==='pending'?` <button type="button" class="btn-control" data-action="cancel-schedule" data-arg="${escapeHtml(message.id)}">Cancel</button>`:''}</p>`).join('') || '<p>No scheduled messages.</p>';
        if(list.innerHTML!==markup)list.innerHTML=markup;
        document.getElementById('reset-reconnect-status').textContent=schedules.reset_armed?'Reset reconnection armed. Waiting for fresh quota confirmation.':'Reset reconnection is not armed.';
        const policy=document.getElementById('routing-policy');
        if(document.activeElement!==policy && !policy.dataset.dirty)policy.value=routing.policy;
        const feedback=notices.notices.filter(item=>(item.conversation_id===session || !item.conversation_id)).slice(0,8).map(item=>`<p>${escapeHtml(item.message)}</p>`).join('');
        if(document.getElementById('automation-notices').innerHTML!==feedback)document.getElementById('automation-notices').innerHTML=feedback;
      }catch(error){document.getElementById('schedule-list').textContent=`Schedule status unavailable. ${error.message}`;}
    }
    document.getElementById('routing-policy').addEventListener('change',event=>{event.target.dataset.dirty='true';});
    let scheduling=false;
    async function scheduleDraft() {
      if(scheduling || !activeChatSession)return;
      const input=document.getElementById('chat-prompt-text'),text=input.value.trim(),session=activeChatSession;
      const date=new Date(document.getElementById('schedule-time').value);
      if(!text || !Number.isFinite(date.getTime()) || date.getTime()<=Date.now()){showNotice('Enter a message and a future delivery time.');return;}
      scheduling=true;
      try {
        await automationRequest({type:'schedule',session_id:session,id:crypto.randomUUID(),text,due_at:Math.floor(date.getTime()/1000)});
        if(session===activeChatSession && input.value.trim()===text){input.value='';chatDrafts.delete(session);resizePrompt();}
        showNotice('Schedule confirmed on your laptop. You can close your phone.');
        await refreshAutomation();
      } catch(error){showNotice(`Schedule not confirmed. Check saved messages before retrying. ${error.message}`);}
      finally{scheduling=false;}
    }
    async function cancelSchedule(id) {try{await automationRequest({type:'cancel_schedule',session_id:activeChatSession,id});await refreshAutomation();}catch(error){showNotice(error.message);}}
    async function saveRouting() {try{await automationRequest({type:'routing',policy:document.getElementById('routing-policy').value});delete document.getElementById('routing-policy').dataset.dirty;showNotice('Routing policy saved on this laptop.');}catch(error){showNotice(error.message);}}
    async function reconnectReset(enabled) {try{await automationRequest({type:'resume_on_reset',session_id:activeChatSession,enabled});showNotice(enabled?'Reconnection armed. It waits for fresh quota confirmation after reset.':'Reset reconnection cancelled.');}catch(error){showNotice(error.message);}}

    const sessionAttachments = new Map();

    async function renderSessionAttachments() {
      const bar = document.getElementById('chat-attachments-bar');
      if (!bar) return;
      const items = sessionAttachments.get(activeChatSession) || [];
      if (!items.length) {
        bar.hidden = true;
        bar.innerHTML = '';
        return;
      }
      const presentation = await import('/console-presentation.js');
      bar.innerHTML = presentation.renderAttachmentPills(items);
      bar.hidden = false;
    }

    function readFileAsBase64(file) {
      return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = () => {
          const result = String(reader.result || '');
          const comma = result.indexOf(',');
          resolve(comma >= 0 ? result.slice(comma + 1) : result);
        };
        reader.onerror = () => reject(new Error('Could not read file.'));
        reader.readAsDataURL(file);
      });
    }

    async function handleLocalAttachmentPick(file) {
      if (!file || !activeChatSession) return;
      const sessionId = activeChatSession;
      const statusEl = document.getElementById('chat-attach-status');
      const presentation = await import('/console-presentation.js');
      const checked = presentation.validateAttachmentCandidate(file);
      if (!checked.ok) {
        if (statusEl) statusEl.textContent = checked.reason;
        return;
      }
      if (statusEl) statusEl.textContent = '';
      const id = crypto.randomUUID();
      const item = { id, filename: checked.filename, size: checked.size, mime_type: checked.mime_type, status: 'uploading' };
      sessionAttachments.set(sessionId, [...(sessionAttachments.get(sessionId) || []), item]);
      if (sessionId === activeChatSession) void renderSessionAttachments();
      try {
        const data_base64 = await readFileAsBase64(file);
        const res = await fetch(`/api/v1/sessions/${encodeURIComponent(sessionId)}/attachments`, {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ filename: checked.filename, mime_type: checked.mime_type, size: checked.size, data_base64 })
        });
        if (!res.ok) {
          const msg = res.status === 404 ? 'This conversation has no workspace to attach files to.'
            : res.status === 413 ? 'Attachment is too large (15 MiB per file, 200 MiB per conversation).'
            : res.status === 415 ? 'Unsupported attachment type.'
            : 'Could not upload attachment.';
          throw new Error(msg);
        }
        const payload = await res.json();
        const workspace_path = payload?.workspace_path;
        if (!workspace_path) throw new Error('Could not upload attachment.');
        const updated = (sessionAttachments.get(sessionId) || []).map(entry => entry.id === id ? { ...entry, status: 'saved', workspace_path } : entry);
        sessionAttachments.set(sessionId, updated);
      } catch (err) {
        const msg = err instanceof Error ? err.message : 'Upload failed.';
        const updated = (sessionAttachments.get(sessionId) || []).map(entry => entry.id === id ? { ...entry, status: 'error', error: msg } : entry);
        sessionAttachments.set(sessionId, updated);
        if (statusEl && sessionId === activeChatSession) statusEl.textContent = msg;
      }
      if (sessionId === activeChatSession) void renderSessionAttachments();
    }

    document.getElementById('chat-attachments-bar')?.addEventListener('click', event => {
      const btn = event.target.closest('[data-attachment-remove]');
      if (!btn || !activeChatSession) return;
      const id = btn.dataset.attachmentRemove;
      sessionAttachments.set(activeChatSession, (sessionAttachments.get(activeChatSession) || []).filter(entry => entry.id !== id));
      void renderSessionAttachments();
    });

    async function sendPromptText(rawText, { fromComposer = false } = {}) {
      const input = document.getElementById('chat-prompt-text');
      const draft = typeof rawText === 'string' ? rawText.trim() : '';
      const readyAttachments = fromComposer ? (sessionAttachments.get(activeChatSession) || []).filter(item => item.status === 'saved' && item.workspace_path) : [];
      const presentation = await import('/console-presentation.js');
      const text = presentation.formatPromptWithAttachments(draft, readyAttachments);
      if (!text || !activeChatSession || document.getElementById('chat-send-btn').disabled) return;
      const status = document.getElementById('chat-delivery-status');
      const sessionId = activeChatSession;
      const deletionGeneration = chatDeletionGenerations.get(sessionId) || 0;
      const optimistic = {id:crypto.randomUUID(),text,created_at:new Date().toISOString(),baseline:chatEvents.map(event => event.event_id),delivered:false};
      optimisticPrompts.set(sessionId, [...(optimisticPrompts.get(sessionId) || []),optimistic]);
      void renderCurrentChat();
      pendingPrompts.add(sessionId);
      const button = document.getElementById('chat-send-btn'); button.disabled = true;
      void renderChatQuickActions();
      if (status) status.textContent = '';
      try {
        const res = await fetch(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/prompt`, {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ text })
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.message || (res.status === 404 ? 'The agent terminal is no longer connected. Open Agent terminal to check it, or start a new conversation. Your draft is saved.' : 'The terminal did not accept the prompt. Check Agent terminal and retry.'));
        }
        if ((chatDeletionGenerations.get(sessionId) || 0) !== deletionGeneration) return;
        optimistic.delivered = true;
        sessionAttachments.set(sessionId, (sessionAttachments.get(sessionId) || []).filter(entry => entry.status !== 'saved'));
        if (sessionId === activeChatSession) void renderSessionAttachments();
        if (sessionId === activeChatSession) void renderCurrentChat();
        if (fromComposer) {
          sessionAttachments.set(sessionId, (sessionAttachments.get(sessionId) || []).filter(entry => entry.status !== 'saved'));
          if (sessionId === activeChatSession) void renderSessionAttachments();
          if (chatDrafts.get(sessionId)?.trim() === draft) chatDrafts.delete(sessionId);
          if (sessionId === activeChatSession) {
            if (input.value.trim() === draft) { input.value = ''; resizePrompt(); }
            const attachStatus = document.getElementById('chat-attach-status');
            if (attachStatus) attachStatus.textContent = '';
            if (status) status.textContent = '';
            input.focus();
          }
        }
        if (sessionId === activeChatSession) setTimeout(() => loadChat(sessionId, false, true), 1000);
      } catch (err) {
        optimisticPrompts.set(sessionId, (optimisticPrompts.get(sessionId) || []).filter(item => item.id !== optimistic.id));
        if (sessionId === activeChatSession) void renderCurrentChat();
        if (sessionId === activeChatSession) showNotice(`Could not send. ${err.message}`);
      } finally {
        pendingPrompts.delete(sessionId);
        button.disabled = pendingPrompts.has(activeChatSession);
        void renderChatQuickActions();
      }
    }

    async function sendPrompt(event) {
      if (event) event.preventDefault();
      await sendPromptText(document.getElementById('chat-prompt-text')?.value, { fromComposer: true });
    }

    let quickActionsCache = null;
    async function getQuickActions() {
      const presentation = await import('/console-presentation.js');
      if (!quickActionsCache) {
        quickActionsCache = presentation.loadStoredQuickActions();
        try {
          const remote = await apiJson('/api/v1/quick-actions');
          if (Array.isArray(remote?.actions)) quickActionsCache = presentation.saveStoredQuickActions(remote.actions);
        } catch { /* daemon list unavailable: keep the local cache */ }
      }
      return { presentation, items: quickActionsCache };
    }

    async function persistQuickActions(items) {
      const presentation = await import('/console-presentation.js');
      const err = document.getElementById('quick-actions-error');
      try {
        const saved = await apiJson('/api/v1/quick-actions', { method: 'PUT', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify({ actions: presentation.toWireQuickActions(items) }) });
        quickActionsCache = presentation.saveStoredQuickActions(Array.isArray(saved?.actions) ? saved.actions : items);
        if (err) { err.hidden = true; err.textContent = ''; }
      } catch (error) {
        if (err) { err.textContent = `Could not save quick prompts: ${error.message}`; err.hidden = false; }
        return;
      }
      void renderChatQuickActions();
      renderQuickActionsDialogList(quickActionsCache);
    }

    async function renderChatQuickActions() {
      const host = document.getElementById('chat-quick-actions');
      if (!host) return;
      const session = ideSessions.find(row => row.id === activeChatSession);
      const imported = session?.source === 'imported';
      host.hidden = imported || !activeChatSession;
      if (host.hidden) return;
      const { presentation, items } = await getQuickActions();
      const disabled = pendingPrompts.has(activeChatSession);
      host.innerHTML = presentation.renderQuickActionsStrip(items, { disabled });
    }

    function renderQuickActionsDialogList(items) {
      const list = document.getElementById('quick-actions-list');
      if (!list) return;
      list.innerHTML = items.map(item => `<div class="sp-quick-action-row"><div><strong>${escapeHtml(item.label)}</strong><p>${escapeHtml(item.prompt)}</p></div><button type="button" class="btn-control" data-action="remove-quick-action" data-arg="${escapeHtml(item.id)}" ${items.length <= 1 ? 'disabled' : ''}>Remove</button></div>`).join('');
    }

    async function sendQuickAction(actionId) {
      const { items } = await getQuickActions();
      const match = items.find(item => item.id === actionId);
      if (match) await sendPromptText(match.prompt, { fromComposer: false });
    }

    async function openQuickActionsDialog() {
      const dialog = document.getElementById('quick-actions-dialog');
      if (!dialog) return;
      const { items } = await getQuickActions();
      renderQuickActionsDialogList(items);
      const err = document.getElementById('quick-actions-error');
      if (err) { err.hidden = true; err.textContent = ''; }
      if (!dialog.open) dialog.showModal();
    }

    async function addQuickActionFromDialog(event) {
      event.preventDefault();
      const labelEl = document.getElementById('quick-action-label');
      const promptEl = document.getElementById('quick-action-prompt');
      const err = document.getElementById('quick-actions-error');
      const presentation = await import('/console-presentation.js');
      const clean = presentation.normalizeQuickAction({ id: `custom-${crypto.randomUUID().slice(0, 8)}`, label: labelEl?.value, prompt: promptEl?.value, builtin: false });
      if (!clean) {
        if (err) { err.textContent = 'Enter a label and a plain prompt (up to 4000 characters, never starting with /).'; err.hidden = false; }
        return;
      }
      if (err) { err.hidden = true; err.textContent = ''; }
      const { items } = await getQuickActions();
      if (items.length >= presentation.MAX_QUICK_ACTIONS) {
        if (err) { err.textContent = `Up to ${presentation.MAX_QUICK_ACTIONS} quick prompts. Remove one first.`; err.hidden = false; }
        return;
      }
      await persistQuickActions([...items, clean]);
      if (labelEl) labelEl.value = '';
      if (promptEl) promptEl.value = '';
    }

    async function removeQuickAction(actionId) {
      const { items } = await getQuickActions();
      const next = items.filter(item => item.id !== actionId);
      if (!next.length) return;
      await persistQuickActions(next);
    }

    async function resetQuickActions() {
      const presentation = await import('/console-presentation.js');
      await persistQuickActions(presentation.DEFAULT_QUICK_ACTIONS);
    }

    document.getElementById('chat-prompt-text').addEventListener('keydown', event => {
      if (event.key === 'Enter' && !event.shiftKey && !event.isComposing) {
        event.preventDefault();
        document.getElementById('chat-prompt-form').requestSubmit();
      }
    });

    function toggleStream() {
      const terminal = document.getElementById('chat-terminal-view');
      const stream = document.getElementById('chat-stream');
      const btn = document.getElementById('chat-toggle-stream');
      if (!terminal || !stream) return;
      if (terminal.hidden) {
        terminal.hidden = false;
        stream.hidden = true;
        if (btn) btn.textContent = 'Show conversation';
        connectTerminalWs();
      } else {
        terminal.hidden = true;
        stream.hidden = false;
        if (btn) btn.textContent = 'Agent terminal';
        if (chatWs) { chatWs.close(); chatWs = null; }
      }
    }

    const workspaceTerminals = new Map();
    let workspaceTerminalBusy = false;
    async function openWorkspaceTerminal() {
      if (!activeChatSession || workspaceTerminalBusy) return;
      const source = activeChatSession;
      workspaceTerminalBusy = true;
      try {
        let entry = workspaceTerminals.get(source);
        if (!entry) {
          const record = await apiJson(`/api/v1/sessions/${encodeURIComponent(source)}/terminal`, {method:'POST',headers});
          document.querySelectorAll('#workspace-terminal-screen [data-disconnected]').forEach(node => node.remove());
          const element = document.createElement('div');
          const terminal = new Terminal({cols:80,rows:12,scrollback:3000,fontSize:13,fontFamily:'Consolas, monospace',theme:{background:'#0d0d0f',foreground:'#f6f7f8'},allowProposedApi:false});
          document.getElementById('workspace-terminal-screen').append(element);
          const fit = new FitAddon.FitAddon(); terminal.loadAddon(fit);
          terminal.open(element);
          const url = new URL(`/api/v1/sessions/${encodeURIComponent(record.session_id)}/pty`,location.href);
          url.protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
          url.searchParams.set('token',token);
          const socket = new WebSocket(url); socket.binaryType = 'arraybuffer';
          const resize = () => {if (element.hidden || document.getElementById('workspace-terminal-pane').hidden) return; fit.fit(); if(socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify({resize:{cols:terminal.cols,rows:terminal.rows}}));};
          const observer = new ResizeObserver(resize); observer.observe(document.getElementById('workspace-terminal-screen'));
          socket.onopen = resize;
          socket.onmessage = event => terminal.write(typeof event.data === 'string' ? event.data : new Uint8Array(event.data));
          socket.onerror = () => terminal.writeln('\r\nTerminal connection unavailable. Close and reopen this pane.');
          socket.onclose = () => { observer.disconnect(); workspaceTerminals.delete(source); terminal.dispose(); element.textContent = 'Terminal disconnected. Reopen the pane to reconnect.'; element.dataset.disconnected = 'true'; };
          terminal.onData(data => { if (socket.readyState === WebSocket.OPEN) socket.send(new TextEncoder().encode(data)); });
          entry = {element,terminal,socket,record,resize}; workspaceTerminals.set(source,entry);
        }
        if (source !== activeChatSession) return;
        document.querySelectorAll('#workspace-terminal-screen>div').forEach(element => element.hidden = element !== entry.element);
        document.getElementById('workspace-terminal-path').textContent = entry.record.repo_path;
        document.getElementById('workspace-terminal-pane').hidden = false;
        requestAnimationFrame(entry.resize);
        entry.terminal.focus();
      } catch { document.getElementById('chat-delivery-status').textContent = 'Could not open a shell in this workspace. Check that the directory still exists.'; }
      finally { workspaceTerminalBusy = false; }
    }

    function connectTerminalWs() {
      if(chatWs || !activeChatSession)return;
      const element=document.getElementById('chat-terminal-view');
      element.replaceChildren();
      const terminal=new Terminal({cols:80,rows:24,scrollback:3000,fontSize:13,fontFamily:'Consolas, monospace',allowProposedApi:false});
      const fit=new FitAddon.FitAddon();terminal.loadAddon(fit);terminal.open(element);
      const url=new URL(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/pty`,location.href);url.protocol=location.protocol==='https:'?'wss:':'ws:';url.searchParams.set('token',token);
      const socket=new WebSocket(url);chatWs=socket;socket.binaryType='arraybuffer';
      const resize=()=>{if(element.hidden)return;fit.fit();if(socket.readyState===WebSocket.OPEN)socket.send(JSON.stringify({resize:{cols:terminal.cols,rows:terminal.rows}}));};
      const observer=new ResizeObserver(resize);observer.observe(element);
      socket.onopen=()=>{resize();terminal.focus();};
      socket.onmessage=event=>terminal.write(typeof event.data==='string'?event.data:new Uint8Array(event.data));
      const input=terminal.onData(data=>{if(socket.readyState===WebSocket.OPEN)socket.send(new TextEncoder().encode(data));});
      socket.onerror=()=>terminal.writeln('\r\nAgent terminal unavailable. Reopen this view to retry.');
      socket.onclose=()=>{observer.disconnect();input.dispose();terminal.dispose();if(chatWs===socket){chatWs=null;element.textContent='Agent terminal disconnected. Reopen this view to reconnect.';}};
      requestAnimationFrame(resize);
    }

    function closeChangesPanel() {
      const panel = document.getElementById('chat-changes-panel');
      if (panel) panel.hidden = true;
    }

    async function loadSessionChanges() {
      const panel = document.getElementById('chat-changes-panel');
      const body = document.getElementById('chat-changes-body');
      const btn = document.getElementById('chat-changes-btn');
      if (!panel || !body || !activeChatSession) return;
      const presentation = await import('/console-presentation.js');
      body.innerHTML = presentation.renderChangesView({ loading: true });
      if (btn) btn.disabled = true;
      try {
        const res = await fetch(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/changes`, { headers });
        if (!res.ok) {
          const message = res.status === 413
            ? 'Diff exceeded the 1 MiB read limit on this desktop.'
            : res.status === 504
              ? 'Git diff timed out after 10 seconds.'
              : res.status === 404
                ? 'This conversation has no workspace on this desktop.'
                : 'This workspace is not a git repository.';
          body.innerHTML = presentation.renderChangesView({ error: message });
          return;
        }
        const data = await res.json();
        body.innerHTML = presentation.renderChangesView(data);
      } catch {
        body.innerHTML = presentation.renderChangesView({ error: 'Could not read git changes from the local daemon.' });
      } finally {
        if (btn) btn.disabled = false;
      }
    }

    function toggleChangesPanel() {
      const panel = document.getElementById('chat-changes-panel');
      if (!panel) return;
      panel.hidden = !panel.hidden;
      if (!panel.hidden) void loadSessionChanges();
    }

    function activeRepoForFiles() {
      const hosted = hostedSessions.find(row => row.session_id === activeChatSession);
      const session = ideSessions.find(row => row.id === activeChatSession);
      return hosted?.repo_path || session?.repo_path || session?.repo_root || document.getElementById('session-action-copy')?.dataset.path || '';
    }

    function closeFilesPanel() {
      const panel = document.getElementById('chat-files-panel');
      if (panel) panel.hidden = true;
    }

    async function loadSessionFileListing(subpath = '') {
      const listEl = document.getElementById('chat-files-list');
      if (!listEl || !activeChatSession) return;
      const presentation = await import('/console-presentation.js');
      const repo = activeRepoForFiles();
      if (!repo) {
        listEl.innerHTML = presentation.renderFileEntries({ error: 'Workspace repository path is unavailable for this session.' });
        return;
      }
      listEl.innerHTML = presentation.renderFileEntries({ loading: true });
      try {
        const res = await fetch(`/api/v1/repos/files?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(subpath)}`, { headers });
        if (res.ok) {
          const listing = await res.json();
          listEl.innerHTML = presentation.renderFileEntries(listing);
          return;
        }
        listEl.innerHTML = presentation.renderFileEntries({ error: res.status === 404 ? 'Folder not found inside the workspace.' : 'Could not list this workspace folder.' });
      } catch {
        listEl.innerHTML = presentation.renderFileEntries({ error: 'Could not list workspace files.' });
      }
    }

    async function readSessionFile(rawPath) {
      const bodyEl = document.getElementById('chat-file-body');
      const inputEl = document.getElementById('chat-file-path-input');
      if (!bodyEl || !activeChatSession) return;
      const presentation = await import('/console-presentation.js');
      const check = presentation.normalizeWorkspaceRelPath(rawPath);
      if (!check.ok) {
        bodyEl.innerHTML = presentation.renderFileViewer({ error: check.error });
        return;
      }
      const repo = activeRepoForFiles();
      if (!repo) {
        bodyEl.innerHTML = presentation.renderFileViewer({ error: 'Workspace repository path is unavailable for this session.' });
        return;
      }
      if (inputEl) inputEl.value = check.path;
      bodyEl.innerHTML = presentation.renderFileViewer({ loading: true });
      try {
        const res = await fetch(`/api/v1/repos/file?repo=${encodeURIComponent(repo)}&path=${encodeURIComponent(check.path)}`, { headers });
        if (!res.ok) {
          const msg = res.status === 413
            ? 'File exceeded the 512 KiB read cap on desktop.'
            : res.status === 404
              ? 'File not found inside the workspace.'
              : res.status === 422
                ? 'Binary or non-UTF-8 files are not shown.'
                : 'File path is outside the workspace or unreadable.';
          bodyEl.innerHTML = presentation.renderFileViewer({ error: msg });
          return;
        }
        const data = await res.json();
        bodyEl.innerHTML = presentation.renderFileViewer(data);
      } catch {
        bodyEl.innerHTML = presentation.renderFileViewer({ error: 'Could not read file from local daemon.' });
      }
    }

    function toggleFilesPanel() {
      const panel = document.getElementById('chat-files-panel');
      if (!panel) return;
      panel.hidden = !panel.hidden;
      if (!panel.hidden) void loadSessionFileListing('');
    }
    let artifactsOpen = false;
    let artifactItems = [];
    let selectedArtifactId = '';

    function resetArtifactsPanel() {
      artifactsOpen = false;
      artifactItems = [];
      selectedArtifactId = '';
      const panel = document.getElementById('chat-artifacts-panel');
      const btn = document.getElementById('chat-artifacts-btn');
      if (panel) panel.hidden = true;
      if (btn) btn.setAttribute('aria-expanded', 'false');
    }

    async function renderArtifactsState() {
      const presentation = await import('/console-presentation.js');
      const list = document.getElementById('chat-artifacts-list');
      const preview = document.getElementById('chat-artifact-preview');
      if (!list || !preview) return;
      list.innerHTML = presentation.renderArtifactsList(artifactItems, selectedArtifactId);
      const selected = artifactItems.find((item, idx) => String(item.id || item.path || idx) === String(selectedArtifactId)) || null;
      preview.innerHTML = presentation.renderArtifactPreview(selected);
    }

    async function loadArtifacts() {
      const sessionId = activeChatSession;
      if (!sessionId) return;
      const list = document.getElementById('chat-artifacts-list');
      const preview = document.getElementById('chat-artifact-preview');
      const changesBadge = document.getElementById('chat-artifacts-changes');
      if (list) list.innerHTML = '<p class="sp-empty">Loading artifacts...</p>';
      if (preview) preview.innerHTML = '';
      const headers = token ? { Authorization: `Bearer ${token}` } : {};
      let items = [];
      let changesCount = null;
      let failed = false;
      try {
        const res = await fetch(`/api/v1/sessions/${encodeURIComponent(sessionId)}/artifacts`, { headers, signal: AbortSignal.timeout(10000) });
        if (res.ok) {
          const data = await res.json();
          items = Array.isArray(data.artifacts) ? data.artifacts : [];
          if (typeof data.changes_count === 'number') changesCount = data.changes_count;
        } else failed = true;
      } catch (_) { failed = true; }
      if (sessionId !== activeChatSession) return;
      if (changesBadge) changesBadge.textContent = failed ? 'Artifacts unavailable for this conversation' : changesCount === null ? 'Artifacts' : `${changesCount} workspace change${changesCount === 1 ? '' : 's'}`;
      artifactItems = items;
      selectedArtifactId = items[0] ? String(items[0].id || items[0].path || 0) : '';
      await renderArtifactsState();
    }

    function toggleArtifacts(forceOpen) {
      const panel = document.getElementById('chat-artifacts-panel');
      const btn = document.getElementById('chat-artifacts-btn');
      if (!panel || !btn) return;
      artifactsOpen = typeof forceOpen === 'boolean' ? forceOpen : !artifactsOpen;
      panel.hidden = !artifactsOpen;
      btn.setAttribute('aria-expanded', String(artifactsOpen));
      if (artifactsOpen) void loadArtifacts();
    }

    document.getElementById('chat-artifacts-list')?.addEventListener('click', event => {
      const item = event.target.closest('[data-artifact-id]');
      if (!item) return;
      selectedArtifactId = item.dataset.artifactId || '';
      void renderArtifactsState();
    });

    async function loadChat(sessionId, earlier = false, silent = false) {
      if (!sessionId) return;
      if (silent && activeChatSession !== sessionId) return;
      if (!silent && !earlier) {
        if (activeChatSession !== sessionId) {
          selectChatSession(sessionId);
          closeChangesPanel();
          closeFilesPanel();
        }
        if (activeChatSession !== sessionId) { selectChatSession(sessionId); resetArtifactsPanel(); }
        void renderSessionAttachments();
        const attachStatus = document.getElementById('chat-attach-status');
        if (attachStatus) attachStatus.textContent = '';
        chatEvents = [];
        if (chatWs) { chatWs.close(); chatWs = null; }
        document.getElementById('chat-terminal-view').hidden = true;
        document.getElementById('chat-stream').hidden = false;
        document.getElementById('chat-toggle-stream').textContent = 'Agent terminal';
      }
      const request = ++chatRequest;
      if (!silent && !earlier) storeFlag('gnomcode-last-chat', JSON.stringify({ id: sessionId, at: Date.now() }));
      document.getElementById('ide-launcher').hidden = true;
      const session = ideSessions.find(session => session.id === sessionId);
      const hosted = hostedSessions.find(session => session.session_id === sessionId);
      document.getElementById('session-chat-heading').textContent = session?.title || hosted?.title || 'New conversation';
      document.getElementById('chat-session-label').textContent = hosted?.agent || session?.agent || 'Agent';
      const imported = session?.source === 'imported';
      document.getElementById('chat-resume').hidden = !session?.can_resume;
      document.getElementById('reset-arm').hidden = !session?.can_reset;
      document.getElementById('reset-capability').hidden = imported || Boolean(session?.can_reset);

      document.getElementById('chat-prompt-form').hidden = imported;
      document.getElementById('chat-quick-actions').hidden = imported;
      document.getElementById('chat-automation').hidden = imported;
      document.getElementById('chat-toggle-stream').hidden = imported;
      if (!silent) void renderChatQuickActions();
      const banner = document.getElementById('imported-session-banner');
      if (banner) {
        banner.hidden = !imported;
        if (imported) {
          const agentName = session?.agent === 'antigravity' ? 'Google Antigravity' : (session?.agent || 'your agent application');
          document.getElementById('imported-banner-text').textContent = `This conversation is active in ${agentName}. Responses, tokens and quota are observed in real time.`;
          document.getElementById('imported-focus-btn').textContent = session?.agent === 'antigravity' ? 'Open in Antigravity' : 'Focus Agent Window';
        }
      }
      if(imported)document.getElementById('chat-session-label').textContent += ' · Observed';
      void showChatLimits();
      renderIdeSessions();
      showCurrentChatModel();
      if (!silent) { loadChatModels(); void refreshAutomation(); }
      const panel = document.getElementById('session-chat-panel');
      const status = document.getElementById('chat-status');
      if (!silent && status) status.textContent = 'Loading conversation...';
      try {
        const before = earlier && chatEvents.length ? `&before=${encodeURIComponent(chatEvents[0].event_id)}` : '';
        const page = await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat?limit=500${before}`, { headers });
        if (request !== chatRequest || sessionId !== activeChatSession) return;
        const merged = new Map((earlier || silent ? chatEvents : []).map(event => [event.event_id, event]));
        page.forEach(event => merged.set(event.event_id, event));
        chatEvents = [...merged.values()].sort((a,b) => a.turn_index-b.turn_index || a.created_at.localeCompare(b.created_at) || a.event_id.localeCompare(b.event_id));
        const presentation = await import('/console-presentation.js');
        const stream = document.getElementById('chat-stream');
        const oldHeight = stream.scrollHeight;
        const oldTop = stream.scrollTop;
        await renderCurrentChat();
        if (earlier) stream.scrollTop = oldTop + stream.scrollHeight - oldHeight;
        else if (!silent) stream.scrollTop = stream.scrollHeight;
        if (!silent) document.getElementById('chat-earlier').hidden = page.length < 500;
        document.getElementById('chat-delete').hidden = chatEvents.length === 0;
        const messageCount = chatEvents.filter(event => (event.role === 'user' || event.role === 'assistant') && event.content?.trim()).length;
        if (status) status.textContent = messageCount ? `${messageCount} message${messageCount === 1 ? '' : 's'}` : '';
        panel.hidden = activeView !== 'sessions';
        if (!silent) requestAnimationFrame(resizePrompt);
      } catch {
        if (request !== chatRequest || sessionId !== activeChatSession) return;
        if (!silent && status) status.textContent = 'Could not load conversation history. Reload the view and check the desktop connection.';
        panel.hidden = activeView !== 'sessions';
        if (!silent) requestAnimationFrame(resizePrompt);
      }
    }

    async function deleteChat() {
      if (!activeChatSession || !await confirmAction('Delete Gnomcode’s local chat history for this session? Session titles, agent-owned logs and backups are not deleted; SQLite files may retain remnants. Changed source IDs, restored backups or a database reset can reimport messages.', 'Delete history')) return;
      const sessionId = activeChatSession;
      try {
        await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat`, { method: 'DELETE', headers });
        chatDeletionGenerations.set(sessionId, (chatDeletionGenerations.get(sessionId) || 0) + 1);
        optimisticPrompts.delete(sessionId);
        handoffNotices.delete(sessionId);
        if (activeChatSession === sessionId) await loadChat(sessionId);
      } catch { document.getElementById('chat-status').textContent = 'Could not delete local chat history.'; }
    }

    // Conversation list: open on desktop unless the person collapsed it; phones start collapsed.
    function storedFlag(key) { try { return localStorage.getItem(key); } catch (_) { return null; } }
    function storeFlag(key, value) { try { localStorage.setItem(key, value); } catch (_) { /* storage unavailable */ } }
    function applyChatList(collapsed) {
      document.body.classList.toggle('chat-list-collapsed', collapsed);
      document.getElementById('chat-list-toggle').setAttribute('aria-expanded', String(!collapsed));
    }
    applyChatList(matchMedia('(max-width: 760px)').matches || storedFlag('gnomcode-chat-list-collapsed') === '1');
    // The rail is a grid column: body[data-rail] drives its width, so content moves and nothing is covered.
    function applyRail(expanded) {
      document.body.dataset.rail = expanded ? 'expanded' : 'collapsed';
      const toggle = document.getElementById('navigation-toggle');
      toggle.setAttribute('aria-expanded', String(expanded));
      toggle.setAttribute('aria-label', expanded ? 'Collapse navigation' : 'Expand navigation');
      toggle.title = toggle.getAttribute('aria-label');
      toggle.querySelector('.rail-label').textContent = toggle.getAttribute('aria-label');
    }
    function toggleNavigation() {
      const expanded = document.body.dataset.rail !== 'expanded';
      applyRail(expanded);
      storeFlag('gnomcode-rail-expanded', expanded ? '1' : '0');
    }
    applyRail(storedFlag('gnomcode-rail-expanded') === '1');
    function toggleChatList() {
      const collapsed = !document.body.classList.contains('chat-list-collapsed');
      applyChatList(collapsed);
      if (!matchMedia('(max-width: 760px)').matches) storeFlag('gnomcode-chat-list-collapsed', collapsed ? '1' : '0');
    }

    document.addEventListener('click', event => {
      const project = event.target.closest('[data-project-id]');
      if (!project) return;
      event.preventDefault(); document.getElementById('project-filter').value = project.dataset.projectId;
      currentPage = 1; lastSessionsJson = ''; switchTab('sessions');
    });

    document.getElementById('sessions-table').addEventListener('click', async event => {
      const button = event.target.closest('[data-session-action="focus"]');
      if (!button || button.disabled) return;
      button.disabled = true;
      const status = document.getElementById('session-action-status');
      document.getElementById('session-action-copy').hidden = true;
      status.textContent = 'Opening agent window...';
      try {
        const response = await fetch(`/api/v1/sessions/${encodeURIComponent(button.dataset.sessionId)}/focus`, { method: 'POST', headers });
        if (!response.ok) throw new Error('Focus failed');
        const result = await response.json(); status.textContent = result.message || (result.focused ? 'Agent window opened.' : 'Session window unavailable.');
        const copy = document.getElementById('session-action-copy'); copy.hidden = result.focused || !result.repo_path; copy.dataset.path = result.repo_path || '';
      } catch (_) { status.textContent = 'Focus request failed. The session may no longer be available.'; }
      finally { button.disabled = false; }
    });
    async function copySessionWorkspace() {
      const status = document.getElementById('session-action-status');
      try { await navigator.clipboard.writeText(document.getElementById('session-action-copy').dataset.path); status.textContent = 'Workspace path copied.'; }
      catch (_) { status.textContent = 'Could not copy the path. Check clipboard access and retry.'; }
    }
    async function loadSessions() {
      try {
        if (!projectsLoaded) await loadProjects();
        const requestedPage = currentPage;
        const request = ++conversationRequest;
        const project = document.getElementById('project-filter').value;
        const agent = document.getElementById('agent-filter').value;
        const status = document.getElementById('status-filter').value;
        const res = await apiJson(`/api/v1/sessions?page=${currentPage}&limit=${pageLimit}&agent=${agent}&status=${status}&project=${encodeURIComponent(project)}`, { headers, signal: AbortSignal.timeout(15000) });
        if (!res) return false;
        if (project !== document.getElementById('project-filter').value || requestedPage !== currentPage || agent !== document.getElementById('agent-filter').value || status !== document.getElementById('status-filter').value) return true;

        const jsonStr = JSON.stringify(res);
        try { hostedSessions = (await apiJson('/api/v1/sessions/hosted', { headers })).sessions || []; } catch { hostedSessions = []; }
        const conversations = await apiJson(`/api/v1/conversations?page=${requestedPage}&limit=${pageLimit}&agent=${agent}&status=${status}&project=${encodeURIComponent(project)}&search=${encodeURIComponent(document.getElementById('ide-session-search').value)}&source=${encodeURIComponent(document.getElementById('conversation-source').value)}&include_subagents=${document.getElementById('include-subagents').checked}`, {headers});
        if(request !== conversationRequest)return true;
        const selected = ideSessions.find(session=>session.id===activeChatSession);
        conversationRows = Array.isArray(conversations) ? conversations : (conversations.sessions || []);
        ideSessions = [...conversationRows];
        if(selected && !ideSessions.some(session=>session.id===selected.id))ideSessions.push(selected);
        reopenLastChat();
        currentPage = (conversations && conversations.page) || requestedPage || 1;
        const totalPages = (conversations && conversations.total_pages) || 1;
        const totalCount = (conversations && conversations.total_count) != null ? conversations.total_count : conversationRows.length;
        document.getElementById('pagination-info').textContent = `${currentPage} / ${totalPages} · ${totalCount} conversations`;
        document.getElementById('pagination-controls').hidden = totalPages <= 1;
        document.getElementById('prev-btn').disabled = currentPage <= 1;
        document.getElementById('next-btn').disabled = currentPage >= totalPages;
        void showChatLimits();
        if (jsonStr === lastSessionsJson) { await renderIdeSessions(); return true; }

        const presentation = await import('/console-presentation.js');
        presentation.updateSessions(document.getElementById('sessions-table'), res.sessions || [], { focus: true });
        await renderIdeSessions();
        lastSessionsJson = jsonStr;
        return true;
      } catch (e) {
        console.error('Failed to load sessions', e);
        document.getElementById('chat-header-limits').textContent = 'Limits unavailable · UNKNOWN';
        lastSessionsJson = '';
        showConnectionError(e);
        document.getElementById('sessions-table').textContent = 'Session data is unavailable. Retry Reload view when the local daemon is reachable.';
        document.getElementById('ide-session-list').textContent = 'Sessions unavailable. Retry Reload view.';
        return false;
      }
    }

    function formatResetCountdown(resetsAtIso, windowKind) {
      if (!resetsAtIso) return "Rolling";
      const target = new Date(resetsAtIso);
      const now = new Date();
      const diffMs = target.getTime() - now.getTime();
      if (diffMs <= 0) return "Resetting now";

      const diffHours = Math.floor(diffMs / (1000 * 60 * 60));
      const diffMinutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
      const diffDays = Math.floor(diffHours / 24);
      const remHours = diffHours % 24;

      const timeStr = target.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
      const dateStr = target.toLocaleDateString([], { month: 'short', day: 'numeric' });

      if (diffDays > 0) {
        return `in ${diffDays}d ${remHours}h (${dateStr}, ${timeStr})`;
      } else if (diffHours > 0) {
        return `in ${diffHours}h ${diffMinutes}m (${timeStr})`;
      } else {
        return `in ${diffMinutes}m (${timeStr})`;
      }
    }

    const accountAccessDisclosure='Linking this native app makes its available conversations and supported session controls accessible from any device signed in to your account, anywhere. This includes devices already signed in and devices you sign in to later. No phone pairing or per-desktop approval will be required. This desktop must stay awake with Gnomcode running. Protect your account: anyone with access to it can use these controls. You can disable account access in Devices or disconnect this desktop.';
    function confirmAccountAccess(){return confirmAction(accountAccessDisclosure,'Link and allow account access');}
    let accountWriteBusy = false;
    async function signInAccount(){
      if(accountWriteBusy)return;
      accountWriteBusy=true;
      const button=document.getElementById('auth-signin-button'),password=document.getElementById('auth-password'),status=document.getElementById('auth-pair-status');
      button.disabled=true;button.textContent='Signing in...';status.hidden=false;
      try{
        if(!await confirmAccountAccess()){status.hidden=true;return;}
        await apiJson('/api/v1/auth/signin',{method:'POST',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify({email:document.getElementById('auth-email').value.trim(),password:password.value,remote_access_confirmed:true}),signal:AbortSignal.timeout(15000)});
        password.value='';status.textContent='Signed in. Your desktop will appear in Devices.';await checkAuthSession();
      }catch{status.textContent='Sign-in failed. Check your email, password and connection. Browser accounts can use the connection option below.';}
      finally{accountWriteBusy=false;button.disabled=false;button.textContent='Sign in';}
    }
    async function pairTokenDirectly() {
      if (accountWriteBusy) return;
      const input = document.getElementById('auth-token-input');
      const status = document.getElementById('auth-pair-status');
      if (!input || !status) return;
      const val = input.value.trim();
      if (!val) return;

      accountWriteBusy = true; input.disabled = true;
      const button = document.getElementById('auth-pair-button'); button.disabled = true; button.textContent = 'Connecting...';
      status.hidden = false;
      status.style.color = 'var(--text-muted)';
      status.textContent = 'Connecting this desktop...';

      try {
        let sessionData = null;
        try {
          sessionData = JSON.parse(val);
        } catch {
          try {
            sessionData = JSON.parse(atob(val));
          } catch {
            sessionData = { access_token: val, token_type: 'bearer' };
          }
        }

        if(!await confirmAccountAccess()){status.hidden=true;return;}
        sessionData.remote_access_confirmed=true;
        const res = await fetch('/api/v1/auth/session', {
          method: 'POST', signal: AbortSignal.timeout(30000),
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify(sessionData)
        });

        if (res.ok) {
          status.style.color = 'var(--green)';
          status.textContent = 'Desktop connected. Waiting for its next metadata update.';
          input.value = '';
          input.type = 'password';
          const visibility = document.querySelector('[data-action="toggle-pair-token"]');
          visibility.setAttribute('aria-pressed', 'false');
          visibility.textContent = 'Show token';
          if (await checkAuthSession()) document.getElementById('auth-cloud-status').focus({ preventScroll: true });
        } else {
          status.style.color = 'var(--red)';
          status.textContent = 'Failed to save session. Check token and try again.';
        }
      } catch (err) {
        status.style.color = 'var(--red)';
        status.textContent = 'Connection could not be confirmed. Reload account status before retrying.';
      } finally { accountWriteBusy = false; input.disabled = false; button.disabled = false; button.textContent = 'Connect desktop'; }
    }

    async function checkAuthSession() {
      try {
        const [account, publication] = await Promise.allSettled([
          apiJson('/api/v1/auth/session', { headers, signal: AbortSignal.timeout(15000) }),
          apiJson('/api/v1/sync/status', { headers, signal: AbortSignal.timeout(15000) })
        ]);
        if (account.status !== 'fulfilled') throw new Error('Account unavailable');
        const res = account.value;
        const sync = publication.status === 'fulfilled' ? publication.value : null;
        const publicationText = sync ? `${sync.message} Last published: ${sync.last_success_at ? new Date(sync.last_success_at).toLocaleString() : 'not yet'}.` : 'Publication status unavailable. Retry Reload view.';
        document.getElementById('auth-publication-status').textContent = publicationText;
        document.getElementById('cloud-sync-status').textContent = publicationText;
        document.getElementById('auth-cloud-status').textContent = res.paired ? 'Connected to your account' : 'Not connected. Local monitoring is ready.';
        document.getElementById('account-entry').hidden = Boolean(res.paired);
        document.getElementById('auth-paired-card').hidden = !res.paired;
        document.getElementById('auth-unpaired-card').hidden = Boolean(res.paired);
        document.getElementById('auth-paired-email').textContent = res.email || 'Connected account';
        return true;
      } catch (_) {
        document.getElementById('auth-cloud-status').textContent = 'Account connection unavailable. Retry Reload view.';
        document.getElementById('auth-publication-status').textContent = 'Local monitoring does not require an account connection.';
        document.getElementById('cloud-sync-status').textContent = 'Account status unavailable. Retry Reload view.';
        document.getElementById('account-entry').hidden = false;
        document.getElementById('auth-paired-card').hidden = true;
        document.getElementById('auth-unpaired-card').hidden = false;
        return false;
      }
    }

    async function unpairSession() {
      if (accountWriteBusy) return;
      if (!await confirmAction('Disconnect this desktop from your account? Recorded local sessions will stay on this device.', 'Disconnect desktop')) return;
      accountWriteBusy = true;
      const button = document.getElementById('auth-disconnect-button'); button.disabled = true; button.textContent = 'Disconnecting...';
      try {
        const res = await fetch('/api/v1/auth/session', {
          method: 'DELETE', signal: AbortSignal.timeout(30000),
          headers
        });
        if (res.ok) {
          await checkAuthSession();
        } else {
          showNotice('Could not disconnect. Reload the account status and retry.');
        }
      } catch (err) {
        showNotice('Connection could not be confirmed. Reload the account status before retrying.');
      } finally { accountWriteBusy = false; button.disabled = false; button.textContent = 'Disconnect desktop'; }
    }

    let homeActiveCount = null;
    const homeGroups = new Map();
    async function loadOngoing() {
      try {
        const response = await apiJson('/api/v1/sessions?page=1&limit=100', { headers, signal: AbortSignal.timeout(15000) });
        const presentation = await import('/console-presentation.js');
        presentation.updateOngoing(document.getElementById('sessions-ongoing'), response.sessions);
        presentation.updateHomeSessions(document.getElementById('home-sessions'), response.sessions);
        homeGroups.clear(); response.sessions.forEach(session => homeGroups.set(session.id, session.home_group));
        homeActiveCount = response.sessions.filter(session => session.home_group === 'needs_you' || session.home_group === 'running').length;
        if (activeView === 'overview') void loadToday();
        return true;
      } catch (_) { document.getElementById('home-sessions').textContent = 'Session status unavailable. Retry Reload view.'; return false; }
    }

    async function loadQuota() {
      try {
        const res = await apiJson('/api/v1/providers/overview', { headers, signal: AbortSignal.timeout(15000) });
        if (!Array.isArray(res)) throw new Error('Invalid quota response');
        // Cards show relative times, so re-render once a minute even when the readings are unchanged.
        const jsonStr = JSON.stringify(res) + Math.floor(Date.now() / 60000);
        if (jsonStr === lastQuotaJson) return true;
        const presentation = await import('/console-presentation.js');
        presentation.updateProviderCards(document.getElementById('quota-table'), res, { cost: true });
        presentation.updateProviderCards(document.getElementById('home-limits'), res, { cost: false });
        presentation.updateProviderSources(document.getElementById('settings-provider-list'), res);
        lastQuotaJson = jsonStr;
        return true;
      } catch (_) {
        lastQuotaJson = '';
        document.getElementById('home-limits').textContent = 'Provider limits unavailable. Retry Reload view.';
        document.getElementById('quota-table').textContent = 'Quota refresh failed. Last observations are stale; retry Reload view.';
        return false;
      }
    }
    async function loadSettingsData() {
      void apiJson('/health',{headers}).then(health=>{document.getElementById('app-build').textContent=`Gnomcode ${health.version || 'version unavailable'}`;}).catch(()=>{document.getElementById('app-build').textContent='Installed version unavailable. Reload when the daemon is connected.';});

      try {
        const [userResponse, statsResponse] = await Promise.all([
          fetch('/api/v1/user', { headers }),
          fetch('/api/v1/stats', { headers }),
          checkAuthSession()
        ]);
        if (!userResponse.ok || !statsResponse.ok) throw new Error('Settings request failed');
        const [user, stats] = await Promise.all([userResponse.json(), statsResponse.json()]);
        if (user) {
          document.getElementById('settings-display-name').value = user.display_name || '';
          document.getElementById('settings-openai-plan').value = user.openai_plan_usd || 20;
          document.getElementById('settings-claude-plan').value = user.claude_plan_usd || 20;
          document.getElementById('settings-google-plan').value = user.google_plan_usd || 0;
        }
        if (stats && [stats.total_turns, stats.total_projects, stats.db_size_bytes].every(Number.isFinite)) {
          document.getElementById('settings-db-stats').textContent =
            `Local Storage: ${stats.total_turns} turns EXACT across ${stats.total_projects} projects EXACT (${(stats.db_size_bytes / 1024 / 1024).toFixed(2)} MB EXACT)`;
        }
        else document.getElementById('settings-db-stats').textContent = 'Storage measurements unavailable · UNKNOWN';
        await loadCustomPricingSettings();
        await loadNotificationSettings();
        await loadRemoteStatus();
        await loadUpdateStatus();
        return true;
      } catch (e) {
        console.error(e);
        document.getElementById('settings-db-stats').textContent = 'Local settings are unavailable. Retry Reload view when the daemon is reachable.';
        return false;
      }
    }

    function renderUpdateStatus(status) {
      const text = document.getElementById('update-status-text');
      const toggle = document.getElementById('update-auto-check');
      const restartBtn = document.getElementById('update-restart-btn');
      if (!text || !toggle || !restartBtn || !status) return;
      toggle.checked = status.auto_update_enabled !== false;
      const version = status.available_version ? `v${status.available_version}` : 'new version';
      if (status.state === 'ready') {
        text.textContent = `Update available: ${version} ready to install on quit or restart.`;
      } else if (status.state === 'downloading' || status.state === 'available') {
        text.textContent = status.state === 'downloading'
          ? `Update available: ${version} downloading in background.`
          : `Update available: ${version} queued for download.`;
      } else if (status.state === 'disabled') {
        text.textContent = 'Automatic update checks off.';
      } else if (status.state === 'error') {
        text.textContent = `Update check failed (${status.last_error || 'offline'}).`;
      } else {
        text.textContent = `Up to date (v${status.current_version || 'unknown'}).`;
      }
      restartBtn.hidden = !(status.state === 'ready' && window.gnomcodeDesktop?.restartToUpdate);
    }

    async function loadUpdateStatus() {
      try {
        const status = await apiJson('/api/v1/update/status', { headers });
        renderUpdateStatus(status);
      } catch {
        const text = document.getElementById('update-status-text');
        const restartBtn = document.getElementById('update-restart-btn');
        if (text) text.textContent = 'Update status unavailable.';
        if (restartBtn) restartBtn.hidden = true;
      }
    }

    async function checkUpdateNow() {
      const btn = document.getElementById('update-check-btn');
      const text = document.getElementById('update-status-text');
      if (btn) btn.disabled = true;
      if (text) text.textContent = 'Checking for updates...';
      try {
        const status = await apiJson('/api/v1/update/check', { method: 'POST', headers });
        renderUpdateStatus(status);
      } catch {
        if (text) text.textContent = 'Update check unavailable right now.';
      } finally {
        if (btn) btn.disabled = false;
      }
    }

    async function toggleAutoUpdate(enabled) {
      try {
        const status = await apiJson('/api/v1/settings/update', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ enabled }),
        });
        renderUpdateStatus(status);
      } catch {
        await loadUpdateStatus();
      }
    }

    async function restartToUpdate() {
      const btn = document.getElementById('update-restart-btn');
      const text = document.getElementById('update-status-text');
      if (!window.gnomcodeDesktop?.restartToUpdate) return;
      if (btn) btn.disabled = true;
      const started = await window.gnomcodeDesktop.restartToUpdate().catch(() => false);
      if (!started) {
        if (btn) btn.disabled = false;
        if (text) text.textContent = 'Could not launch verified update installer. Re-check updates and try again.';
      }
    }

    async function loadCustomPricingSettings() {
      try {
        const res = await apiJson('/api/v1/pricing/custom', { headers });
        const container = document.getElementById('settings-pricing-list');
        if (!res || !res.custom_pricing || res.custom_pricing.length === 0) {
          container.innerHTML = '<span class="sp-tone-muted">No custom rate overrides configured (using default catalog).</span>';
          return;
        }
        let html = '<div class="sp-pricing-rows">';
        for (const cp of res.custom_pricing) {
          html += `
            <div class="sp-pricing-row">
              <span><strong>${escapeHtml(cp.model_pattern)}</strong>: In $${cp.input_per_mtok_usd.toFixed(2)} | Out $${cp.output_per_mtok_usd.toFixed(2)} | Cache $${cp.cache_read_per_mtok_usd.toFixed(2)}</span>
              <button class="btn-control sp-row-action sp-tone-red" data-action="delete-pricing" data-arg="${escapeHtml(cp.model_pattern)}">Remove</button>
            </div>`;
        }
        html += '</div>';
        container.innerHTML = html;
      } catch (e) {
        console.error(e);
      }
    }

    async function addCustomPricingSettings() {
      const pattern = document.getElementById('settings-model-pattern').value.trim();
      const inPrice = parseFloat(document.getElementById('settings-model-in').value) || 0.0;
      const outPrice = parseFloat(document.getElementById('settings-model-out').value) || 0.0;
      const cachePrice = parseFloat(document.getElementById('settings-model-cache').value) || 0.0;
      if (!pattern) return;
      await fetch('/api/v1/pricing/custom', {
        method: 'POST',
        headers: { ...headers, 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model_pattern: pattern,
          input_per_mtok_usd: inPrice,
          output_per_mtok_usd: outPrice,
          cache_read_per_mtok_usd: cachePrice,
          updated_at: new Date().toISOString()
        })
      });
      document.getElementById('settings-model-pattern').value = '';
      document.getElementById('settings-model-in').value = '';
      document.getElementById('settings-model-out').value = '';
      document.getElementById('settings-model-cache').value = '';
      await loadCustomPricingSettings();
      loadReport();
    }

    async function deleteCustomPricing(pattern) {
      await fetch(`/api/v1/pricing/custom?pattern=${encodeURIComponent(pattern)}`, {
        method: 'DELETE',
        headers
      });
      await loadCustomPricingSettings();
      loadReport();
    }

    async function saveSettings() {
      const profile = {
        id: 'local_user',
        username: 'developer',
        display_name: document.getElementById('settings-display-name').value || 'Developer',
        openai_plan_usd: parseFloat(document.getElementById('settings-openai-plan').value) || 0.0,
        claude_plan_usd: parseFloat(document.getElementById('settings-claude-plan').value) || 0.0,
        google_plan_usd: parseFloat(document.getElementById('settings-google-plan').value) || 0.0,
        other_plan_usd: 0.0,
        updated_at: new Date().toISOString()
      };

      await fetch('/api/v1/user', {
        method: 'POST',
        headers: { ...headers, 'Content-Type': 'application/json' },
        body: JSON.stringify(profile)
      });

      const fb = document.getElementById('settings-saved-feedback');
      fb.style.display = 'inline';
      setTimeout(() => fb.style.display = 'none', 3000);
      loadReport();
    }

    async function loadNotificationSettings() {
      try {
        const cfg = await apiJson('/api/v1/settings/notifications', { headers });
        if (!cfg) return;
        document.getElementById('notify-approval-waiting').checked = Boolean(cfg.approval_waiting);
        document.getElementById('notify-turn-finished').checked = Boolean(cfg.turn_finished);
        document.getElementById('notify-turn-minutes').value = Number.isFinite(cfg.turn_finished_min_minutes) ? cfg.turn_finished_min_minutes : 2;
        document.getElementById('notify-limit-low').checked = Boolean(cfg.limit_low);
        document.getElementById('notify-limit-reset').checked = Boolean(cfg.limit_reset);
      } catch (e) {
        console.error(e);
      }
    }

    async function saveNotificationSettings() {
      const minutesRaw = parseInt(document.getElementById('notify-turn-minutes').value, 10);
      const body = {
        approval_waiting: document.getElementById('notify-approval-waiting').checked,
        turn_finished: document.getElementById('notify-turn-finished').checked,
        turn_finished_min_minutes: Number.isFinite(minutesRaw) ? Math.max(0, Math.min(1440, minutesRaw)) : 2,
        limit_low: document.getElementById('notify-limit-low').checked,
        limit_reset: document.getElementById('notify-limit-reset').checked
      };
      try {
        const res = await fetch('/api/v1/settings/notifications', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        });
        if (!res.ok) return;
        const fb = document.getElementById('notify-saved-feedback');
        fb.style.display = 'inline';
        setTimeout(() => fb.style.display = 'none', 3000);
      } catch (e) {
        console.error(e);
      }
    }


    // Tool approvals held by a Claude Code hook (ADR 0032). The daemon owns the rules;
    // this only lists pending requests and sends the person's choice for one exact id.
    let approvalsMarkup = null;
    async function loadApprovals() {
      const region = document.getElementById('approvals');
      if (!region) return;
      let items = [];
      try { items = (await apiJson('/api/v1/approvals', { headers })).items || []; } catch (_) { return; }
      const markup = items.map(item => {
        const agent = item.agent === 'claude-code' ? 'Claude Code' : item.agent;
        return `<article class="sp-approval" data-approval-id="${escapeHtml(item.id)}"><div class="sp-approval-body"><p class="sp-approval-title"><span class="sp-session-dot sp-session-dot--needs_you" aria-hidden="true"></span>${escapeHtml(agent)} wants to run <strong>${escapeHtml(item.tool)}</strong></p>${item.summary ? `<code class="sp-approval-summary">${escapeHtml(item.summary)}</code>` : ''}<p class="sp-approval-meta">If you do not answer, Claude asks in its own window.</p></div><div class="sp-approval-actions"><button type="button" class="btn-control" data-action="approval-decide" data-arg="${escapeHtml(item.id)}:deny" aria-label="Deny ${escapeHtml(item.tool)}">Deny</button><button type="button" class="btn-control" data-action="approval-decide" data-arg="${escapeHtml(item.id)}:allow" aria-label="Approve ${escapeHtml(item.tool)} once">Approve once</button></div></article>`;
      }).join('');
      if (markup === approvalsMarkup) return;
      approvalsMarkup = markup;
      region.innerHTML = markup;
      region.hidden = items.length === 0;
    }
    async function decideApproval(arg) {
      const [id, decision] = String(arg || '').split(':');
      if (!id || !['allow', 'deny'].includes(decision)) return;
      document.querySelectorAll(`[data-approval-id="${CSS.escape(id)}"] button`).forEach(button => { button.disabled = true; });
      try {
        await fetch(`/api/v1/approvals/${encodeURIComponent(id)}/decision`, { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify({ decision }) });
      } catch (_) { /* the list shows the real state */ }
      approvalsMarkup = null;
      await loadApprovals();
    }

    let remoteWasPaired = null;
    let remoteNoticeTimer;
    let remoteDevicesMarkup = '';
    async function loadRemoteStatus() {
      try {
        const result=await apiJson('/api/v1/remote/devices',{headers});
        const connected=result.devices.some(device=>device.connected);
        if(remoteWasPaired!==null && connected!==remoteWasPaired){
          document.getElementById('remote-connection-message').textContent=connected?'Device connected':'Device disconnected';
          const badge=document.getElementById('remote-connection-state');badge.hidden=false;
          clearTimeout(remoteNoticeTimer);remoteNoticeTimer=setTimeout(()=>{badge.hidden=true;},5000);
        }
        remoteWasPaired=connected;
        document.getElementById('remote-status').textContent=result.directory_error?'Device discovery is unavailable. Check your connection.':!result.desktop_id?'Sign in to your account to make this desktop available.':!result.account_access?'Confirm account access to make this desktop available to signed-in devices.':!result.remote_enabled?'Account access is disabled for this desktop.':'This desktop is accessible from any device signed in to your account.';
        const accountAction=result.desktop_id?`<p><button type="button" class="btn-control" data-action="remote-device" data-device-operation="${result.account_access&&result.remote_enabled?'disable_account_access':'enable_account_access'}">${result.account_access&&result.remote_enabled?'Disable account access':'Enable account access'}</button></p>`:'';
        const markup=accountAction+result.devices.map(device=>`<div class="sp-device-row"><div><strong>${escapeHtml(device.name)}</strong><p>${device.connected?'Connected':'Saved connection'}</p></div><button type="button" class="btn-control" style="color: var(--red);" data-action="remove-remote-device" data-device-id="${escapeHtml(device.id)}" data-device-name="${escapeHtml(device.name)}">Remove</button></div>`).join('');
        if(markup!==remoteDevicesMarkup){document.getElementById('remote-link').innerHTML=markup;remoteDevicesMarkup=markup;}
      }catch{document.getElementById('remote-status').textContent='Devices unavailable. Reload to try again.';}
    }
    async function removeRemoteDevice(target){
      const id=target.dataset.deviceId;
      const name=target.dataset.deviceName||'this device';
      if(!id)return;
      if(!await confirmAction(`Remove ${name}? It will be disconnected and cannot reconnect without pairing again.`,'Remove device'))return;
      target.disabled=true;
      try{
        await apiJson('/api/v1/remote/devices',{method:'POST',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify({type:'remove_device',device_id:id})});
        await loadRemoteStatus();
      }catch{document.getElementById('remote-status').textContent='Could not remove device. Refresh Devices and retry.';}
      finally{target.disabled=false;}
    }
    async function manageRemoteDevice(target){
      target.disabled=true;
      try{
        const enable=target.dataset.deviceOperation==='enable_account_access';
        if(enable && !await confirmAccountAccess())return;
        if(!enable && !await confirmAction('Disable access to this desktop for every device signed in to your account?','Disable account access'))return;
        await apiJson('/api/v1/remote/devices',{method:'POST',headers:{...headers,'Content-Type':'application/json'},body:JSON.stringify({type:target.dataset.deviceOperation,remote_access_confirmed:enable})});
        await loadRemoteStatus();
      }catch{document.getElementById('remote-status').textContent='Could not update account access. Refresh Devices and retry.';}
      finally{target.disabled=false;}
    }

    document.addEventListener('click', async event => {
      const copy = event.target.closest('[data-copy]');
      if (!copy) return;
      const label = copy.querySelector('span');
      try { await navigator.clipboard.writeText(copy.dataset.copy); if (label) label.textContent = 'Copied'; }
      catch (_) { if (label) label.textContent = 'Select and copy'; }
      setTimeout(() => { if (label) label.textContent = 'Copy'; }, 1500);
    });

    document.addEventListener('click', event => {
      const target = event.target.closest('[data-action]');
      if (!target) return;
      const action = target.dataset.action;
      if(action==='remote-device'){void manageRemoteDevice(target);return;}
      if(action==='remove-remote-device'){void removeRemoteDevice(target);return;}
      if (action.startsWith('view:')) { event.preventDefault(); switchTab(action.slice(5)); if (event.detail && matchMedia('(max-width:760px)').matches) document.querySelector('.view-content.active')?.focus({preventScroll:true}); return; }
      if (action.startsWith('export:')) { exportData(action.slice(7)); return; }
    async function handlePermissionAction(sessionId, action, toolCallId) {
      try {
        await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/permissions`, {
          method: 'POST',
          headers,
          body: JSON.stringify({ action, tool_call_id: toolCallId })
        });
        await loadChat(sessionId);
      } catch (e) {
        console.error('Failed to record permission action', e);
      }
    }

    const actions = { 'approval-decide': decideApproval, 'onboarding-start': startOnboarding, 'onboarding-skip': () => finishOnboarding(false), 'onboarding-back': () => moveOnboarding(-1), 'onboarding-next': () => moveOnboarding(1), 'dismiss-tour-tip': dismissTourTip, 'dismiss-connection': () => {document.getElementById('remote-connection-state').hidden = true;}, 'dismiss-notice': () => {document.getElementById('app-notice').hidden = true;}, 'toggle-navigation': toggleNavigation, 'toggle-chat-list': toggleChatList, theme: cycleTheme, reload: refreshCurrentView, resync: resyncLocal,
      'automation-chat': () => navigateView('sessions'), 'delete-chat': deleteChat, 'load-chat': id => { void loadChat(id); if (matchMedia('(max-width: 760px)').matches) applyChatList(true); },
      'toggle-artifacts': () => toggleArtifacts(), 'refresh-artifacts': () => { void loadArtifacts(); }, 'close-artifacts': () => toggleArtifacts(false),
      'pick-attachment': () => { document.getElementById('chat-attach-input')?.click(); },
      'earlier-chat': () => loadChat(activeChatSession, true),
      'workspace-terminal': openWorkspaceTerminal, 'hide-workspace-terminal': () => {document.getElementById('workspace-terminal-pane').hidden = true;}, 'toggle-stream': toggleStream,
      'close-chat': () => showLauncher(false), 'new-session': () => showLauncher(), 'home-new-session': () => { navigateView('sessions'); showLauncher(); }, 'open-home-session': id => { navigateView('sessions'); void loadChat(id); },
      'schedule-message': scheduleDraft, 'cancel-schedule': cancelSchedule, 'save-routing': saveRouting, 'reconnect-reset': () => reconnectReset(true), 'cancel-reset': () => reconnectReset(false),
      'switch-model': switchChatModel, 'focus-chat': focusChat, 'resume-chat': resumeChat,
      'choose-workspace': chooseWorkspace,
        'copy-workspace': copySessionWorkspace, previous: prevPage, next: nextPage,
        'save-settings': saveSettings, 'save-notifications': saveNotificationSettings, 'add-pricing': addCustomPricingSettings,
        'check-update': checkUpdateNow, 'restart-to-update': restartToUpdate,
        unpair: unpairSession, 'activate-license': activateLicenseKey, 'remove-license': removeLicenseKey,
        disarm: disarmResume, 'delete-rule': deleteRule, 'execute-relay': executeRelay,
        'dismiss-relay': dismissRelay, 'delete-pricing': deleteCustomPricing,
        'quick-action': sendQuickAction, 'manage-quick-actions': openQuickActionsDialog,
        'remove-quick-action': removeQuickAction, 'reset-quick-actions': resetQuickActions,
        'close-quick-actions': () => document.getElementById('quick-actions-dialog')?.close(),
        'toggle-changes': toggleChangesPanel, 'refresh-changes': loadSessionChanges, 'close-changes': closeChangesPanel,
        'toggle-files': toggleFilesPanel, 'close-files': closeFilesPanel,
        'open-repo-dir': p => { void loadSessionFileListing(p); }, 'open-repo-file': p => { void readSessionFile(p); },
 };
      if (action === 'toggle-pair-token') { const input = document.getElementById('auth-token-input'); const visible = input.type === 'password'; input.type = visible ? 'text' : 'password'; target.setAttribute('aria-pressed', String(visible)); target.textContent = visible ? 'Hide token' : 'Show token'; return; }
      if (action === 'replace-license') { document.getElementById('license-entry').hidden = false; target.hidden = true; return; }
      if (Object.hasOwn(actions, action)) actions[action](target.dataset.arg);
    });
    document.addEventListener('change', event => {
      if (event.target.id === 'chat-attach-input') {
        const file = event.target.files?.[0];
        event.target.value = '';
        if (file) void handleLocalAttachmentPick(file);
        return;
      }
      if (event.target.id === 'chat-model-select') { void switchChatModel(); return; }
      if (event.target.id === 'update-auto-check') { void toggleAutoUpdate(event.target.checked); return; }
      if (event.target.id === 'launch-agent-select') {
        updateAgentInstallHint();
        return;
      }
      const target = event.target.closest('[data-change]');
      if (!target) return;
      if (target.dataset.change === 'timeline') setTimeline(target.value);
      else if (target.dataset.change === 'project-filter') { currentPage = 1; lastSessionsJson = ''; loadSessions(); }
      else if (target.dataset.change === 'filter') onFilterChange();
      else if (target.dataset.change === 'relay-config') saveRelayConfig();
    });
    document.getElementById('ide-session-search').addEventListener('input', () => {currentPage=1;void loadSessions();});
    document.getElementById('conversation-source').addEventListener('change', () => {currentPage=1;void loadSessions();});
    document.getElementById('include-subagents').addEventListener('change', () => {currentPage=1;void loadSessions();});
    document.addEventListener('submit', event => {
      const form = event.target.closest('[data-submit]');
      if (!form) return;
      const submit = form.dataset.submit;
      if (submit === 'signin') { event.preventDefault(); signInAccount(); }
      else if (submit === 'pair') { event.preventDefault(); pairTokenDirectly(); }
      else if (submit === 'launch-agent') { event.preventDefault(); launchNewAgent(event); }
      else if (submit === 'send-prompt') { event.preventDefault(); sendPrompt(event); }
      else if (submit === 'add-quick-action') { void addQuickActionFromDialog(event); }
      else if (submit === 'open-repo-file') { event.preventDefault(); void readSessionFile(document.getElementById('chat-file-path-input')?.value); }
    });

    normalizeHash();
    const hashView = resolveView(window.location.hash.slice(1));
    let initialView = hashView;
    if (!initialView) {
      try { initialView = resolveView(sessionStorage.getItem('gnomcode-active-view') || ''); } catch (_) {}
    }
    if (VIEWS.includes(initialView)) setView(initialView, Boolean(hashView));
    else setView('overview', false);
    openExplorerWorkspace();
    void checkOnboardingOffer();
    void checkAuthSession();
    pollLocalData();
    if (initialView === 'sessions') loadInstalledAgents();
    void loadApprovals();
    setInterval(() => {
      pollLocalData();
      void loadApprovals();
      loadRemoteStatus();
      void refreshAutomation();
      if (activeView === 'sessions' && !installedAgents.length) loadInstalledAgents();
      const chatPanel = document.getElementById('session-chat-panel');
      if (chatPanel && !chatPanel.hidden && activeChatSession) {
        const terminal = document.getElementById('chat-terminal-view');
        if (terminal && terminal.hidden) {
          loadChat(activeChatSession, false, true);
        }
      }
    }, 2000);
