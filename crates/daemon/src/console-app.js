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
        if (next === 'system') localStorage.removeItem('spawnt-theme');
        else localStorage.setItem('spawnt-theme', next);
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
    const updateNavOrientation = () => consoleNavigation.setAttribute('aria-orientation', 'vertical');
    updateNavOrientation(); window.addEventListener('resize', updateNavOrientation);
    let currentTimeline = 'all';
    let currentPage = 1;
    const pageLimit = 10;
    const token = new URLSearchParams(window.location.search).get('token') || '';
    const headers = token ? { 'Authorization': `Bearer ${token}` } : {};
    const summaryState = { activeCount: null, report: null, quotaCount: null };
    async function refreshSummary() {
      const presentation = await import('/console-presentation.js');
      presentation.updateSummary(document.getElementById('overview-summary'), summaryState);
    }
    let refreshBusy = false;
    let lastSuccessfulUpdate = null;
    let activeView = 'overview';
    let projectsLoaded = false;
    let lastReportJson = '';
    let lastSessionsJson = '';
    let lastQuotaJson = '';

    function consoleViewUrl(view) { const url = new URL(window.location.href); url.hash = view === 'overview' ? '' : view; return url.pathname + url.search + url.hash; }
    document.querySelector('.app-brand').href = consoleViewUrl('overview');

    function setView(view, updateUrl = true) {
      document.getElementById('console-title').textContent = view === 'overview' ? 'Your agent fleet' : view === 'auth' ? 'Account' : view[0].toUpperCase() + view.slice(1);
      activeView = view;
      document.body.dataset.consoleView = view;
      document.getElementById('chat-list-toggle').hidden = view !== 'sessions';
      const titles = { overview: 'Dashboard', sessions: 'Workspace', usage: 'Usage', limits: 'Limits', projects: 'Projects', automations: 'Automations', settings: 'Profile', auth: 'Account' };
      document.getElementById('console-title').textContent = titles[view] || 'Your agent fleet';
      if (['overview', 'sessions', 'usage', 'limits'].includes(view)) {
        document.getElementById('view-console').setAttribute('aria-labelledby', `tab-btn-${view}`);
      }
      document.querySelectorAll('.tab-btn').forEach(button => {
        const selected = button.dataset.view === view;
        button.classList.toggle('active', selected);
        button.setAttribute('aria-selected', String(selected));
        button.tabIndex = selected ? 0 : -1;
      });
      document.querySelectorAll('[data-local-view]').forEach(section => { section.hidden = !section.dataset.localView.split(',').includes(view); });
      document.getElementById('session-chat-panel').hidden = view !== 'sessions' || !activeChatSession;
      document.querySelectorAll('.view-content').forEach(section => section.classList.toggle('active', section.id === `view-${view}` || (section.id === 'view-console' && ['overview', 'sessions', 'usage', 'limits'].includes(view))));
      if (updateUrl && window.location.hash.slice(1) !== view) history.pushState(null, '', consoleViewUrl(view));
    }

    document.querySelector('.nav-tabs').addEventListener('keydown', event => {
      const tabs = [...document.querySelectorAll('.nav-tabs [role="tab"]')];
      const current = tabs.indexOf(document.activeElement);
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
    window.addEventListener('hashchange', () => {
      const view = window.location.hash.slice(1) || 'overview';
      if (['overview', 'sessions', 'usage', 'limits', 'projects', 'automations', 'settings', 'auth'].includes(view)) { setView(view, false); refreshCurrentView(); }
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
        if (['overview', 'sessions', 'usage', 'limits'].includes(activeView)) {
          const results = await Promise.all([loadReport(), loadSessions(), loadQuota(), loadOngoing()]);
          if (results.some(result => !result)) throw new Error('One or more local data sources failed');
        } else if (activeView === 'settings') {
          if (!await loadSettingsData()) throw new Error('Settings could not be loaded');
        } else if (activeView === 'projects') {
          if (!await loadProjects()) throw new Error('Projects could not be loaded');
        } else if (activeView === 'automations') {
          if (!await loadProData()) throw new Error('Automation data could not be loaded');
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
      if (!['overview', 'sessions', 'usage', 'limits'].includes(activeView)) return;
      pollBusy = true;
      try {
      const results = await Promise.all([loadReport(), loadSessions(), loadQuota(), loadOngoing()]);
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
      if (!response.ok) throw new Error(response.status === 401 ? 'Local authorization is missing or expired. Close this window and reopen spawnt from its tray icon or shortcut.' : `spawnt request failed (${response.status}). Retry Reload view or restart spawnt if this persists.`);
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
          showNotice("Exporting analytics requires a Pro license. Activate Pro in Automations.");
          return;
        }
        if (!res.ok) {
          showNotice("Failed to export data.");
          return;
        }
        const blob = await res.blob();
        const downloadUrl = window.URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = downloadUrl;
        a.download = `spawnt-report-${currentTimeline}.${fmt}`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        window.URL.revokeObjectURL(downloadUrl);
      } catch (err) {
        showNotice("Export failed: " + err);
      }
    }

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
            qTable.innerHTML = '<tr><td colspan="8" style="text-align: center; color: var(--text-muted);">No armed sessions in queue.</td></tr>';
          } else {
            qTable.innerHTML = queue.map(e => `
              <tr>
                <td style="font-family: var(--font-mono); font-size: 11px;">${escapeHtml(e.session_id)}</td>
                <td><span class="badge">${escapeHtml(e.agent)}</span></td>
                <td style="color: var(--text-muted); font-size: 11px;">${escapeHtml(e.cwd)}</td>
                <td>${escapeHtml(e.resume_prompt)}</td>
                <td>${e.max_runs} <span class="trust-mark">EXACT</span></td>
                <td>${e.runs_completed} <span class="trust-mark">EXACT</span></td>
                <td style="color: var(--text-muted); font-size: 11px;">${formatRelativeTime(e.eligible_after)}</td>
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
            rTable.innerHTML = '<tr><td colspan="6" style="text-align: center; color: var(--text-muted);">No custom guardrail rules configured.</td></tr>';
          } else {
            rTable.innerHTML = rules.map(r => `
              <tr>
                <td style="font-family: var(--font-mono); font-size: 11px; font-weight: 600;">${escapeHtml(r.id)}</td>
                <td>${escapeHtml(r.agent || 'All Agents')}</td>
                <td>${r.max_spend_usd ? '$' + r.max_spend_usd.toFixed(2) + ' EXACT' : 'None'}</td>
                <td>${r.max_turns ? r.max_turns + ' EXACT' : 'None'}</td>
                <td><span class="badge" style="color: ${r.action === 'HardStop' ? 'var(--red)' : 'var(--accent)'};">${escapeHtml(r.action)}</span></td>
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
            aTable.innerHTML = '<tr><td colspan="4" style="text-align: center; color: var(--text-muted);">No guardrail alerts recorded.</td></tr>';
          } else {
            aTable.innerHTML = alerts.map(a => `
              <tr>
                <td style="font-family: var(--font-mono); font-size: 11px;">${escapeHtml(a.session_id)}</td>
                <td style="font-weight: 600;">${escapeHtml(a.rule_id)}</td>
                <td><span class="badge" style="color: ${a.action === 'hard_stop' ? 'var(--red)' : 'var(--accent)'};">${escapeHtml(a.action)}</span></td>
                <td style="color: var(--text-muted); font-size: 11px;">${formatRelativeTime(a.created_at)}</td>
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
            table.innerHTML = '<tr><td colspan="7" style="text-align: center; color: var(--text-muted);">No active context relays recorded.</td></tr>';
          } else {
            table.innerHTML = data.relays.map(r => {
              const statusColor = r.status === 'executed' ? 'var(--green)' : (r.status === 'pending' ? 'var(--yellow)' : 'var(--text-muted)');
              const actionBtn = r.status === 'pending'
                ? `<button class="btn-primary" style="padding: 4px 10px; font-size: 11px; margin-right: 6px;" data-action="execute-relay" data-arg="${escapeHtml(r.id)}">Handoff & Rotate</button><button class="btn-control" style="padding: 4px 8px; font-size: 11px;" data-action="dismiss-relay" data-arg="${escapeHtml(r.id)}">Dismiss</button>`
                : `<span style="font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);">${escapeHtml(r.status)}</span>`;
              return `<tr>
                <td style="font-family: var(--font-mono); font-size: 11px;">${escapeHtml(r.session_id)}</td>
                <td style="font-weight: 600;">${escapeHtml(r.project_name)}</td>
                <td>${escapeHtml(r.source_agent)} &rarr; ${escapeHtml(r.target_agent)}</td>
                <td style="font-weight: 600; color: ${r.fill_pct >= 90 ? 'var(--red)' : 'var(--yellow)'};">${r.fill_pct}% <span class="trust-mark">EST</span></td>
                <td><span class="badge" style="color: ${statusColor}; border-color: ${statusColor};">${r.status.toUpperCase()}</span></td>
                <td style="color: var(--text-muted); font-size: 11px;">${formatRelativeTime(r.created_at)}</td>
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
          showNotice('Context Relay configuration requires an active spawnt Pro license.');
          return;
        }
        if (res.ok) {
          loadRelayData();
        }
      } catch (err) {
        showNotice('Failed to save config: ' + err.message);
      }
    }

    async function executeRelay(id) {
      if (!await confirmAction('Rotate session now? spawnt will create .spawnt/HANDOFF.md and spawn a clean session.', 'Rotate session')) return;
      try {
        const res = await fetch('/api/v1/pro/relay/execute', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ id })
        });
        if (res.status === 402) {
          showNotice('Context Relay execution requires an active spawnt Pro license.');
          return;
        }
        if (res.ok) {
          showNotice('Context Relay handoff executed successfully! A fresh session was launched with a repository handoff.');
          loadRelayData();
        } else {
          showNotice('Failed to execute relay handoff.');
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
      if (activeView === 'projects') loadProjects();
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
        summaryState.report = tl; await refreshSummary();
        const presentation = await import("/console-presentation.js");
        presentation.updateActivity(document.getElementById("daily-activity"), tl);


        // Date range header
        let dateStr = tl.range_label;
        if (tl.earliest_at && tl.latest_at) {
          const e = new Date(tl.earliest_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
          const l = new Date(tl.latest_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
          dateStr = e === l ? `Showing: ${tl.range_label} (${e})` : `Showing: ${tl.range_label} (${e} — ${l})`;
        }
        document.getElementById('timeline-label').textContent = dateStr;

// Render ROI cards
        const activeRoi = (res.roi_by_provider || []).filter(r => r.spend_usd > 0);
        const roiContainer = document.getElementById('roi-cards-container');
        roiContainer.innerHTML = activeRoi.map(r => `
          <div style="background: var(--bg); border: 1px solid var(--border); border-radius: 0; padding: 12px; font-family: var(--font-mono);">
            <div style="font-size: 11px; color: var(--text-muted); text-transform: uppercase;">${escapeHtml(r.provider)}</div>
            <div style="font-size: 18px; font-weight: 600; margin: 4px 0;">${r.multiplier.toFixed(1)}x Return <span class="trust-mark">EST</span></div>
            <div style="font-size: 11px; color: ${r.net_savings_usd >= 0 ? 'var(--green)' : 'var(--red)'};">
              ${r.net_savings_usd >= 0 ? '+' : ''}$${r.net_savings_usd.toFixed(2)} <span class="trust-mark">EST</span>
            </div>
            <div style="font-size: 10px; color: var(--text-muted); margin-top: 4px;">Paid: $${r.plan_usd.toFixed(2)}/${r.period_label || 'period'} <span class="trust-mark">EXACT</span> | Value: $${r.spend_usd.toFixed(2)} <span class="trust-mark">EST</span></div>
          </div>
        `).join('') || '<div style="color: var(--text-muted); font-size: 12px;">No active subscription usage for this timeframe.</div>';

        // Render the shared provider breakdown.
        presentation.updateProviders(document.getElementById('providers-container'), tl);
        lastReportJson = jsonStr;
        return true;
      } catch (e) {
        console.error('Failed to load report', e);
        lastReportJson = '';
        showConnectionError(e);
        document.getElementById('daily-activity').textContent = 'Daily activity unavailable. ' + e.message;
        summaryState.report = null; await refreshSummary();
        document.getElementById('providers-container').innerHTML = '<p class="local-view-message">Usage data is unavailable. Retry Reload view when the local daemon is reachable.</p>';
        return false;
      }
    }

    async function loadProjects() {
      const container = document.getElementById('project-rollups');
      const range = currentTimeline;
      try {
        let url = `/api/v1/projects?range=${encodeURIComponent(range)}`;
        if (range === 'today') { const start = new Date(); start.setHours(0, 0, 0, 0); url += `&from=${encodeURIComponent(start.toISOString())}`; }
        const projects = await apiJson(url, { headers, signal: AbortSignal.timeout(15000) });
        if (range !== currentTimeline) return true;
        const presentation = await import('/console-presentation.js');
        container.innerHTML = presentation.renderProjects(projects);
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
    let hostedSessions = [];
    let chatRequest = 0;
    const chatDrafts = new Map();
    const pendingPrompts = new Set();
    const optimisticPrompts = new Map();
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
      const known = new Set(ideSessions.map(session => session.id));
      const hosted = hostedSessions.filter(session => !known.has(session.session_id)).map(session => ({ ...session, id: session.session_id, title: session.title || 'New conversation', last_event_at: session.started_at, state: 'hosted' }));
      document.getElementById('ide-session-list').innerHTML = presentation.renderSessionList([...hosted, ...ideSessions], activeChatSession, document.getElementById('ide-session-search').value);
    }

    function showLauncher() {
      selectChatSession(''); chatEvents = []; chatRequest++;
      if (chatWs) { chatWs.close(); chatWs = null; }
      document.getElementById('session-chat-panel').hidden = true;
      document.getElementById('ide-launcher').hidden = false;
      document.getElementById('launch-status').textContent = '';
      renderIdeSessions(); loadInstalledAgents();
    }

    function showCurrentChatModel() {
      const select = document.getElementById('chat-model-select');
      const session = ideSessions.find(session => session.id === activeChatSession);
      const hosted = hostedSessions.find(session => session.session_id === activeChatSession);
      const model = session?.model || hosted?.model || '';
      const observed = Boolean(session?.model || hosted?.model_observed);
      const status = document.getElementById('chat-model-status');
      const message = (observed ? '' : model ? `Requested model: ${model} · awaiting agent confirmation` : 'CLI default · waiting for the agent to report its model') + (select.disabled ? (observed ? 'Model switching unavailable' : ' · Model switching unavailable') : '');
      if (status.textContent !== message) status.textContent = message;
      if (select.dataset.sessionId !== activeChatSession || select.dataset.currentModel !== model) {
        select.querySelectorAll('[data-current-only]').forEach(option => option.remove());
        if (model && ![...select.options].some(option => option.value === model)) {
          const option = document.createElement('option'); option.value = model; option.textContent = `${model} (current)`; option.disabled = true; option.dataset.currentOnly = ''; select.append(option);
        }
        select.value = model;
        select.dataset.sessionId = activeChatSession;
        select.dataset.currentModel = model;
      }
      document.getElementById('chat-switch-btn').disabled = select.disabled || !select.value || select.selectedOptions[0]?.disabled || select.value === select.dataset.currentModel;
    }

    async function loadChatModels() {
      const select = document.getElementById('chat-model-select');
      const button = document.getElementById('chat-switch-btn');
      const sessionId = activeChatSession;
      try {
        const [models, installed] = await Promise.all([apiJson('/api/v1/models', { headers }),apiJson('/api/v1/agents/installed', { headers })]);
        if (sessionId !== activeChatSession) return;
        select.innerHTML = '<option value="">Agent default</option>';
        const available = models.models || models;
        for (const agent of (installed.agents || []).filter(agent => agent.installed && agent.id !== 'terminal')) {
          const group = document.createElement('optgroup'); group.label = agent.name;
          const option = document.createElement('option'); option.value = `default:${agent.id}`; option.dataset.agent = agent.id; option.dataset.model = '@default'; option.textContent = `${agent.name} · default model`; group.append(option); select.append(group);
        }
        for (const model of available) {
          const option = document.createElement('option'); option.value = model.model;
          option.dataset.agent = model.agent; option.textContent = model.display_name || model.model; select.append(option);
        }
        select.disabled = select.options.length <= 1; button.disabled = select.disabled;
        delete select.dataset.sessionId;
        showCurrentChatModel();
      } catch {
        if (sessionId !== activeChatSession) return;
        select.innerHTML = '<option value="">Model catalog unavailable</option>'; select.disabled = true; button.disabled = true;
        delete select.dataset.sessionId;
        showCurrentChatModel();
      }
    }

    async function switchChatModel() {
      const select = document.getElementById('chat-model-select');
      if (!activeChatSession || !select.value) return;
      const sourceSession = activeChatSession;
      const previousModel = select.dataset.currentModel || 'CLI default';
      const targetModel = select.selectedOptions[0].dataset.model || select.value;
      const targetAgent = select.selectedOptions[0].dataset.agent;
      const button = document.getElementById('chat-switch-btn'); button.disabled = true;
      try {
        const result = await apiJson(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/switch`, { method:'POST', headers:{ ...headers, 'Content-Type':'application/json' }, body:JSON.stringify({ target_model:targetModel, agent:targetAgent }) });
        handoffNotices.set(result.session_id, [{event_id:`handoff:${result.session_id}`,role:'system',status:'model_changed',created_at:new Date().toISOString(),content:`Agent/model change requested: ${ideSessions.find(session=>session.id === sourceSession)?.agent || 'Agent'} / ${previousModel} → ${targetAgent} / ${targetModel === '@default' ? 'agent default' : targetModel}. Waiting for confirmation.`}]);
        await loadSessions();
        if (activeChatSession === sourceSession) {
          await loadChat(result.session_id);
          document.getElementById('chat-delivery-status').textContent = '';
          void renderCurrentChat();
        }
      } catch { if (activeChatSession === sourceSession) document.getElementById('chat-delivery-status').textContent = 'Model handoff failed. Check the target CLI installation and login, then retry.'; }
      finally { showCurrentChatModel(); }
    }

    async function focusChat() {
      if (!activeChatSession) return;
      try {
        const result = await apiJson(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/focus`, { method:'POST', headers });
        document.getElementById('session-action-status').textContent = result.message;
        const copy = document.getElementById('session-action-copy'); copy.hidden = result.focused || !result.repo_path; copy.dataset.path = result.repo_path || '';
      } catch { document.getElementById('session-action-status').textContent = 'Window unavailable. Use the terminal stream for a hosted session.'; }
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

    document.getElementById('launch-workspace-browse').hidden = !window.spawntDesktop?.chooseWorkspace;
    async function chooseWorkspace() {
      try {
        const folder = await window.spawntDesktop?.chooseWorkspace();
        if (folder) document.getElementById('launch-repo-input').value = folder;
      } catch { document.getElementById('launch-status').textContent = 'Folder picker unavailable. Enter the workspace path and retry.'; }
    }

    function resizePrompt() {
      const input = document.getElementById('chat-prompt-text');
      input.style.height = 'auto';
      input.style.height = `${Math.min(160, Math.max(36, input.scrollHeight))}px`;
    }
    document.getElementById('chat-prompt-text').addEventListener('input', resizePrompt);

    async function sendPrompt(event) {
      if (event) event.preventDefault();
      const input = document.getElementById('chat-prompt-text');
      const text = input?.value.trim();
      if (!text || !activeChatSession || document.getElementById('chat-send-btn').disabled) return;
      const status = document.getElementById('chat-delivery-status');
      const sessionId = activeChatSession;
      const optimistic = {id:crypto.randomUUID(),text,created_at:new Date().toISOString(),baseline:chatEvents.map(event => event.event_id),delivered:false};
      optimisticPrompts.set(sessionId, [...(optimisticPrompts.get(sessionId) || []),optimistic]);
      void renderCurrentChat();
      pendingPrompts.add(sessionId);
      const button = document.getElementById('chat-send-btn'); button.disabled = true;
      if (status) status.textContent = '';
      try {
        const res = await fetch(`/api/v1/sessions/${encodeURIComponent(activeChatSession)}/prompt`, {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ text })
        });
        if (!res.ok) {
          const err = await res.json().catch(() => ({}));
          throw new Error(err.message || (res.status === 404 ? 'This session is not controlled by Spawnt. Start an agent with New Session to send messages here.' : 'The terminal did not accept the prompt. Check Terminal stream and retry.'));
        }
        if (!(optimisticPrompts.get(sessionId) || []).includes(optimistic)) return;
        optimistic.delivered = true;
        if (sessionId === activeChatSession) void renderCurrentChat();
        if (chatDrafts.get(sessionId)?.trim() === text) chatDrafts.delete(sessionId);
        if (sessionId === activeChatSession) {
          if (input.value.trim() === text) { input.value = ''; resizePrompt(); }
          if (status) status.textContent = '';
          input.focus();
        }
        if (sessionId === activeChatSession) setTimeout(() => loadChat(sessionId, false, true), 1000);
      } catch (err) {
        optimisticPrompts.set(sessionId, (optimisticPrompts.get(sessionId) || []).filter(item => item.id !== optimistic.id));
        if (sessionId === activeChatSession) void renderCurrentChat();
        if (sessionId === activeChatSession && status) status.textContent = `Prompt delivery failed: ${err.message}`;
      } finally {
        pendingPrompts.delete(sessionId);
        button.disabled = pendingPrompts.has(activeChatSession);
      }
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
        if (btn) btn.textContent = 'Terminal stream';
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
      if (chatWs || !activeChatSession) return;
      const terminal = document.getElementById('chat-terminal-view');
      if (!terminal) return;
      terminal.textContent = 'Connecting to PTY terminal stream...\n';
      const url = new URL(window.location.href);
      const token = url.searchParams.get('token') || '';
      const wsProto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${wsProto}//${location.host}/api/v1/sessions/${encodeURIComponent(activeChatSession)}/pty?token=${encodeURIComponent(token)}`;
      try {
        chatWs = new WebSocket(wsUrl);
        chatWs.binaryType = 'arraybuffer';
        chatWs.onmessage = (e) => {
          const text = typeof e.data === 'string' ? e.data : new TextDecoder().decode(e.data);
          terminal.textContent = (terminal.textContent + text.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, '')).slice(-200000);
          terminal.scrollTop = terminal.scrollHeight;
        };
        chatWs.onerror = () => { terminal.textContent += '\n[Terminal stream disconnected or not hosted by Spawnt]'; };
        chatWs.onclose = () => { chatWs = null; };
      } catch (e) {
        terminal.textContent += '\n[Failed to open terminal stream]';
      }
    }

    async function loadChat(sessionId, earlier = false, silent = false) {
      if (!sessionId) return;
      if (silent && activeChatSession !== sessionId) return;
      if (!silent && !earlier) {
        if (activeChatSession !== sessionId) selectChatSession(sessionId);
        chatEvents = [];
        if (chatWs) { chatWs.close(); chatWs = null; }
        document.getElementById('chat-terminal-view').hidden = true;
        document.getElementById('chat-stream').hidden = false;
        document.getElementById('chat-toggle-stream').textContent = 'Terminal stream';
      }
      const request = ++chatRequest;
      document.getElementById('ide-launcher').hidden = true;
      const session = ideSessions.find(session => session.id === sessionId);
      const hosted = hostedSessions.find(session => session.session_id === sessionId);
      document.getElementById('session-chat-heading').textContent = session?.title || hosted?.title || 'New conversation';
      document.getElementById('chat-session-label').textContent = `${session?.agent || hosted?.agent || 'Agent'} · ${session?.state || 'hosted'}`;
      renderIdeSessions();
      showCurrentChatModel();
      if (!silent) loadChatModels();
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
        if (!silent && status) status.textContent = 'Could not load conversation history.';
        panel.hidden = activeView !== 'sessions';
        if (!silent) requestAnimationFrame(resizePrompt);
      }
    }

    async function deleteChat() {
      if (!activeChatSession || !await confirmAction('Permanently delete local chat history for this session? This action cannot be undone.', 'Delete history')) return;
      const sessionId = activeChatSession;
      try {
        await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat`, { method: 'DELETE', headers });
        optimisticPrompts.delete(sessionId);
        handoffNotices.delete(sessionId);
        if (activeChatSession === sessionId) await loadChat(sessionId);
      } catch { document.getElementById('chat-status').textContent = 'Could not delete local chat history.'; }
    }

    if (matchMedia('(max-width: 760px)').matches) {
      document.body.classList.add('chat-list-collapsed');
      document.getElementById('chat-list-toggle').setAttribute('aria-expanded', 'false');
    }
    function toggleNavigation() {
      const expanded = document.body.classList.toggle('navigation-expanded');
      document.getElementById('navigation-toggle').setAttribute('aria-expanded', String(expanded));
    }
    function toggleChatList() {
      const collapsed = document.body.classList.toggle('chat-list-collapsed');
      document.getElementById('chat-list-toggle').setAttribute('aria-expanded', String(!collapsed));
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
        const project = document.getElementById('project-filter').value;
        const agent = document.getElementById('agent-filter').value;
        const status = document.getElementById('status-filter').value;
        const res = await apiJson(`/api/v1/sessions?page=${currentPage}&limit=${pageLimit}&agent=${agent}&status=${status}&project=${encodeURIComponent(project)}`, { headers, signal: AbortSignal.timeout(15000) });
        if (!res) return false;
        if (project !== document.getElementById('project-filter').value || requestedPage !== currentPage || agent !== document.getElementById('agent-filter').value || status !== document.getElementById('status-filter').value) return true;

        const jsonStr = JSON.stringify(res);
        try { hostedSessions = (await apiJson('/api/v1/sessions/hosted', { headers })).sessions || []; } catch { hostedSessions = []; }
        if (jsonStr === lastSessionsJson) { await renderIdeSessions(); return true; }

        summaryState.activeCount = res.ongoing_count ?? res.active_count ?? null; await refreshSummary();
        document.getElementById('pagination-info').textContent = res.total_pages > 1 ? `${res.page} / ${res.total_pages}` : `${res.total_count} conversation${res.total_count === 1 ? '' : 's'}`;
        document.getElementById('pagination-controls').hidden = res.total_pages <= 1;
        document.getElementById('prev-btn').disabled = res.page <= 1;
        document.getElementById('next-btn').disabled = res.page >= res.total_pages;

        const presentation = await import('/console-presentation.js');
        presentation.updateSessions(document.getElementById('sessions-table'), res.sessions || [], { focus: true });
        ideSessions = res.sessions || [];
        await renderIdeSessions();
        lastSessionsJson = jsonStr;
        return true;
      } catch (e) {
        console.error('Failed to load sessions', e);
        lastSessionsJson = '';
        showConnectionError(e);
        summaryState.activeCount = null; await refreshSummary();
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

    let accountWriteBusy = false;
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

        const res = await fetch('/api/v1/auth/session', {
          method: 'POST', signal: AbortSignal.timeout(30000),
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify(sessionData)
        });

        if (res.ok) {
          status.style.color = 'var(--green)';
          status.textContent = 'Desktop connected. Waiting for its next metadata update.';
          input.value = '';
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
        document.getElementById('auth-paired-card').hidden = !res.paired;
        document.getElementById('auth-unpaired-card').hidden = Boolean(res.paired);
        document.getElementById('auth-paired-email').textContent = res.email || 'Connected account';
        return true;
      } catch (_) {
        document.getElementById('auth-cloud-status').textContent = 'Account connection unavailable. Retry Reload view.';
        document.getElementById('auth-publication-status').textContent = 'Local monitoring does not require an account connection.';
        document.getElementById('cloud-sync-status').textContent = 'Account status unavailable. Retry Reload view.';
        document.getElementById('auth-paired-card').hidden = true;
        document.getElementById('auth-unpaired-card').hidden = true;
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

    async function loadOngoing() {
      try {
        const response = await apiJson('/api/v1/sessions?status=ongoing&page=1&limit=100', { headers, signal: AbortSignal.timeout(15000) });
        const presentation = await import('/console-presentation.js');
        presentation.updateOngoing(document.getElementById('ongoing-sessions'), response.sessions);
        presentation.updateOngoing(document.getElementById('sessions-ongoing'), response.sessions);
        return true;
      } catch (_) { document.getElementById('ongoing-sessions').textContent = 'Ongoing session status unavailable. Retry Reload view.'; return false; }
    }

    async function loadQuota() {
      try {
        const res = await apiJson('/api/v1/quota', { headers, signal: AbortSignal.timeout(15000) });
        if (!Array.isArray(res)) throw new Error('Invalid quota response');
        summaryState.quotaCount = res.length; await refreshSummary();
        const jsonStr = JSON.stringify(res);
        if (jsonStr === lastQuotaJson) return true;
        const presentation = await import('/console-presentation.js');
        presentation.updateQuota(document.getElementById('quota-table'), res, new Date().toISOString());
        presentation.updateQuota(document.getElementById('overview-limits'), res, new Date().toISOString());
        lastQuotaJson = jsonStr;
        return true;
      } catch (_) {
        lastQuotaJson = '';
        document.getElementById('overview-limits').textContent = 'Provider limits unavailable. Retry Reload view.';
        summaryState.quotaCount = null; await refreshSummary();
        document.getElementById('quota-table').textContent = 'Quota refresh failed. Last observations are stale; retry Reload view.';
        return false;
      }
    }
    async function loadSettingsData() {
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
        await loadRemoteStatus();
        return true;
      } catch (e) {
        console.error(e);
        document.getElementById('settings-db-stats').textContent = 'Local settings are unavailable. Retry Reload view when the daemon is reachable.';
        return false;
      }
    }

    async function loadCustomPricingSettings() {
      try {
        const res = await apiJson('/api/v1/pricing/custom', { headers });
        const container = document.getElementById('settings-pricing-list');
        if (!res || !res.custom_pricing || res.custom_pricing.length === 0) {
          container.innerHTML = '<span style="color: var(--text-muted);">No custom rate overrides configured (using default catalog).</span>';
          return;
        }
        let html = '<div style="display: flex; flex-direction: column; gap: 4px;">';
        for (const cp of res.custom_pricing) {
          html += `
            <div style="display: flex; justify-content: space-between; align-items: center; background: var(--bg); padding: 5px 8px; border: 1px solid var(--border); border-radius: 0;">
              <span><strong>${escapeHtml(cp.model_pattern)}</strong>: In $${cp.input_per_mtok_usd.toFixed(2)} | Out $${cp.output_per_mtok_usd.toFixed(2)} | Cache $${cp.cache_read_per_mtok_usd.toFixed(2)}</span>
              <button class="btn-control" style="padding: 2px 6px; font-size: 10px; color: var(--red);" data-action="delete-pricing" data-arg="${escapeHtml(cp.model_pattern)}">Remove</button>
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


    let remoteWasPaired = null;
    let remoteNoticeTimer;
    let remotePairingMarkup = '';
    function showRemoteStatus(status) {
      const badge = document.getElementById('remote-connection-state');
      if (remoteWasPaired !== null && Boolean(status.paired) !== remoteWasPaired) {
        remoteWasPaired = Boolean(status.paired);
        document.getElementById('remote-connection-message').textContent = status.paired ? 'Phone connected' : 'Phone disconnected';
        badge.hidden = false;
        clearTimeout(remoteNoticeTimer);
        remoteNoticeTimer = setTimeout(() => { badge.hidden = true; }, 5000);
      }
      remoteWasPaired = Boolean(status.paired);
      document.getElementById('remote-confirmation').hidden = !status.verify_code || status.paired;
      const output = document.getElementById('remote-status');
      const link = document.getElementById('remote-link');
      if (!output || !link) return;
      output.textContent = !status.enabled ? 'No remote connection.' : status.paired ? 'Phone connected.' : status.verify_code ? `Compare code ${status.verify_code} with your phone, then confirm on both devices.` : 'Waiting for the code to be entered on the web.';
      if (!status.paired && status.room_id && status.public_key && status.relay_url) {
        const markup = `<div class="desktop-pairing-code"><label for="desktop-pairing-code">Enter this code in web Profile</label><input id="desktop-pairing-code" readonly value="${escapeHtml(status.pairing_code || '')}" aria-label="Desktop pairing code"><small>Expires in five minutes. Sign in to the same account on both devices.</small></div>`;
        if (remotePairingMarkup !== markup) { link.innerHTML = markup; remotePairingMarkup = markup; }
      } else if (remotePairingMarkup) { link.textContent = ''; remotePairingMarkup = ''; }

    }

    async function loadRemoteStatus() {
      try { showRemoteStatus(await apiJson('/api/v1/remote/status', { headers })); }
      catch { document.getElementById('remote-status').textContent = 'Remote status unavailable.'; document.getElementById('remote-connection-state').hidden = true; }
    }

    async function startRemote() {
      try { showRemoteStatus(await apiJson('/api/v1/remote/pairing', { headers })); }
      catch { document.getElementById('remote-status').textContent = 'Could not start pairing. Check your Pro account and internet connection, then retry.'; }
    }

    async function confirmRemote() {
      const code = document.getElementById('remote-code').value.trim();
      if (!/^\d{6}$/.test(code)) { document.getElementById('remote-status').textContent = 'Enter the six-digit code shown on both devices.'; return; }
      try {
        showRemoteStatus(await apiJson('/api/v1/remote/confirm', { method: 'POST', headers: { ...headers, 'Content-Type': 'application/json' }, body: JSON.stringify({ code }) }));
        document.getElementById('remote-code').value = '';
      } catch { document.getElementById('remote-status').textContent = 'Codes did not match or pairing expired.'; }
    }

    async function revokeRemote() {
      try { await apiJson('/api/v1/remote/revoke', { method: 'POST', headers }); await loadRemoteStatus(); }
      catch { document.getElementById('remote-status').textContent = 'Could not revoke remote connection.'; }
    }

    document.addEventListener('click', event => {
      const target = event.target.closest('[data-action]');
      if (!target) return;
      const action = target.dataset.action;
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

    const actions = { 'dismiss-connection': () => {document.getElementById('remote-connection-state').hidden = true;}, 'dismiss-notice': () => {document.getElementById('app-notice').hidden = true;}, 'toggle-navigation': toggleNavigation, 'toggle-chat-list': toggleChatList, theme: cycleTheme, reload: refreshCurrentView, resync: resyncLocal,
      'delete-chat': deleteChat, 'load-chat': loadChat,
      'earlier-chat': () => loadChat(activeChatSession, true),
      'workspace-terminal': openWorkspaceTerminal, 'hide-workspace-terminal': () => {document.getElementById('workspace-terminal-pane').hidden = true;}, 'toggle-stream': toggleStream,
      'close-chat': showLauncher, 'new-session': showLauncher,
      'switch-model': switchChatModel, 'focus-chat': focusChat,
      'choose-workspace': chooseWorkspace,
        'copy-workspace': copySessionWorkspace, previous: prevPage, next: nextPage,
        'save-settings': saveSettings, 'add-pricing': addCustomPricingSettings,
        unpair: unpairSession, 'activate-license': activateLicenseKey, 'remove-license': removeLicenseKey,
        disarm: disarmResume, 'delete-rule': deleteRule, 'execute-relay': executeRelay,
        'dismiss-relay': dismissRelay, 'delete-pricing': deleteCustomPricing,
        'remote-start': startRemote, 'remote-confirm': confirmRemote, 'remote-revoke': revokeRemote };
      if (action === 'replace-license') { document.getElementById('license-entry').hidden = false; target.hidden = true; return; }
      if (Object.hasOwn(actions, action)) actions[action](target.dataset.arg);
    });
    document.addEventListener('change', event => {
      if (event.target.id === 'chat-model-select') { showCurrentChatModel(); return; }
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
    document.getElementById('ide-session-search').addEventListener('input', renderIdeSessions);
    document.getElementById('chat-terminal-view').addEventListener('keydown', event => {
      if (!chatWs || chatWs.readyState !== WebSocket.OPEN || event.metaKey || event.altKey) return;
      const keys = { Enter:'\r', Backspace:'\x7f', Tab:'\t', Escape:'\x1b', ArrowUp:'\x1b[A', ArrowDown:'\x1b[B', ArrowRight:'\x1b[C', ArrowLeft:'\x1b[D' };
      const input = event.ctrlKey && event.key.toLowerCase() === 'c' ? '\x03' : !event.ctrlKey ? keys[event.key] || (event.key.length === 1 ? event.key : '') : '';
      if (input) { event.preventDefault(); chatWs.send(new TextEncoder().encode(input)); }
    });
    document.addEventListener('submit', event => {
      const form = event.target.closest('[data-submit]');
      if (!form) return;
      const submit = form.dataset.submit;
      if (submit === 'pair') { event.preventDefault(); pairTokenDirectly(); }
      else if (submit === 'launch-agent') { event.preventDefault(); launchNewAgent(event); }
      else if (submit === 'send-prompt') { event.preventDefault(); sendPrompt(event); }
    });

    const initialView = window.location.hash.slice(1);
    if (['overview', 'sessions', 'usage', 'limits', 'projects', 'automations', 'settings', 'auth'].includes(initialView)) setView(initialView, false);
    else setView('overview', false);
    pollLocalData();
    if (initialView === 'sessions') loadInstalledAgents();
    setInterval(() => {
      pollLocalData();
      loadRemoteStatus();
      if (activeView === 'sessions' && !installedAgents.length) loadInstalledAgents();
      const chatPanel = document.getElementById('session-chat-panel');
      if (chatPanel && !chatPanel.hidden && activeChatSession) {
        const terminal = document.getElementById('chat-terminal-view');
        if (terminal && terminal.hidden) {
          loadChat(activeChatSession, false, true);
        }
      }
    }, 2000);
