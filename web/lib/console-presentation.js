import MarkdownIt from "./markdown-it.mjs";

export const sessionStateLabel = state => ({ thinking: 'Running', tool: 'Running', running: 'Running', awaiting_approval: 'Waiting', waiting: 'Waiting', idle: 'Idle', error: 'Error', ended: 'Idle' }[state] || 'Unknown');
// Shared presentation only. Rust supplies totals, pricing, and chart proportions.
const escape = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const tokens = value => Number.isFinite(value) ? new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(value) : 'Unknown';
const relativeActivity = value => { const date = new Date(value); if (!Number.isFinite(date.getTime())) return 'Unknown'; const age = Math.max(0, Math.floor((Date.now()-date.getTime())/1000)); return age < 60 ? 'Just now' : age < 3600 ? Math.floor(age/60)+'m ago' : age < 86400 ? Math.floor(age/3600)+'h ago' : Math.floor(age/86400)+'d ago'; };
const trust = value => ({ exact: 'EXACT · Provider measured', estimated: 'EST · Estimate', api: 'API · Provider API' }[value] || 'UNKNOWN');

const priced = record => record?.priced_turns > 0 || Boolean(record?.cost_basis && record.cost_basis !== 'unknown');
const coverage = record => record?.unpriced_turns > 0 ? 'EST · Partial estimate; unpriced usage excluded' : 'EST · Estimate';
const cost = (record, value) => priced(record) && Number.isFinite(value) ? '$' + value.toFixed(2) : 'Unknown';
const period = report => report?.range_label === 'This month' ? 'this month' : report?.range_label === 'Today' ? 'today' : (report?.range_label || 'selected period').toLowerCase();

export function renderProviders(report) {
  if (!Array.isArray(report?.providers)) return '<section class="sp-activity"><h2>Agent and model usage</h2><p>Usage breakdown is unavailable from this source.</p></section>';
  const money = (record, value) => cost(record, value) + ' · ' + (priced(record) ? coverage(record) : 'Price unavailable · UNKNOWN');
  return `<section class="sp-activity"><h2>Agent and model usage</h2><p>Estimated API-equivalent costs. Expand an agent for its recorded token breakdown.</p><div class="sp-provider-list">${report.providers.length ? report.providers.map(provider => `<details data-key="${escape(provider.provider)}"><summary><strong>${escape(provider.provider)}</strong><span>${money(provider, provider.total_cost_usd)}</span></summary><p>${money(provider, provider.total_cache_savings_usd)} cache savings</p>${provider.models.map(model => `<div><strong>${escape(model.model)}</strong><span>Input ${escape(tokens(model.input_tokens))}</span><span>Output ${escape(tokens(model.output_tokens))}</span><span>${money(model, model.cost_usd)}</span></div>`).join('')}</details>`).join('') : '<p>No recorded agent usage in this period.</p>'}</div></section>`;
}

export function updateProviders(element, report) {
  updateMarkup(element, renderProviders(report));
}

export function updateSummary(element, data) {
  updateMarkup(element, renderSummary(data));
}

export function renderSummary({ activeCount = null, report = null, quotaCount = null } = {}) {
  const metric = (label, value, basis, caption) => `<div><span>${label}</span><strong title="${escape(basis)}" aria-label="${escape(value)}; ${escape(basis)}">${escape(value)}</strong><span class="sp-data-trust">${escape(basis)}</span><p>${escape(caption)}</p></div>`;
  return `<section class="sp-summary-grid" aria-label="Observed fleet summary">${
    metric('Ongoing sessions', Number.isFinite(activeCount) ? activeCount : 'Unknown', Number.isFinite(activeCount) ? 'EXACT · Recorded' : 'UNKNOWN', 'Includes recent activity') +
    metric('API-equivalent spend', cost(report, report?.total_cost_usd), priced(report) ? coverage(report) : 'Price unavailable · UNKNOWN', 'API equivalent · ' + period(report)) +
    metric('Cache savings', cost(report, report?.total_cache_savings_usd), priced(report) ? coverage(report) : 'Price unavailable · UNKNOWN', Number.isFinite(report?.cache_share_pct) ? report.cache_share_pct.toFixed(1) + '% cached input (estimated)' : 'Cached input share unknown') +
    metric('Quota pools', Number.isFinite(quotaCount) ? quotaCount : 'Unknown', Number.isFinite(quotaCount) ? 'EXACT · Recorded' : 'UNKNOWN', 'At last source observation')
  }</section>`;
}

