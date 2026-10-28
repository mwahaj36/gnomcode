// Shared presentation only. Rust supplies totals, pricing, and chart proportions.
const escape = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const tokens = value => Number.isFinite(value) ? new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(value) : 'Unknown';
const relativeActivity = value => { const date = new Date(value); if (!Number.isFinite(date.getTime())) return 'Unknown'; const age = Math.max(0, Math.floor((Date.now()-date.getTime())/1000)); return age < 60 ? 'Just now' : age < 3600 ? Math.floor(age/60)+'m ago' : age < 86400 ? Math.floor(age/3600)+'h ago' : Math.floor(age/86400)+'d ago'; };
const trust = value => ({ exact: 'Provider measured', estimated: 'Estimate', api: 'Provider API' }[value] || 'Unknown');

const priced = record => record?.priced_turns > 0 || Boolean(record?.cost_basis && record.cost_basis !== 'unknown');
const coverage = record => record?.unpriced_turns > 0 ? 'Partial estimate; unpriced usage excluded' : 'Estimate';
const cost = (record, value) => priced(record) && Number.isFinite(value) ? '$' + value.toFixed(2) : 'Unknown';
const period = report => report?.range_label === 'This month' ? 'this month' : report?.range_label === 'Today' ? 'today' : (report?.range_label || 'selected period').toLowerCase();

export function renderProviders(report) {
  if (!Array.isArray(report?.providers)) return '<section class="sp-activity"><h2>Agent and model usage</h2><p>Usage breakdown is unavailable from this source.</p></section>';
  const money = (record, value) => cost(record, value) + ' · ' + (priced(record) ? coverage(record) : 'Price unavailable');
  return `<section class="sp-activity"><h2>Agent and model usage</h2><p>Estimated API-equivalent costs. Expand an agent for its recorded token breakdown.</p><div class="sp-provider-list">${report.providers.length ? report.providers.map(provider => `<details data-key="${escape(provider.provider)}"><summary><strong>${escape(provider.provider)}</strong><span>${money(provider, provider.total_cost_usd)}</span></summary><p>${money(provider, provider.total_cache_savings_usd)} cache savings</p>${provider.models.map(model => `<div><strong>${escape(model.model)}</strong><span>Input ${escape(tokens(model.input_tokens))}</span><span>Output ${escape(tokens(model.output_tokens))}</span><span>${money(model, model.cost_usd)}</span></div>`).join('')}</details>`).join('') : '<p>No recorded agent usage in this period.</p>'}</div></section>`;
}

export function updateProviders(element, report) {
  updateMarkup(element, renderProviders(report));
}

export function updateSummary(element, data) {
  updateMarkup(element, renderSummary(data));
}

export function renderSummary({ activeCount = null, report = null, quotaCount = null } = {}) {
  const metric = (label, value, basis, caption) => `<div><span>${label}</span><strong title="${escape(basis)}" aria-label="${escape(value)}; ${escape(basis)}">${escape(value)}</strong><p>${escape(caption)}</p></div>`;
  return `<section class="sp-summary-grid" aria-label="Observed fleet summary">${
    metric('Ongoing sessions', Number.isFinite(activeCount) ? activeCount : 'Unknown', Number.isFinite(activeCount) ? 'Recorded' : 'Unavailable', 'Includes recent activity') +
    metric('API-equivalent spend', cost(report, report?.total_cost_usd), priced(report) ? coverage(report) : 'Price unavailable', 'API equivalent · ' + period(report)) +
    metric('Cache savings', cost(report, report?.total_cache_savings_usd), priced(report) ? coverage(report) : 'Price unavailable', Number.isFinite(report?.cache_share_pct) ? report.cache_share_pct.toFixed(1) + '% cached input (estimated)' : 'Cached input share unknown') +
    metric('Quota pools', Number.isFinite(quotaCount) ? quotaCount : 'Unknown', Number.isFinite(quotaCount) ? 'Recorded' : 'Unavailable', 'At last source observation')
  }</section>`;
}

export function renderProjects(projects) {
  if (!Array.isArray(projects)) return '<section class="sp-activity"><h2>Project activity</h2><p>Project rollups remain local. This cloud source does not publish repository identity or project totals.</p></section>';
  return `<section class="sp-activity"><h2>Project activity</h2><p>Projects with measured usage in the selected period. Repository identity stays on this device.</p><div class="sp-quota-grid">${projects.length ? projects.map(project => `<a class="sp-project-link" href="#sessions" data-project-id="${escape(project.id)}"><article><header><strong>${escape(project.name)}</strong></header><div class="sp-quota-number">${escape(tokens(project.total_tokens))}<span class="sp-data-trust">recorded tokens</span></div><p>${escape(project.sessions)} recorded sessions</p><p>${cost(project, project.cost_usd)} · ${priced(project) ? coverage(project) : 'Price unavailable'}</p></article></a>`).join('') : '<p>No project usage was recorded in this period.</p>'}</div></section>`;
}

