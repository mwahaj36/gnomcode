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
    const updateNavOrientation = () => consoleNavigation.setAttribute('aria-orientation', matchMedia('(min-width: 761px)').matches ? 'vertical' : 'horizontal');
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
      const titles = { overview: 'Your agent fleet', sessions: 'Sessions', usage: 'Usage', limits: 'Limits', projects: 'Projects', automations: 'Automations', settings: 'Settings' };
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
          alert("Exporting analytics requires a Pro license. Activate Pro under the 'Pro & Guardrails' tab.");
          return;
        }
        if (!res.ok) {
          alert("Failed to export data.");
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
        alert("Export failed: " + err);
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
          if (pol) pol.value = data.config.policy;
          if (str) str.value = data.config.strategy;
          if (thr) thr.value = data.config.threshold_pct.toString();
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
          alert('Context Relay configuration requires an active spawnt Pro license.');
          return;
        }
        if (res.ok) {
          loadRelayData();
        }
      } catch (err) {
        alert('Failed to save config: ' + err.message);
      }
    }

    async function executeRelay(id) {
      if (!confirm('Rotate session now? spawnt will create .spawnt/HANDOFF.md and spawn a clean session.')) return;
      try {
        const res = await fetch('/api/v1/pro/relay/execute', {
          method: 'POST',
          headers: { ...headers, 'Content-Type': 'application/json' },
          body: JSON.stringify({ id })
        });
        if (res.status === 402) {
          alert('Context Relay execution requires an active spawnt Pro license.');
          return;
        }
        if (res.ok) {
          alert('Context Relay handoff executed successfully! Clean session launched with 0 context tokens.');
          loadRelayData();
        } else {
          alert('Failed to execute relay handoff.');
        }
      } catch (err) {
        alert('Error: ' + err.message);
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
        alert('Error: ' + err.message);
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
      if (!confirm('Remove the saved offline license? Local features remain free.')) return;
      try {
        await fetch('/api/v1/license', { method: 'DELETE', headers });
        loadProData();
      } catch (err) {
        alert(err);
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
        alert(err);
      }
    }

    async function deleteRule(ruleId) {
      try {
        await fetch(`/api/v1/rules/${encodeURIComponent(ruleId)}`, { method: 'DELETE', headers });
        loadProData();
      } catch (err) {
        alert(err);
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
        if (jsonStr === lastSessionsJson) return true;

        summaryState.activeCount = res.ongoing_count ?? res.active_count ?? null; await refreshSummary();
        document.getElementById('pagination-info').textContent = `Page ${res.page} of ${res.total_pages} (${res.total_count} total)`;
        document.getElementById('prev-btn').disabled = res.page <= 1;
        document.getElementById('next-btn').disabled = res.page >= res.total_pages;

        const presentation = await import('/console-presentation.js');
        presentation.updateSessions(document.getElementById('sessions-table'), res.sessions || [], { focus: true });
        lastSessionsJson = jsonStr;
        return true;
      } catch (e) {
        console.error('Failed to load sessions', e);
        lastSessionsJson = '';
        showConnectionError(e);
        summaryState.activeCount = null; await refreshSummary();
        document.getElementById('sessions-table').textContent = 'Session data is unavailable. Retry Reload view when the local daemon is reachable.';
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
      if (!confirm('Disconnect this desktop from your account? Recorded local sessions will stay on this device.')) return;
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
          alert('Could not disconnect. Reload the account status and retry.');
        }
      } catch (err) {
        alert('Connection could not be confirmed. Reload the account status before retrying.');
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
        if (stats) {
          document.getElementById('settings-db-stats').textContent =
            `Local Storage: ${stats.total_turns} turns EXACT across ${stats.total_projects} projects EXACT (${(stats.db_size_bytes / 1024 / 1024).toFixed(2)} MB EXACT)`;
        }
        await loadCustomPricingSettings();
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


    document.addEventListener('click', event => {
      const target = event.target.closest('[data-action]');
      if (!target) return;
      const action = target.dataset.action;
      if (action.startsWith('view:')) { event.preventDefault(); switchTab(action.slice(5)); return; }
      if (action.startsWith('export:')) { exportData(action.slice(7)); return; }
      async function loadChat(sessionId) {
      if (!sessionId) return;
      try {
        const events = await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat`, { headers });
        const presentation = await import('/console-presentation.js');
        const container = document.getElementById('chat-stream');
        if (container) presentation.updateChat(container, events || []);
      } catch (e) {
        console.error('Failed to load chat history', e);
      }
    }

    async function deleteChat(sessionId) {
      if (!sessionId || !confirm('Permanently delete local chat history for this session? This action cannot be undone.')) return;
      try {
        await apiJson(`/api/v1/sessions/${encodeURIComponent(sessionId)}/chat`, { method: 'DELETE', headers });
        await loadChat(sessionId);
      } catch (e) {
        console.error('Failed to delete chat history', e);
      }
    }

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

    const actions = { theme: cycleTheme, reload: refreshCurrentView, resync: resyncLocal,
      'delete-chat': deleteChat, 'load-chat': loadChat,
        'copy-workspace': copySessionWorkspace, previous: prevPage, next: nextPage,
        'save-settings': saveSettings, 'add-pricing': addCustomPricingSettings,
        unpair: unpairSession, 'activate-license': activateLicenseKey, 'remove-license': removeLicenseKey,
        disarm: disarmResume, 'delete-rule': deleteRule, 'execute-relay': executeRelay,
        'dismiss-relay': dismissRelay, 'delete-pricing': deleteCustomPricing };
      if (action === 'replace-license') { document.getElementById('license-entry').hidden = false; target.hidden = true; return; }
      if (Object.hasOwn(actions, action)) actions[action](target.dataset.arg);
    });
    document.addEventListener('change', event => {
      const target = event.target.closest('[data-change]');
      if (!target) return;
      if (target.dataset.change === 'timeline') setTimeline(target.value);
      else if (target.dataset.change === 'project-filter') { currentPage = 1; lastSessionsJson = ''; loadSessions(); }
      else if (target.dataset.change === 'filter') onFilterChange();
      else if (target.dataset.change === 'relay-config') saveRelayConfig();
    });
    document.addEventListener('submit', event => {
      if (event.target.dataset.submit === 'pair') { event.preventDefault(); pairTokenDirectly(); }
    });

    const initialView = window.location.hash.slice(1);
    if (['overview', 'sessions', 'usage', 'limits', 'projects', 'automations', 'settings', 'auth'].includes(initialView)) setView(initialView, false);
    else setView('overview', false);
    pollLocalData();
    setInterval(() => {
      pollLocalData();
    }, 2000);
