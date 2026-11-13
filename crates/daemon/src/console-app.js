// Console shell. Home, Workspace, Usage and Settings are React views in /console-react.js;
// this file owns view routing, connection status, account sign-in and pairing and the
// delegated clicks for presenter markup.
    const token = new URLSearchParams(window.location.search).get('token') || '';
    const headers = token ? { 'Authorization': `Bearer ${token}` } : {};
    const isPhone = () => matchMedia('(max-width: 760px)').matches;
    const fire = (name, detail) => window.dispatchEvent(new CustomEvent(name, { detail }));
    // Workspace actions that arrive before the Workspace view has mounted wait for it.
    let workspaceReady = false;
    const queuedWorkspaceActions = [];
    const workspaceAction = detail => { if (workspaceReady) fire('gnomcode:workspace-action', detail); else queuedWorkspaceActions.push(detail); };
    function storedFlag(key) { try { return localStorage.getItem(key); } catch (_) { return null; } }
    function storeFlag(key, value) { try { localStorage.setItem(key, value); } catch (_) { /* storage unavailable */ } }

    async function apiJson(url, options = { headers }) {
      const response = await fetch(url, options);
      if (!response.ok) {const error=await response.json().catch(()=>null);throw new Error(response.status === 401 ? 'Local authorization is missing or expired. Close this window and reopen gnomcode from its tray icon or shortcut.' : typeof error?.message === 'string' ? error.message : `gnomcode request failed (${response.status}). Retry Reload view or restart gnomcode if this persists.`);}
      return response.json();
    }

    const showNotice = message => window.__gnomcodeShell.showNotice(message);
    const confirmAction = (message, label) => window.__gnomcodeShell.confirmAction(message, label);