export function renderSessions(sessions, { focus = false, rotate = false, illustrative = false } = {}) {
  if (!sessions?.length) return '<p class="sp-session-empty">No sessions match this view. Usage remains unknown until an agent reports it.</p>';
  return `<table class="sp-session-table"><thead><tr><th>Session</th><th>Agent / inferred state</th><th>Recorded tokens</th><th>Context / estimated capacity</th><th>API equivalent</th><th>Last activity</th></tr></thead><tbody>${sessions.map(session => {
    const measured = session.usage_available !== false && Number.isFinite(session.total_tokens);
    const context = session.usage_available !== false && Number.isFinite(session.context_tokens);
    const fill = Number.isFinite(session.context_fill_pct) ? session.context_fill_pct : null;
    const knownCost = priced(session) && Number.isFinite(session.total_cost_usd);
    const activeAt = new Date(session.last_event_at);
    const detail = value => session.usage_available !== false && Number.isFinite(value) ? escape(tokens(value)) + ' recorded' : 'Unknown · UNKNOWN';
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input</dt><dd>${detail(session.input_tokens)}</dd><dt>Output</dt><dd>${detail(session.output_tokens)}</dd><dt>Cache read</dt><dd>${detail(session.cache_read_tokens)}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><strong>${escape(session.agent)}</strong><span>${session.is_ongoing ? 'Ongoing · ' : ''}${escape((session.state || 'unknown').replaceAll('_', ' '))}</span></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong>${!measured ? '<span class="sp-data-trust">UNKNOWN</span>' : ''}</td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${context ? 'Recorded' : 'Unavailable'}</span><small>Window ${escape(tokens(session.context_window))} · ${Number.isFinite(session.context_window) ? 'estimated capacity' : 'capacity unknown'}</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% estimated</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><span class="sp-data-trust">${knownCost ? coverage(session) : 'Price unavailable'}</span></td><td><time datetime="${escape(session.last_event_at || '')}" title="${Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}">${illustrative ? 'Sample' : escape(relativeActivity(session.last_event_at))}</time><small>${illustrative ? '' : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}</small></td></tr>`;
  }).join('')}</tbody></table>`;
}

export function renderActivity(report) {
  if (!Array.isArray(report?.daily)) return '<section class="sp-activity"><h2>Activity</h2><p>Measurements are unavailable. Update the paired desktop app to publish this view.</p></section>';
  if (!report.daily.length) return '<section class="sp-activity"><h2>Activity</h2><p>No recorded usage in this period. Unobserved intervals are not counted as zero.</p></section>';
  const hours = report.range_label === 'Today';
  const days = report.daily.slice(-84);
  const label = day => hours ? new Date(day.date).toLocaleTimeString([], {hour:'2-digit', minute:'2-digit', hour12:false}) : day.date.slice(5);
  return `<section class="sp-activity" aria-label="Recorded token activity">
    <div class="sp-activity-heading"><div><h2>${hours ? 'Today, by hour' : 'Daily activity'}</h2><p>${hours ? 'Since local midnight · local time' : 'Recorded days · UTC'} · expand for details</p></div><div class="sp-activity-totals"><div><strong>${escape(Number.isFinite(report.total_tokens) ? new Intl.NumberFormat('en', {notation:'compact', compactDisplay:'long', maximumFractionDigits:1}).format(report.total_tokens) : 'Unknown')}</strong><span class="sp-data-trust">Recorded tokens · ${escape(period(report))}</span></div></div></div>
    <div class="sp-activity-bars" role="img" aria-label="Recorded token volumes; measurements below">${days.map((day, index) => `<i style="--day-delay:${Math.min(index, 20) * 25}ms;--day-height:${Number.isFinite(day.chart_pct) ? Math.min(100, Math.max(0, day.chart_pct)) : 0}%" title="${escape(label(day))}: ${escape(tokens(day.total_tokens))} tokens"></i>`).join('')}</div>
    <div class="sp-activity-grid">${days.map(day => `<details data-date="${escape(day.date)}"><summary><time datetime="${escape(day.date)}">${escape(label(day))}</time><strong>${escape(tokens(day.total_tokens))}</strong></summary><div><span>API equivalent</span><strong>${cost(day, day.cost_usd)}</strong><span>${priced(day) ? coverage(day) : 'Price unavailable'}</span><span>${escape(day.turns)} recorded invocations</span></div></details>`).join('')}</div>
    <p class="sp-activity-caption">${days.length < report.daily.length ? 'Most recent 84 intervals. ' : ''}Only recorded activity is shown.</p>
  </section>`;
}

export function renderOngoing(sessions) {
  if (!Array.isArray(sessions)) return '<section class="sp-ongoing"><h2>Ongoing sessions</h2><p>Session status is unavailable.</p></section>';
  const active = sessions.filter(session => session.is_ongoing ?? session.is_active);
  return `<section class="sp-ongoing"><div class="sp-panel-heading"><h2>Ongoing sessions</h2><p>In progress, waiting or recently active.</p></div>${active.length ? active.map(session => `<details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private session')}</strong><span>${escape(session.agent)} · ${escape(session.model || 'Model unknown')}</span><b>${session.presence === 'recently_idle' ? 'Recently idle' : escape((session.state || 'unknown').replaceAll('_',' '))}</b><time datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">Last activity ${escape(relativeActivity(session.last_event_at))}</time></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Last activity</dt><dd>${escape(relativeActivity(session.last_event_at))} · ${escape(new Date(session.last_event_at).toLocaleString())}</dd><dt>Active context</dt><dd>${escape(tokens(session.context_tokens))} / ${escape(tokens(session.context_window))}</dd><dt>Project</dt><dd>${escape(session.project_name || 'Private project')}</dd></dl></details>`).join('') : '<p>No ongoing sessions at the last observation.</p>'}</section>`;
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
      return `<article><header><strong>${escape(pool.provider)} / ${escape(pool.limit_id)}</strong><span>${escape(pool.window?.replaceAll('_', ' '))}</span></header><div class="sp-quota-number">${known ? used.toFixed(1) + '%' : 'Unknown'}<span class="sp-data-trust">${known ? ({exact:'Provider measured',api:'Provider API',estimated:'Estimate'}[pool.basis] || 'Unavailable') : 'Unavailable'} · used</span></div><div class="sp-quota-track" aria-hidden="true"><i style="width:${used}%"></i></div><p>${Number.isFinite(pool.remaining_pct) ? pool.remaining_pct.toFixed(1) + '% remaining' : 'Remaining capacity unknown'}</p><p>${illustrative ? "Resets " + escape(pool.reset_label || "unknown") : Number.isFinite(reset) ? reset <= Date.now() ? 'Reset passed · awaiting provider update' : 'Resets ' + escape(new Date(reset).toLocaleString()) : 'Reset time unknown'}</p><small>Observed ${illustrative ? "sample" : observation && Number.isFinite(observation.getTime()) ? escape(observation.toLocaleString()) : 'unknown'}</small></article>`;
    }).join('')}</div>` : '<p>No measured quota pools are available. Remaining capacity is unknown.</p>'}</section>`;
}


export function renderChatCards(events) {
  if (!Array.isArray(events) || !events.length) {
    return '<div class="sp-chat-empty"><p>No conversation history recorded for this session.</p></div>';
  }
  return `<div class="sp-chat-stream">${events.map(evt => {
    const role = evt.role || 'system';
    const roleLabel = role === 'user' ? 'User' : role === 'assistant' ? 'Agent' : role === 'tool' ? (evt.tool_name ? `Tool: ${evt.tool_name}` : 'Tool') : 'System';
    const statusBadge = evt.status ? `<span class="sp-chat-badge sp-badge-${escape(evt.status)}">${escape(evt.status.toUpperCase())}</span>` : '';
    const approvalPrompt = evt.status === 'pending' ? `
      <div class="sp-approval-banner" data-tool-call="${escape(evt.tool_name || '')}">
        <p><strong>Action requires approval:</strong> ${escape(evt.content)}</p>
        <div class="sp-approval-actions">
          <button type="button" class="btn btn-primary" data-chat-action="approve" data-event-id="${escape(evt.event_id)}">Approve</button>
          <button type="button" class="btn btn-secondary" data-chat-action="deny" data-event-id="${escape(evt.event_id)}">Deny</button>
        </div>
      </div>` : '';
    return `<article class="sp-chat-card sp-chat-${escape(role)}">
      <header>
        <strong>${escape(roleLabel)}</strong>
        ${statusBadge}
        <time>${escape(new Date(evt.created_at).toLocaleTimeString())}</time>
      </header>
      <div class="sp-chat-body">${escape(evt.content)}</div>
      ${approvalPrompt}
    </article>`;
  }).join('')}</div>`;
}

export function updateChat(element, events) {
  updateMarkup(element, renderChatCards(events));
}
