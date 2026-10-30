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
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input</dt><dd>${detail(session.input_tokens)}</dd><dt>Output</dt><dd>${detail(session.output_tokens)}</dd><dt>Cache read</dt><dd>${detail(session.cache_read_tokens)}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button><button type="button" data-action="load-chat" data-arg="${escape(session.id)}">View chat</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><div style="display:flex;align-items:center;justify-content:space-between;gap:8px;"><div><strong>${escape(session.agent)}</strong><span>${session.is_ongoing ? 'Ongoing · ' : ''}${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}</div><button type="button" class="btn-control" style="padding:2px 8px;font-size:11px;" data-action="load-chat" data-arg="${escape(session.id)}">Chat</button></div></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${measured ? 'EXACT' : 'UNKNOWN'}</span></td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${context ? 'EXACT · Recorded' : 'UNKNOWN'}</span><small>Window ${escape(tokens(session.context_window))} · ${Number.isFinite(session.context_window) ? 'estimated capacity' : 'capacity unknown'}</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% estimated</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><span class="sp-data-trust">${knownCost ? coverage(session) : 'Price unavailable · UNKNOWN'}</span></td><td><time datetime="${escape(session.last_event_at || '')}" title="${Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}">${illustrative ? 'Sample' : escape(relativeActivity(session.last_event_at))}</time><small>${illustrative ? '' : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}</small></td></tr>`;
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
  return `<section class="sp-ongoing"><div class="sp-panel-heading"><h2>Ongoing sessions</h2><p>In progress, waiting or recently active.</p></div>${active.length ? active.map(session => `<details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private session')}</strong><span>${escape(session.project_name || 'Private project')} · ${escape(session.agent)} · ${escape(session.model || 'Model unknown')}</span><b>${session.presence === 'recently_idle' ? 'Idle' : escape(sessionStateLabel(session.state))}</b><time datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">Last activity ${escape(relativeActivity(session.last_event_at))}</time></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Last activity</dt><dd>${escape(relativeActivity(session.last_event_at))} · ${escape(new Date(session.last_event_at).toLocaleString())}</dd><dt>Active context</dt><dd>${escape(tokens(session.context_tokens))} / ${escape(tokens(session.context_window))}</dd><dt>Project</dt><dd>${escape(session.project_name || 'Private project')}</dd></dl></details>`).join('') : '<p>No ongoing sessions at the last observation.</p>'}</section>`;
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
  return matching.map(session => `<button type="button" class="ide-session" data-action="load-chat" data-arg="${escape(session.id)}" aria-current="${session.id === selected}"><strong>${escape(session.title || session.project_name || session.repo_path || 'Untitled session')}</strong><span>${escape(session.agent)} · ${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}<time datetime="${escape(session.last_event_at || '')}">${escape(relativeActivity(session.last_event_at))}</time></button>`).join('') || '<p class="local-view-message">No matching sessions. Clear search or launch an installed agent.</p>';
}

// A deliberately restricted Markdown surface: raw HTML and embedded media stay inert.
function chatInline(text) {
  return text.split(/(`+[^`]+`+|\[[^\]]+\]\([^\s)]+\))/g).map(part => {
    if (part.startsWith('`')) return `<code>${escape(part.replace(/^`+|`+$/g, ''))}</code>`;
    const link = part.match(/^\[([^\]]+)\]\(([^\s)]+)\)$/);
    if (link) return /^https?:\/\//i.test(link[2]) ? `<a href="${escape(link[2])}" target="_blank" rel="noopener noreferrer">${escape(link[1])}</a>` : escape(part);
    return escape(part).replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>').replace(/(?<!\*)\*([^*]+)\*(?!\*)/g, '<em>$1</em>').replace(/~~([^~]+)~~/g, '<del>$1</del>');
  }).join('');
}