// Views and navigation
    let activeView = 'overview';
    function consoleViewUrl(view) { const url = new URL(window.location.href); url.hash = view === 'overview' ? '' : view; return url.pathname + url.search + url.hash; }
    document.querySelector('.app-brand').href = consoleViewUrl('overview');
    document.querySelector('.sp-console-nav [role=tablist]').setAttribute('aria-orientation', 'vertical');

    // Old links keep working: Limits and Projects moved into Usage, Automations into Settings.
    const VIEW_ALIASES = { limits: 'usage', projects: 'usage', automations: 'settings' };
    const VIEWS = ['overview', 'sessions', 'usage', 'settings', 'auth'];
    const CONSOLE_VIEWS = ['overview', 'sessions', 'usage'];
    const TITLES = { overview: 'Home', sessions: 'Workspace', usage: 'Usage', settings: 'Settings', auth: 'Account' };
    const resolveView = name => VIEW_ALIASES[name] || name;

    function setView(view, updateUrl = true) {
      view = resolveView(view);
      activeView = view;
      try { sessionStorage.setItem('gnomcode-active-view', view); } catch (_) {}
      document.body.dataset.consoleView = view;
      document.getElementById('chat-list-toggle').hidden = view !== 'sessions';
      document.getElementById('console-title').textContent = TITLES[view] || 'Home';
      if (CONSOLE_VIEWS.includes(view)) {
        document.getElementById('view-console').setAttribute('aria-labelledby', `tab-btn-${view}`);
      }
      document.querySelectorAll('.tab-btn').forEach(button => {
        const selected = button.dataset.view === view;
        button.classList.toggle('active', selected);
        button.setAttribute('aria-selected', String(selected));
        button.tabIndex = selected || (view === 'auth' && button.dataset.view === 'settings') ? 0 : -1;
      });
      document.querySelectorAll('[data-local-view]').forEach(section => {
        section.hidden = !section.dataset.localView.split(',').includes(view);
      });
      document.querySelectorAll('.view-content').forEach(section => section.classList.toggle('active', section.id === `view-${view}` || (section.id === 'view-console' && CONSOLE_VIEWS.includes(view))));
      if (updateUrl && window.location.hash.slice(1) !== view) history.pushState(null, '', consoleViewUrl(view));
      if (view !== 'auth') fire('gnomcode:view-change', { view });
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

    // Conversation list: open on desktop unless the person collapsed it; phones start collapsed.
    function applyChatList(collapsed) {
      document.body.classList.toggle('chat-list-collapsed', collapsed);
      document.getElementById('chat-list-toggle').setAttribute('aria-expanded', String(!collapsed));
    }
    function toggleChatList() {
      const collapsed = !document.body.classList.contains('chat-list-collapsed');
      applyChatList(collapsed);
      if (!isPhone()) storeFlag('gnomcode-chat-list-collapsed', collapsed ? '1' : '0');
    }
    applyChatList(isPhone() || storedFlag('gnomcode-chat-list-collapsed') === '1');

    // Deep link from the file manager: #sessions?folder=<path> opens the launcher on that folder.
    function openExplorerWorkspace() {
      const url = new URL(window.location.href);
      if (!url.hash.startsWith('#sessions?')) return;
      const folder = new URLSearchParams(url.hash.slice('#sessions?'.length)).get('folder');
      if (!folder) return;
      url.hash = 'sessions';
      history.replaceState(null, '', url.toString());
      setView('sessions', false);
      workspaceAction({ type: 'new-session', folder });
    }
// End views and navigation

// Connection status
    let refreshBusy = false;
    let pollBusy = false;
    let lastSuccessfulUpdate = null;
    const syncState = (text, failed = false) => {
      const state = document.getElementById('sync-state');
      state.textContent = text;
      state.classList.toggle('local-status-error', failed);
    };
    // The views load their own data; the shell only needs to know the daemon still answers.
    async function probeDaemon() {
      try {
        await Promise.all([
          apiJson('/api/v1/sessions?page=1&limit=1', { headers, signal: AbortSignal.timeout(15000) }),
          apiJson('/api/v1/providers/overview', { headers, signal: AbortSignal.timeout(15000) }),
        ]);
        return true;
      } catch (_) { return false; }
    }
    async function pollLocalData() {
      if (refreshBusy || pollBusy || document.hidden) return;
      if (!CONSOLE_VIEWS.includes(activeView) && activeView !== 'settings') return;
      pollBusy = true;
      try {
        if (await probeDaemon()) {
          lastSuccessfulUpdate = new Date();
          syncState('Local daemon connected');
        } else {
          syncState(lastSuccessfulUpdate ? 'Local data stale' : 'Local data unavailable', true);
        }
      } finally { pollBusy = false; }
    }
    async function refreshCurrentView() {
      if (refreshBusy) return;
      refreshBusy = true;
      try {
        while (pollBusy) await new Promise(resolve => setTimeout(resolve, 25));
        fire('gnomcode:reload-view', { view: activeView });
        if (activeView === 'auth' ? !await checkAuthSession() : !await probeDaemon()) throw new Error('Local data could not be loaded');
        lastSuccessfulUpdate = new Date();
        syncState('View refreshed');
      } catch {
        syncState('Refresh failed', true);
      } finally {
        refreshBusy = false;
      }
    }
// End connection status

// Account
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
        document.getElementById('auth-cloud-status').textContent = res.paired ? 'Connected to your account' : 'Not connected. Local monitoring is ready.';
        document.getElementById('account-entry').hidden = Boolean(res.paired);
        document.getElementById('auth-paired-card').hidden = !res.paired;
        document.getElementById('auth-unpaired-card').hidden = Boolean(res.paired);
        document.getElementById('auth-paired-email').textContent = res.email || 'Connected account';
        return true;
      } catch (_) {
        document.getElementById('auth-cloud-status').textContent = 'Account connection unavailable. Retry Reload view.';
        document.getElementById('auth-publication-status').textContent = 'Local monitoring does not require an account connection.';
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
// End account


// Delegated clicks: presenter markup in the views carries data-action instead of handlers.
    document.addEventListener('click', async event => {
      const copy = event.target.closest('[data-copy]');
      if (!copy) return;
      const label = copy.querySelector('span');
      try { await navigator.clipboard.writeText(copy.dataset.copy); if (label) label.textContent = 'Copied'; }
      catch (_) { if (label) label.textContent = 'Select and copy'; }
      setTimeout(() => { if (label) label.textContent = 'Copy'; }, 1500);
    });

    document.addEventListener('click', event => {
      const project = event.target.closest('[data-project-id]');
      if (!project) return;
      event.preventDefault();
      navigateView('sessions');
    });

    const actions = {
      'toggle-navigation': toggleNavigation,
      'toggle-chat-list': toggleChatList,
      'home-new-session': () => { navigateView('sessions'); workspaceAction({ type: 'new-session' }); },
      'open-home-session': id => { navigateView('sessions'); workspaceAction({ type: 'open-session', sessionId: id }); },
      unpair: unpairSession,
    };
    document.addEventListener('click', event => {
      const target = event.target.closest('[data-action]');
      if (!target) return;
      const action = target.dataset.action;
      if (action.startsWith('view:')) {
        event.preventDefault();
        navigateView(action.slice(5));
        if (event.detail && isPhone()) document.querySelector('.view-content.active')?.focus({preventScroll:true});
        return;
      }
      if (action === 'toggle-pair-token') {
        const input = document.getElementById('auth-token-input');
        const visible = input.type === 'password';
        input.type = visible ? 'text' : 'password';
        target.setAttribute('aria-pressed', String(visible));
        target.textContent = visible ? 'Hide token' : 'Show token';
        return;
      }
      if (Object.hasOwn(actions, action)) actions[action](target.dataset.arg);
    });
    document.addEventListener('submit', event => {
      const form = event.target.closest('[data-submit]');
      if (!form) return;
      const submit = form.dataset.submit;
      if (submit === 'signin') { event.preventDefault(); signInAccount(); }
      else if (submit === 'pair') { event.preventDefault(); pairTokenDirectly(); }
    });

    // Bridge the React views call to move around the shell.
    window.__gnomcodeLocalActions = {
      setView: view => { navigateView(view); },
      openSession: id => {
        navigateView('sessions');
        workspaceAction({ type: 'open-session', sessionId: id });
      },
      workspaceAction,
      workspaceReady: () => { workspaceReady = true; queuedWorkspaceActions.splice(0).forEach(detail => fire('gnomcode:workspace-action', detail)); },
    };

    normalizeHash();
    const hashView = resolveView(window.location.hash.slice(1));
    let initialView = hashView;
    if (!initialView) {
      try { initialView = resolveView(sessionStorage.getItem('gnomcode-active-view') || ''); } catch (_) {}
    }
    if (VIEWS.includes(initialView)) setView(initialView, Boolean(hashView));
    else setView('overview', false);
    openExplorerWorkspace();
    void checkAuthSession();
    pollLocalData();
    setInterval(pollLocalData, 2000);