export function renderProjects(projects) {
  if (!Array.isArray(projects)) return '<section class="sp-activity"><h2>Project activity</h2><p>Project rollups remain local. This cloud source does not publish repository identity or project totals.</p></section>';
  return `<section class="sp-activity"><h2>Project activity</h2><p>Projects with measured usage in the selected period. Repository identity stays on this device.</p><div class="sp-quota-grid">${projects.length ? projects.map(project => `<a class="sp-project-link" href="#sessions" data-project-id="${escape(project.id)}"><article><header><strong>${escape(project.name)}</strong></header><div class="sp-quota-number">${escape(tokens(project.total_tokens))}<span class="sp-data-trust">EXACT · recorded tokens</span></div><p>${escape(project.sessions)} recorded sessions</p><p>${cost(project, project.cost_usd)} · ${priced(project) ? coverage(project) : 'Price unavailable · UNKNOWN'}</p></article></a>`).join('') : '<p>No project usage was recorded in this period.</p>'}</div></section>`;
}

export function renderSessions(sessions, { focus = false, rotate = false, illustrative = false } = {}) {
  if (!sessions?.length) return '<p class="sp-session-empty">No sessions match this view. Clear filters or launch an installed agent to start recording usage.</p>';
  return `<table class="sp-session-table"><thead><tr><th>Session</th><th>Agent / inferred state</th><th>Recorded tokens</th><th>Context / estimated capacity</th><th>API equivalent</th><th>Last activity</th></tr></thead><tbody>${sessions.map(session => {
    const measured = session.usage_available !== false && Number.isFinite(session.total_tokens);
    const context = session.usage_available !== false && Number.isFinite(session.context_tokens);
    const fill = Number.isFinite(session.context_fill_pct) ? session.context_fill_pct : null;
    const knownCost = priced(session) && Number.isFinite(session.total_cost_usd);
    const activeAt = new Date(session.last_event_at);
    const detail = value => session.usage_available !== false && Number.isFinite(value) ? escape(tokens(value)) + ' recorded' : 'Unknown · UNKNOWN';
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}${isSubagent ? ' · Subagent' : ''}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.parent_session_id ? `<dt>Parent session</dt><dd>${escape(session.parent_session_id)}</dd>` : ''}${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input</dt><dd>${detail(session.input_tokens)}</dd><dt>Output</dt><dd>${detail(session.output_tokens)}</dd><dt>Cache read</dt><dd>${detail(session.cache_read_tokens)}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button><button type="button" data-action="load-chat" data-arg="${escape(session.id)}">View chat</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><div style="display:flex;align-items:center;justify-content:space-between;gap:8px;"><div><strong>${escape(session.agent)}</strong><span>${session.is_ongoing ? 'Ongoing · ' : ''}${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}</div><button type="button" class="btn-control" style="padding:2px 8px;font-size:11px;" data-action="load-chat" data-arg="${escape(session.id)}">Chat</button></div></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${measured ? 'EXACT' : 'UNKNOWN'}</span></td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${context ? 'EXACT · Recorded' : 'UNKNOWN'}</span><small>Window ${escape(tokens(session.context_window))} · ${Number.isFinite(session.context_window) ? 'estimated capacity' : 'capacity unknown'}</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% estimated</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><span class="sp-data-trust">${knownCost ? coverage(session) : 'Price unavailable · UNKNOWN'}</span></td><td><time datetime="${escape(session.last_event_at || '')}" title="${Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}">${illustrative ? 'Sample' : escape(relativeActivity(session.last_event_at))}</time><small>${illustrative ? '' : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}</small></td></tr>`;
  }).join('')}</tbody></table>`;
}

export function renderActivity(report) {
  if (!Array.isArray(report?.daily)) return '<section class="sp-activity"><h2>Activity</h2><p>Measurements are unavailable. Update the paired desktop app to publish this view.</p></section>';
  if (!report.daily.length) return '<section class="sp-activity"><h2>Activity</h2><p>No recorded usage in this period. Unobserved intervals are not counted as zero.</p></section>';
  const hours = report.range_label === 'Today';
  const interval = report.activity_interval || (hours ? 'hour' : 'day');
  const days = report.daily.slice(-84);
  const chartDate = value => new Date(value).toLocaleDateString('en-US', {month:'short',day:'numeric',year:'numeric',...(hours ? {} : {timeZone:'UTC'})});
  const chartWindow = `${chartDate(report.earliest_at || days[0].date)} – ${chartDate(report.latest_at || days.at(-1).date)}`;
  const weekLabel = value => { const date=new Date(value+'T00:00:00Z'); date.setUTCDate(date.getUTCDate()+3); const year=date.getUTCFullYear(); const first=new Date(Date.UTC(year,0,4)); const firstMonday=new Date(first); firstMonday.setUTCDate(first.getUTCDate()-((first.getUTCDay()+6)%7)); return year+' · Week '+(Math.floor((date-firstMonday)/604800000)+1); };
  const label = day => interval === 'week' ? weekLabel(day.date) : interval === 'year' ? day.date.slice(0,4) : interval === 'month' ? new Date(day.date).toLocaleDateString('en-US',{month:'short',year:'numeric',timeZone:'UTC'}) : hours ? new Date(day.date).toLocaleTimeString([], {hour:'2-digit', minute:'2-digit', hour12:false}) : day.date.slice(5);
  return `<section class="sp-activity" aria-label="Recorded token activity">
    <div class="sp-activity-heading"><div><h2>${hours ? 'Today, by hour' : 'Recorded activity'}</h2><p>${hours ? 'Since local midnight · local time' : `${escape(report.range_label || 'Selected period')} · ${interval === 'week' ? 'weekly' : interval === 'month' ? 'monthly' : interval === 'year' ? 'yearly' : 'daily'} totals · UTC`} · expand for details</p></div><div class="sp-activity-totals"><div><strong>${escape(Number.isFinite(report.total_tokens) ? new Intl.NumberFormat('en', {notation:'compact', compactDisplay:'long', maximumFractionDigits:1}).format(report.total_tokens) : 'Unknown')}</strong><span class="sp-data-trust">EXACT · Recorded tokens · ${escape(period(report))}</span></div></div></div>
    <p class="sp-activity-caption">Chart window: ${escape(chartWindow)}. ${days.length < report.daily.length ? 'Latest 84 recorded intervals; the total above covers the full selected period.' : 'All recorded intervals in the selected period.'} Gaps are not counted as zero.</p>
    <div class="sp-activity-bars" role="img" aria-label="Recorded token volumes; measurements below">${days.map((day, index) => `<i style="--day-delay:${Math.min(index, 20) * 25}ms;--day-height:${Number.isFinite(day.chart_pct) ? Math.min(100, Math.max(0, day.chart_pct)) : 0}%" title="${escape(label(day))}: ${escape(tokens(day.total_tokens))} tokens"></i>`).join('')}</div>
    <div class="sp-activity-grid">${days.map(day => `<details data-date="${escape(day.date)}"><summary><time datetime="${escape(day.date)}">${escape(label(day))}</time><strong>${escape(tokens(day.total_tokens))}</strong></summary><div><span>API equivalent</span><strong>${cost(day, day.cost_usd)}</strong><span>${priced(day) ? coverage(day) : 'Price unavailable · UNKNOWN'}</span><span>${escape(day.turns)} recorded invocations</span></div></details>`).join('')}</div>
    <p class="sp-activity-caption">${days.length < report.daily.length ? 'Most recent 84 intervals. ' : ''}Only recorded activity is shown.</p>
  </section>`;
}

export function renderOngoing(sessions) {
  if (!Array.isArray(sessions)) return '<section class="sp-ongoing"><h2>Ongoing sessions</h2><p>Session status is unavailable.</p></section>';
  const active = sessions.filter(session => session.is_ongoing ?? session.is_active);
  return `<section class="sp-ongoing"><div class="sp-panel-heading"><h2>Ongoing sessions</h2><p>In progress, waiting or recently active.</p></div>${active.length ? active.map(session => {
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    const tokenCount = Number.isFinite(session.total_tokens) ? `${escape(tokens(session.total_tokens))} tokens` : 'Tokens unknown';
    const subagentBadge = isSubagent ? ' · Subagent' : '';
    return `<details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private session')}</strong><span>${escape(session.project_name || 'Private project')} · ${escape(session.agent)}${subagentBadge} · ${escape(session.model || 'Model unknown')} · ${tokenCount}</span><b>${session.presence === 'recently_idle' ? 'Idle' : escape(sessionStateLabel(session.state))}</b><time datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">Last activity ${escape(relativeActivity(session.last_event_at))}</time></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Recorded tokens</dt><dd>${Number.isFinite(session.total_tokens) ? escape(tokens(session.total_tokens)) + ' recorded' : 'Tokens unknown'}</dd><dt>Active context</dt><dd>${escape(tokens(session.context_tokens))} / ${escape(tokens(session.context_window))}</dd>${session.parent_session_id ? `<dt>Parent session</dt><dd>${escape(session.parent_session_id)}</dd>` : ''}<dt>Last activity</dt><dd>${escape(relativeActivity(session.last_event_at))} · ${escape(new Date(session.last_event_at).toLocaleString())}</dd><dt>Project</dt><dd>${escape(session.project_name || 'Private project')}</dd></dl></details>`;
  }).join('') : '<p>No ongoing sessions at the last observation.</p>'}</section>`;
}

export function updateOngoing(element, sessions) { updateMarkup(element, renderOngoing(sessions)); }

const activityMarkup = new WeakMap();
export function updateActivity(element, report) {
  const markup = renderActivity(report);
  updateMarkup(element, markup);
}

export function updateSessions(element, sessions, options) {
  updateMarkup(element, renderSessions(sessions, options));
}

function updateMarkup(element, markup) {
  if (activityMarkup.get(element) === markup) return;
  const previousValues = [...element.querySelectorAll('strong')].map(value => value.textContent);
  const key = day => day.dataset.key || day.dataset.date;
  const open = new Set([...element.querySelectorAll('details[open]')].map(key));
  const focused = element.contains(document.activeElement) ? key(document.activeElement.closest('details') || { dataset: {} }) : null;
  const focusedAction = element.contains(document.activeElement) ? document.activeElement.dataset.sessionAction : null;
  const entering = !element.querySelector('.sp-activity-bars');
  element.innerHTML = markup;
  element.querySelectorAll('strong').forEach((value, index) => {
    if (previousValues[index] !== undefined && previousValues[index] !== value.textContent) value.classList.add('sp-value-changed');
  });
  if (entering) element.querySelector('.sp-activity')?.setAttribute('data-enter', '');
  for (const day of element.querySelectorAll('details')) {
    day.open = open.has(key(day));
    if (focused && focused === key(day)) {
      const target = focusedAction ? [...day.querySelectorAll('[data-session-action]')].find(button => button.dataset.sessionAction === focusedAction) : null;
      (target || day.querySelector('summary')).focus({ preventScroll: true });
    }
  }
  activityMarkup.set(element, markup);
}

export function updateQuota(element, pools, publishedAt) { updateMarkup(element, renderQuota(pools, publishedAt)); }

export function renderQuota(pools, publishedAt, { illustrative = false, compact = true } = {}) {
  if (compact && pools?.length) {
    const grouped = new Map();
    for (const pool of pools) { const list = grouped.get(pool.provider) || []; list.push(pool); grouped.set(pool.provider, list); }
    const stale = !publishedAt || !Number.isFinite(Date.parse(publishedAt)) || Date.now()-Date.parse(publishedAt)>120000;
    return `<section class="sp-limits-compact" aria-label="Provider limits"><div class="sp-panel-heading"><h2>Provider limits</h2><p>${illustrative ? 'Illustrative limits' : stale ? 'Last snapshot stale or unavailable' : 'Latest received limits'}</p></div>${[...grouped].map(([provider, windows])=>`<details data-key="${escape(provider)}"><summary><strong>${escape(provider)}</strong><div class="sp-limit-windows">${windows.map(pool=>`<span title="${escape(trust(pool.basis))}; ${Number.isFinite(pool.remaining_pct)?pool.remaining_pct.toFixed(1)+'% remaining':'remaining unknown'}"><small>${escape(pool.window?.replaceAll('_',' ') || 'Window')}</small><b>${pool.basis && pool.basis!=='unknown' && Number.isFinite(pool.used_pct) ? pool.used_pct.toFixed(1)+'%' : 'Unknown'}</b></span>`).join('')}</div></summary>${renderQuota(windows,publishedAt,{illustrative,compact:false})}</details>`).join('')}</section>`;
  }
  const publication = publishedAt ? new Date(publishedAt).getTime() : NaN;
  const stale = !Number.isFinite(publication) || Date.now() - publication > 120000;
  return `<section class="sp-quota-section" aria-label="Provider limits"><div class="sp-activity-heading"><div><h2>Provider limits</h2><p>${illustrative ? 'Illustrative quota samples. Not connected to your account.' : stale ? 'Snapshot stale or publication time unknown. Values below are last observed.' : 'Latest received snapshot. Providers update limits when they emit usage.'}</p></div><span class="sp-data-trust">${illustrative ? 'SAMPLE' : stale ? 'STALE' : 'RECEIVED'}</span></div>
    ${pools?.length ? `<div class="sp-quota-grid">${pools.map(pool => {
      const known = pool.basis && pool.basis !== 'unknown' && typeof pool.used_pct === 'number' && Number.isFinite(pool.used_pct);
      const used = known ? Math.min(100, Math.max(0, pool.used_pct)) : 0;
      const reset = pool.resets_at ? new Date(pool.resets_at).getTime() : NaN;
      const observation = pool.observed_at ? new Date(pool.observed_at) : null;
      return `<article><header><strong>${escape(pool.provider)} / ${escape(pool.limit_id)}</strong><span>${escape(pool.window?.replaceAll('_', ' '))}</span></header><div class="sp-quota-number">${known ? used.toFixed(1) + '%' : 'Unknown'}<span class="sp-data-trust">${known ? ({exact:'EXACT · Provider measured',api:'API · Provider API',estimated:'EST · Estimate'}[pool.basis] || 'UNKNOWN') : 'UNKNOWN'} · used</span></div><div class="sp-quota-track" aria-hidden="true"><i style="width:${used}%"></i></div><p>${Number.isFinite(pool.remaining_pct) ? pool.remaining_pct.toFixed(1) + '% remaining' : 'Remaining capacity unknown'}</p><p>${illustrative ? "Resets " + escape(pool.reset_label || "unknown") : Number.isFinite(reset) ? reset <= Date.now() ? 'Reset passed · awaiting provider update' : 'Resets ' + escape(new Date(reset).toLocaleString()) : 'Reset time unknown'}</p><small>Observed ${illustrative ? "sample" : observation && Number.isFinite(observation.getTime()) ? escape(observation.toLocaleString()) : 'unknown'}</small></article>`;
    }).join('')}</div>` : '<p>No measured quota pools are available. Remaining capacity is unknown.</p>'}</section>`;
}


export function renderSessionList(sessions, selected = '', query = '') {
  const matching = sessions.filter(session => `${session.title || ''} ${session.agent} ${session.repo_path || session.project_name || ''}`.toLowerCase().includes(query.trim().toLowerCase()));
  return matching.map(session => {
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    return `<button type="button" class="ide-session" data-action="load-chat" data-arg="${escape(session.id)}" aria-current="${session.id === selected}"><strong>${escape(session.title || session.project_name || session.repo_path || 'Untitled session')}</strong><span>${escape(session.agent)}${isSubagent ? ' · Subagent' : ''} · ${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}<time datetime="${escape(session.last_event_at || '')}">${escape(relativeActivity(session.last_event_at))}</time></button>`;
  }).join('') || '<p class="local-view-message">No matching sessions. Clear search or launch an installed agent.</p>';
}

// Raw HTML and remote image fetching stay disabled for agent-supplied Markdown.
const markdown = new MarkdownIt({html:false,breaks:true,linkify:false});
markdown.renderer.rules.image = (tokens,index) => `<span class="sp-chat-attachment">Image: ${escape(tokens[index].content || 'attachment')} (preview unavailable)</span>`;
markdown.renderer.rules.table_open = (tokens,index,options,env) => {
  env.chatTableHeaders=[];
  for(let i=index+1;i<tokens.length && tokens[i].type!=='thead_close';i++) if(tokens[i].type==='inline') env.chatTableHeaders.push(tokens[i].children?.map(token=>token.content).join('') || tokens[i].content);
  return `<div class="sp-chat-table"${env.chatTableHeaders.length>3?' data-wide="true"':''}><table>`;
};
markdown.renderer.rules.table_close = () => '</table></div>';
markdown.renderer.rules.link_open = (tokens,index,options,env,self) => {
  tokens[index].attrSet('target','_blank');tokens[index].attrSet('rel','noopener noreferrer');
  return self.renderToken(tokens,index,options);
};
markdown.renderer.rules.tr_open = (tokens,index,options,env,self) => {env.chatTableColumn=0;return self.renderToken(tokens,index,options);};
markdown.renderer.rules.td_open = (tokens,index,options,env,self) => {
  tokens[index].attrSet('data-label',env.chatTableHeaders?.[env.chatTableColumn++] || '');
  return self.renderToken(tokens,index,options);
};
markdown.core.ruler.after('inline','chat-task-lists',state=>{
  for(let i=2;i<state.tokens.length;i++){
    const token=state.tokens[i],first=token.children?.[0];
    if(token.type!=='inline' || state.tokens[i-2].type!=='list_item_open' || first?.type!=='text')continue;
    const task=first.content.match(/^\[([ xX])\]\s+/);if(!task)continue;
    first.content=first.content.slice(task[0].length);
    const checkbox=new state.Token('html_inline','',0);
    checkbox.content=`<input type="checkbox" disabled${task[1].toLowerCase()==='x'?' checked':''} aria-label="Task status"> `;
    token.children.unshift(checkbox);
  }
});
const chatMarkdown = text => markdown.render(text,{});

export function renderChatCards(events) {
  events = events?.filter(evt => (evt.role === 'user' || evt.role === 'assistant' || (evt.role === 'system' && ['model_active','model_changed'].includes(evt.status))) && evt.content?.trim());
  if (!Array.isArray(events) || !events.length) {
    return '<div class="sp-chat-empty"><p>No conversation history recorded for this session.</p></div>';
  }
  return `<div class="sp-chat-stream">${events.map(evt => {
    if (evt.role === 'system') return `<div class="sp-chat-notice" role="status">${escape(evt.content)}<time datetime="${escape(evt.created_at)}">${escape(new Date(evt.created_at).toLocaleString())}</time></div>`;
    const role = evt.role || 'system';
    const roleLabel = role === 'user' ? 'You' : 'Agent';
    const statusBadge = evt.status ? `<span class="sp-chat-badge sp-badge-${escape(evt.status)}">${escape(evt.status.toUpperCase())}</span>` : '';
    const approvalPrompt = evt.status === 'pending' ? `<p class="sp-approval-banner">Approval is pending in the agent.</p>` : '';
    return `<article data-chat-id="${escape(evt.event_id || '')}" class="sp-chat-card sp-chat-${escape(role)}${evt.waiting ? ' sp-chat-waiting' : ''}">
      <header>
        <strong>${escape(roleLabel)}</strong>
        ${statusBadge}
        <time>${escape(new Date(evt.created_at).toLocaleTimeString())}</time>
      </header>
      <div class="sp-chat-body">${role === 'assistant' ? chatMarkdown(evt.content) : escape(evt.content)}</div>
      ${evt.delivery_note ? `<p class="sp-chat-delivery-note" role="status">${escape(evt.delivery_note)}</p>` : ''}
      ${approvalPrompt}
    </article>`;
  }).join('')}</div>`;
}

export function reconcileChat(events, pending, state = '') {
  const output = [...(events || [])];
  const remaining = [];
  const consumed = new Set();
  let waiting;
  for (const prompt of pending || []) {
    const index = output.findIndex((event,index) => !consumed.has(index) && !event.local_pending && event.role === 'user' && !prompt.baseline.includes(event.event_id) && event.content?.trim() === prompt.text.trim());
    if (index >= 0) consumed.add(index);
    if (index >= 0 && output.slice(index + 1).some(event => event.role === 'assistant' && !event.local_pending)) continue;
    prompt.baseline = [...new Set([...prompt.baseline,...[...consumed].filter(i=>i!==index).map(i=>output[i].event_id)])];
    remaining.push(prompt);
    const unconfirmed = index < 0 && prompt.delivered && (Date.now()-Date.parse(prompt.created_at)>15000 || output.some(event=>event.role==='user' && !event.local_pending && Date.parse(event.created_at)>Date.parse(prompt.created_at)));
    if (!unconfirmed) waiting = prompt;
    if (index < 0) {
      const event={event_id:prompt.id,role:'user',content:prompt.text,created_at:prompt.created_at,local_pending:true,...(unconfirmed ? {delivery_note:'Not confirmed by agent. Check Terminal before retrying.'}: {})};
      const before=output.findIndex(item=>Date.parse(item.created_at)>Date.parse(prompt.created_at));
      if(before<0)output.push(event);else output.splice(before,0,event);
    }
  }
  if (waiting || (remaining.length && ['ended','disconnected'].includes(state))) {
    const latest = waiting || remaining.at(-1);
    const content = ['ended','disconnected'].includes(state) ? 'Agent session ended without a recorded reply.' : !latest.delivered ? 'Sending...' : state === 'thinking' ? 'Thinking...' : state === 'waiting' ? 'Waiting for approval in Terminal stream' : 'Waiting for agent response...';
    output.push({event_id:latest.id+':response',role:'assistant',content,created_at:latest.created_at,local_pending:true,waiting:true});
  }
  return {events:output,pending:remaining};
}

export function updateChat(element, events) {
  const previousHeight = element.scrollHeight;
  const previousTop = element.scrollTop;
  const follow = previousHeight - previousTop - element.clientHeight < 80;
  const previous = new Set([...element.querySelectorAll('[data-chat-id]')].map(node => node.dataset.chatId));
  updateMarkup(element, renderChatCards(events));
  element.querySelectorAll('[data-chat-id]').forEach(node => {
    if (!previous.has(node.dataset.chatId) && (previous.size || events.some(event=>event.local_pending)) && !matchMedia('(prefers-reduced-motion: reduce)').matches) node.animate([{opacity:0,transform:'translateY(6px)'},{opacity:1,transform:'translateY(0)'}],{duration:180,easing:'cubic-bezier(.16,1,.3,1)'});
  });
  element.scrollTop = follow ? element.scrollHeight : previousTop;
}

export function renderConversationLimits(limits = []) {
  if (!limits.length) return '<span>Limits unavailable <small>UNKNOWN</small></span>';
  return limits.map(limit => {
    const known = ['exact','estimated','api'].includes(limit.basis) && Number.isFinite(limit.remaining_pct);
    const observed = limit.observed_at ? new Date(limit.observed_at).toLocaleString() : 'Unknown';
    const basis = {exact:'EXACT',estimated:'EST',api:'API',unknown:'UNKNOWN'}[limit.basis] || 'UNKNOWN';
    return `<span class="sp-conversation-limit" title="Last updated ${escape(observed)}; ${escape(trust(limit.basis))}"><span>${escape(limit.label)}</span> <strong>${known ? limit.remaining_pct.toFixed(0) + '% left' : 'Unknown'}</strong> <small>${basis}${limit.stale ? ' · Last seen' : ''}</small></span>`;
  }).join('');
}