function chatMarkdown(text) {
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  const html = [];
  const cells = line => line.trim().replace(/^\||\|$/g, '').split('|').map(cell => cell.trim());
  for (let i = 0; i < lines.length;) {
    const line = lines[i];
    if (!line.trim()) { i++; continue; }
    const fence = line.match(/^\s*(`{3,}|~{3,})([^\s]*)\s*$/);
    if (fence) {
      const code = []; i++;
      while (i < lines.length && !new RegExp(`^\\s*${fence[1][0]}{${fence[1].length},}\\s*$`).test(lines[i])) code.push(lines[i++]);
      i++;
      html.push(`<pre><code${/^[\w+-]+$/.test(fence[2]) ? ` class="language-${escape(fence[2])}"` : ''}>${escape(code.join('\n'))}</code></pre>`);
      continue;
    }
    const heading = line.match(/^(#{1,6})\s+(.+)$/);
    if (heading) { html.push(`<h${heading[1].length}>${chatInline(heading[2])}</h${heading[1].length}>`); i++; continue; }
    if (line.includes('|') && lines[i + 1] && cells(lines[i + 1]).every(cell => /^:?-{3,}:?$/.test(cell))) {
      const head = cells(line); i += 2;
      const rows = [];
      while (i < lines.length && lines[i].includes('|') && lines[i].trim()) rows.push(`<tr>${cells(lines[i++]).map((cell, index) => `<td data-label="${escape(head[index] || '')}">${chatInline(cell)}</td>`).join('')}</tr>`);
      html.push(`<div class="sp-chat-table"${head.length > 3 ? ' data-wide="true"' : ''}><table><thead><tr>${head.map(cell => `<th>${chatInline(cell)}</th>`).join('')}</tr></thead><tbody>${rows.join('')}</tbody></table></div>`);
      continue;
    }
    const list = line.match(/^\s*(?:([-+*])|\d+[.)])\s+(.+)$/);
    if (list) {
      const tag = list[1] ? 'ul' : 'ol'; const items = [];
      const indent = line.match(/^\s*/)[0].length;
      while (i < lines.length) {
        const item = lines[i].match(/^\s*(?:([-+*])|\d+[.)])\s+(.+)$/);
        if (!item || lines[i].match(/^\s*/)[0].length !== indent || Boolean(item[1]) !== Boolean(list[1])) break;
        const continuation = []; i++;
        while (i < lines.length && lines[i].trim() && lines[i].match(/^\s*/)[0].length > indent) continuation.push(lines[i++].slice(indent + 2));
        items.push(`<li>${chatInline(item[2])}${continuation.length ? chatMarkdown(continuation.join('\n')) : ''}</li>`);
      }
      const start = tag === 'ol' ? Number.parseInt(line.trim(), 10) : 1;
      html.push(`<${tag}${start !== 1 ? ` start="${start}"` : ''}>${items.join('')}</${tag}>`); continue;
    }
    if (/^>\s?/.test(line)) { html.push(`<blockquote>${chatInline(line.replace(/^>\s?/, ''))}</blockquote>`); i++; continue; }
    if (/^\s*(?:---+|\*\*\*+|___+)\s*$/.test(line)) { html.push('<hr>'); i++; continue; }
    const paragraph = [line]; i++;
    while (i < lines.length && lines[i].trim() && !/^\s*(?:#{1,6}\s|`{3,}|~{3,}|>|[-+*]\s|\d+[.)]\s|---+$|___+$)/.test(lines[i]) && !(lines[i].includes('|') && lines[i + 1] && cells(lines[i + 1]).every(cell => /^:?-{3,}:?$/.test(cell)))) paragraph.push(lines[i++]);
    html.push(`<p>${paragraph.map(chatInline).join('<br>')}</p>`);
  }
  return html.join('');
}

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
      ${approvalPrompt}
    </article>`;
  }).join('')}</div>`;
}

export function reconcileChat(events, pending, state = '') {
  const output = [...(events || [])];
  const remaining = [];
  const consumed = new Set();
  for (const prompt of pending || []) {
    const index = output.findIndex((event,index) => !consumed.has(index) && !event.local_pending && event.role === 'user' && !prompt.baseline.includes(event.event_id) && event.content?.trim() === prompt.text.trim());
    if (index >= 0) consumed.add(index);
    if (index >= 0 && output.slice(index + 1).some(event => event.role === 'assistant' && !event.local_pending)) continue;
    prompt.baseline = [...new Set([...prompt.baseline,...[...consumed].filter(i=>i!==index).map(i=>output[i].event_id)])];
    remaining.push(prompt);
    if (index < 0) output.push({event_id:prompt.id,role:'user',content:prompt.text,created_at:prompt.created_at,local_pending:true});
  }
  if (remaining.length) {
    const latest = remaining.at(-1);
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
