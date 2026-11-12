import MarkdownIt from "./markdown-it.mjs";

export const sessionStateLabel = state => ({ thinking: 'Running', tool: 'Running', running: 'Running', awaiting_approval: 'Waiting', waiting: 'Waiting', idle: 'Idle', error: 'Error', ended: 'Ended' }[state] || 'Unknown');
// Shared presentation only. Rust supplies totals, pricing, and chart proportions.
const escape = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const tokens = value => Number.isFinite(value) ? new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(value) : 'Unknown';
const relativeActivity = value => { const date = new Date(value); if (!Number.isFinite(date.getTime())) return 'Unknown'; const age = Math.max(0, Math.floor((Date.now()-date.getTime())/1000)); return age < 60 ? 'Just now' : age < 3600 ? Math.floor(age/60)+'m ago' : age < 86400 ? Math.floor(age/3600)+'h ago' : Math.floor(age/86400)+'d ago'; };
const trust = value => ({ exact: 'Provider measured', estimated: 'Estimated', api: 'Provider API' }[value] || 'UNKNOWN');

const priced = record => record?.priced_turns > 0 || Boolean(record?.cost_basis && record.cost_basis !== 'unknown');
const coverage = record => record?.unpriced_turns > 0 ? 'Partial estimate; unpriced usage excluded' : 'Estimated';
const cost = (record, value) => priced(record) && Number.isFinite(value) ? '$' + value.toFixed(2) : 'Unknown';
const period = report => report?.range_label === 'This month' ? 'this month' : report?.range_label === 'Today' ? 'today' : (report?.range_label || 'selected period').toLowerCase();

export function renderProviders(report) {
  if (!Array.isArray(report?.providers)) return '<section class="sp-activity"><h2>Agent and model usage</h2><p>Usage breakdown is unavailable from this source.</p></section>';
  const money = (record, value) => `<span title="${escape(priced(record) ? coverage(record) : 'Price unavailable')}">${cost(record, value)}${record?.unpriced_turns > 0 && priced(record) ? ' · partial' : ''}</span>`;
  return `<section class="sp-activity"><h2>Agent and model usage</h2><p>Estimated API-equivalent costs. Expand an agent for its recorded token breakdown.</p><div class="sp-provider-list">${report.providers.length ? report.providers.map(provider => `<details data-key="${escape(provider.provider)}"><summary><strong>${escape(provider.provider)}</strong><span>${money(provider, provider.total_cost_usd)}</span></summary><p>${money(provider, provider.total_cache_savings_usd)} cache savings</p>${provider.models.map(model => `<div><strong>${escape(model.model)}</strong><span>Input ${escape(tokens(model.input_tokens))}</span><span>Output ${escape(tokens(model.output_tokens))}</span><span>${money(model, model.cost_usd)}</span></div>`).join('')}</details>`).join('') : '<p>No recorded agent usage in this period.</p>'}</div></section>`;
}

export function updateProviders(element, report) {
  updateMarkup(element, renderProviders(report));
}

export function updateSummary(element, data) {
  updateMarkup(element, renderSummary(data));
}

export function renderSummary({ activeCount = null, report = null, quotaCount = null } = {}) {
  if (!report && !Number.isFinite(activeCount) && !Number.isFinite(quotaCount)) return '';
  const metric = (label, value, basis, caption) => `<div><span>${label}</span><strong title="${escape(basis)}" aria-label="${escape(value)}; ${escape(basis)}">${escape(value)}</strong><p>${escape(caption)}</p></div>`;
  return `<section class="sp-summary-grid" aria-label="Observed fleet summary">${
    metric('Recent sessions', Number.isFinite(activeCount) ? activeCount : 'Unknown', Number.isFinite(activeCount) ? 'EST · Inferred' : 'UNKNOWN', 'From recent activity') +
    metric('Estimated spend', cost(report, report?.total_cost_usd), priced(report) ? coverage(report) : 'Price unavailable', 'API equivalent · ' + period(report)) +
    metric('Estimated savings', cost(report, report?.total_cache_savings_usd), priced(report) ? coverage(report) : 'Price unavailable', Number.isFinite(report?.cache_share_pct) ? report.cache_share_pct.toFixed(1) + '% cached input' : 'Cached input share unknown') +
    metric('Quota pools', Number.isFinite(quotaCount) ? quotaCount : 'Unknown', Number.isFinite(quotaCount) ? 'EXACT · Recorded' : 'UNKNOWN', 'Provider limits')
  }</section><details class="sp-data-details"><summary>About these numbers</summary><p>Session counts are inferred from recent records, not a live process count. Spend and savings are API-equivalent estimates, not invoices. Unpriced usage is excluded. Token and quota counts come from recorded observations.</p></details>`;
}

export function renderProjects(projects) {
  if (!Array.isArray(projects)) return '<section class="sp-activity"><h2>Project activity</h2><p>Project rollups remain local. This cloud source does not publish repository identity or project totals.</p></section>';
  return `<section class="sp-activity"><h2>Project activity</h2><p>Usage by project in this period.</p><div class="sp-quota-grid">${projects.length ? projects.map(project => `<a class="sp-project-link" href="#sessions" data-project-id="${escape(project.id)}"><article><header><strong>${escape(project.name)}</strong></header><div class="sp-quota-number">${escape(tokens(project.total_tokens))}<span class="sp-data-unit">recorded tokens</span></div><p>${escape(project.sessions)} recorded sessions</p><p>${cost(project, project.cost_usd)} · ${priced(project) ? coverage(project) : 'Price unavailable'}</p></article></a>`).join('') : '<p>No project usage was recorded in this period.</p>'}</div></section>`;
}

export function renderSessions(sessions, { focus = false, rotate = false, illustrative = false } = {}) {
  if (!sessions?.length) return '<p class="sp-session-empty">No sessions match this view. Clear filters or launch an installed agent to start recording usage.</p>';
  return `<table class="sp-session-table"><thead><tr><th>Session</th><th>Agent</th><th>Recorded tokens</th><th>Context</th><th>Estimated cost</th><th>Last activity</th></tr></thead><tbody>${sessions.map(session => {
    const measured = session.usage_available !== false && Number.isFinite(session.total_tokens);
    const context = session.usage_available !== false && Number.isFinite(session.context_tokens);
    const fill = Number.isFinite(session.context_fill_pct) ? session.context_fill_pct : null;
    const knownCost = priced(session) && Number.isFinite(session.total_cost_usd);
    const activeAt = new Date(session.last_event_at);
    const detail = value => session.usage_available !== false && Number.isFinite(value) ? escape(tokens(value)) + ' recorded' : 'Unknown';
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}${isSubagent ? ' · Subagent' : ''}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.parent_session_id ? `<dt>Parent session</dt><dd>${escape(session.parent_session_id)}</dd>` : ''}${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input</dt><dd>${detail(session.input_tokens)}</dd><dt>Output</dt><dd>${detail(session.output_tokens)}</dd><dt>Cache read</dt><dd>${detail(session.cache_read_tokens)}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button><button type="button" data-action="load-chat" data-arg="${escape(session.id)}">View chat</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><div style="display:flex;align-items:center;justify-content:space-between;gap:8px;"><div><strong>${escape(session.agent)}</strong><span>${session.is_ongoing ? 'Ongoing · ' : ''}${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}</div><button type="button" class="btn-control" style="padding:2px 8px;font-size:11px;" data-action="load-chat" data-arg="${escape(session.id)}">Chat</button></div></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong></td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><small>Window ${escape(tokens(session.context_window))} · ${Number.isFinite(session.context_window) ? 'estimated capacity' : 'capacity unknown'}</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% estimated</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><small>${knownCost && session.unpriced_turns > 0 ? 'Partial estimate' : ''}</small></td><td><time datetime="${escape(session.last_event_at || '')}" title="${Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}">${illustrative ? 'Sample' : escape(relativeActivity(session.last_event_at))}</time><small>${illustrative ? '' : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}</small></td></tr>`;
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
    <div class="sp-activity-heading"><div><h2>${hours ? 'Today, by hour' : 'Recorded activity'}</h2><p>${hours ? 'Since local midnight · local time' : `${escape(report.range_label || 'Selected period')} · ${interval === 'week' ? 'weekly' : interval === 'month' ? 'monthly' : interval === 'year' ? 'yearly' : 'daily'} totals · UTC`}</p></div><div class="sp-activity-totals"><div><strong>${escape(Number.isFinite(report.total_tokens) ? new Intl.NumberFormat('en', {notation:'compact', compactDisplay:'long', maximumFractionDigits:1}).format(report.total_tokens) : 'Unknown')}</strong><span class="sp-data-unit">Recorded tokens</span></div></div></div>
    <p class="sp-activity-caption">${escape(chartWindow)}</p>
    <div class="sp-activity-bars" role="img" aria-label="Recorded token volumes; measurements below">${days.map((day, index) => `<i style="--day-delay:${Math.min(index, 20) * 25}ms;--day-height:${Number.isFinite(day.chart_pct) ? Math.min(100, Math.max(0, day.chart_pct)) : 0}%" title="${escape(label(day))}: ${escape(tokens(day.total_tokens))} tokens"></i>`).join('')}</div>
    <details class="sp-activity-breakdown"><summary>Activity details</summary><p>Recorded totals; gaps are not counted as zero. Costs are estimated. ${days.length < report.daily.length ? 'The chart shows the latest 84 intervals; the total covers the full period.' : ''}</p><div class="sp-activity-grid">${days.map(day => `<details data-date="${escape(day.date)}"><summary><time datetime="${escape(day.date)}">${escape(label(day))}</time><strong>${escape(tokens(day.total_tokens))}</strong></summary><div><span>API equivalent</span><strong>${cost(day, day.cost_usd)}</strong><span>${priced(day) ? coverage(day) : 'Price unavailable'}</span><span>${escape(day.turns)} recorded invocations</span></div></details>`).join('')}</div>
    </details>
  </section>`;
}

export function renderOngoing(sessions) {
  if (!Array.isArray(sessions)) return '<section class="sp-ongoing"><h2>Recent sessions</h2><p>Session status is unavailable.</p></section>';
  const active = sessions.filter(session => session.is_ongoing ?? session.is_active);
  return `<section class="sp-ongoing"><div class="sp-panel-heading"><h2>Recent sessions</h2></div>${active.length ? active.map(session => {
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    const tokenCount = Number.isFinite(session.total_tokens) ? `${escape(tokens(session.total_tokens))} tokens` : 'Tokens unknown';
    const subagentBadge = isSubagent ? ' · Subagent' : '';
    const project = session.project_name && session.project_name.toLowerCase() !== 'unknown' ? session.project_name : 'Private project';
    const title = session.title && session.title.toLowerCase() !== 'unknown' ? session.title : project === 'Private project' ? 'Private session' : project;
    const state = session.presence === 'recently_idle' ? 'Idle' : sessionStateLabel(session.state);
    return `<details data-key="${escape(session.id)}"><summary><strong>${escape(title)}</strong><span>${escape(project)} · ${escape(session.agent)}${subagentBadge} · ${escape(session.model || 'Model unknown')} · ${tokenCount}</span><b>${escape(state)}</b><time datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">Last activity ${escape(relativeActivity(session.last_event_at))}</time></summary><dl><dt>Status source</dt><dd>Inferred from recorded activity; this does not confirm a process is running.</dd><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Recorded tokens</dt><dd>${Number.isFinite(session.total_tokens) ? escape(tokens(session.total_tokens)) + ' recorded' : 'Tokens unknown'}</dd><dt>Active context</dt><dd>${escape(tokens(session.context_tokens))} / ${escape(tokens(session.context_window))}</dd>${session.parent_session_id ? `<dt>Parent session</dt><dd>${escape(session.parent_session_id)}</dd>` : ''}<dt>Last activity</dt><dd>${escape(relativeActivity(session.last_event_at))} · ${escape(new Date(session.last_event_at).toLocaleString())}</dd><dt>Project</dt><dd>${escape(project)}</dd></dl></details>`;
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

export function updateQuota(element, pools, publishedAt) { updateProviderCards(element, quotaOverviews(pools), { cost: false, publishedAt }); }

// ---- Provider limit cards (SPEC 3.9) ----
// Rust supplies every state, label, pace, pool and headline window. Nothing below decides what is low,
// exhausted or on pace; it only formats those fields into markup.
const WINDOW_STATES = ['ok', 'low', 'exhausted', 'reset_awaiting', 'unknown'];
const stateOf = value => WINDOW_STATES.includes(value?.state) ? value.state : 'unknown';
// Payloads synced from an older desktop have no state but still say whether the reset has passed.
const awaiting = win => stateOf(win) === 'reset_awaiting' || win.reset_passed === true;
const CHIP_TEXT = { ok: 'OK', low: 'Low', exhausted: 'Out', reset_awaiting: 'Resetting', unknown: 'No data' };
const CHIP_CLASS = { ok: 'ok', low: 'low', exhausted: 'exhausted', reset_awaiting: 'unknown', unknown: 'unknown' };
const WINDOW_NAMES = { five_hour: 'Session', weekly: 'Weekly', monthly: 'Monthly' };
const titleCase = text => String(text ?? '').replace(/[_-]+/g, ' ').replace(/\b\w/g, char => char.toUpperCase()).trim();
const windowName = win => win.window_label || WINDOW_NAMES[win.window] || titleCase(win.window) || 'Window';
const windowLabel = win => win.label || windowName(win);
const percent = value => `${Math.round(value)}%`;
const barWidth = win => awaiting(win) ? 100 : Number.isFinite(win.remaining_pct) ? Math.min(100, Math.max(0, win.remaining_pct)) : 0;
const seenText = win => {
  if (!Number.isFinite(Date.parse(win.observed_at))) return win.stale ? 'Last seen' : '';
  return `${win.stale ? 'Last seen' : 'Seen'} ${relativeActivity(win.observed_at)}`;
};

// "in 2h 14m" within 48 hours, otherwise a weekday and clock time. `bare` drops the "in".
function whenPart(iso, { short = false, bare = false } = {}) {
  const at = Date.parse(iso);
  if (!Number.isFinite(at)) return null;
  const minutes = Math.max(0, Math.round((at - Date.now()) / 60000));
  if (minutes < 2880) {
    const left = minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${String(minutes % 60).padStart(2, '0')}m`;
    return bare ? left : `in ${left}`;
  }
  return new Date(at).toLocaleString(undefined, short ? { weekday: 'short' } : { weekday: 'short', hour: 'numeric', minute: '2-digit' });
}

function valueText(win) {
  const state = stateOf(win);
  if (state === 'exhausted') return 'Exhausted';
  if (awaiting(win)) return 'Reset';
  return Number.isFinite(win.remaining_pct) && win.basis !== 'unknown' ? `${percent(win.remaining_pct)} left` : 'Unknown';
}

function detailText(win) {
  const seen = seenText(win).toLowerCase();
  if (win.source_issue) return seen ? `${win.source_issue} · ${seen}` : win.source_issue;
  const state = stateOf(win);
  if (awaiting(win)) return 'Awaiting next reading';
  const when = whenPart(win.resets_at);
  if (state === 'exhausted') return when ? `Back ${when}` : 'Reset time unknown';
  return when ? `Resets ${when}` : win.reset_label ? `Resets ${win.reset_label}` : 'Reset time unknown';
}

function paceText(win) {
  const pace = win.pace;
  if (!pace) return '';
  if (pace.verdict === 'on_pace') return 'On pace';
  if (pace.verdict === 'behind') return `${Math.round(Math.abs(pace.delta_pct))}% in reserve`;
  const runsOut = pace.runs_out_at ? whenPart(pace.runs_out_at) : null;
  return runsOut ? `Runs out ${runsOut}` : `${Math.round(Math.abs(pace.delta_pct))}% ahead of pace`;
}

function meterTitle(win) {
  const parts = [`${trust(win.basis)}.`];
  const seen = seenText(win);
  if (seen) parts.push(`${seen}.`);
  if (win.source_issue) parts.push(`${win.source_issue}.`);
  return parts.join(' ');
}

const meterBar = (win, name) => {
  const known = Number.isFinite(win.remaining_pct);
  const width = Number(barWidth(win).toFixed(1));
  const pace = Number.isFinite(win.pace?.pace_remaining_pct) ? `<i class="sp-meter-pace" style="--pace:${Number(Math.min(100, Math.max(0, win.pace.pace_remaining_pct)).toFixed(1))}%"></i>` : '';
  const aria = known ? `role="meter" aria-valuemin="0" aria-valuemax="100" aria-valuenow="${Math.round(win.remaining_pct)}"` : 'role="img"';
  return `<div class="sp-meter" ${aria} aria-label="${escape(`${name}: ${valueText(win)}`)}"><i class="sp-meter-fill" style="--fill:${width}%"></i>${pace}</div>`;
};

export function renderMeterRow(win, { detail = true } = {}) {
  const state = stateOf(win);
  const name = windowLabel(win);
  return `<div class="sp-meter-row sp-meter-row--${state}${win.source_issue ? ' sp-meter-row--issue' : ''}" title="${escape(meterTitle(win))}"><div class="sp-meter-top"><span class="sp-meter-label">${escape(name)}</span><span class="sp-meter-value">${escape(valueText(win))}</span></div>${meterBar(win, name)}${detail ? `<div class="sp-meter-bottom"><span>${escape(detailText(win))}</span><span>${escape(paceText(win))}</span></div>` : ''}</div>`;
}

function renderMiniRow(win) {
  const state = stateOf(win);
  const name = windowName(win);
  const when = whenPart(win.resets_at, { short: true, bare: true }) || '';
  const value = state === 'exhausted' ? 'Out' : awaiting(win) ? 'Reset' : Number.isFinite(win.remaining_pct) && win.basis !== 'unknown' ? percent(win.remaining_pct) : 'Unknown';
  return `<div class="sp-mini sp-mini--${state}${win.source_issue ? ' sp-mini--issue' : ''}" title="${escape(meterTitle(win))}"><span>${escape(name)}</span>${meterBar(win, windowLabel(win))}<span class="sp-mini-value"><b>${escape(value)}</b>${escape(awaiting(win) ? '' : when)}</span></div>`;
}

/** Multi-pool providers: windows grouped under their Rust-supplied pool name as compact rows. */
export function renderPoolGroup(windows) {
  const groups = new Map();
  for (const win of windows) {
    const key = win.pool || '';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(win);
  }
  return `<div class="sp-windows">${[...groups].map(([pool, list]) => `<div class="sp-pool">${pool ? `<p class="sp-pool-name">${escape(pool)}</p>` : ''}${list.map(renderMiniRow).join('')}</div>`).join('')}</div>`;
}

const CHEVRON = '<svg class="sp-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>';
const providerName = p => p.display_name || titleCase(p.provider);
const providerMark = p => `<span class="sp-provider-mark" aria-hidden="true">${escape(Array.from(providerName(p))[0]?.toUpperCase() || '?')}</span>`;
const chip = state => `<span class="sp-chip sp-chip--${CHIP_CLASS[state]}">${CHIP_TEXT[state]}</span>`;
const money = (value, basis) => Number.isFinite(value) && !(basis === 'unknown' && value === 0) ? `$${value.toFixed(2)}${basis === 'exact' ? '' : ' est.'}` : 'Unknown';

function summaryLine(p, windows) {
  if (p.source_issue) return `<span class="sp-provider-line sp-provider-line--issue" title="${escape(p.source_issue)}">${escape(p.source_issue)}</span>`;
  const head = windows[Number.isInteger(p.headline_window) && windows[p.headline_window] ? p.headline_window : 0];
  const state = stateOf(head);
  const label = escape(windowLabel(head));
  const when = whenPart(head.resets_at, { short: true });
  const text = state === 'exhausted' ? `${label} <b>exhausted</b>${when ? ` · back ${escape(whenPart(head.resets_at))}` : ''}`
    : awaiting(head) ? `${label} <b>reset</b> · awaiting reading`
      : `${label} <b>${escape(valueText(head))}</b>${when ? ` · resets ${escape(when)}` : ''}`;
  return `<span class="sp-provider-line">${text}</span>`;
}

function costLine(p) {
  if (!p.cost) return '';
  return `<span>Today ${escape(money(p.cost.today_usd, p.cost.basis))} · 30 days ${escape(money(p.cost.last_30_days_usd, p.cost.basis))}${p.cost.basis === 'unknown' ? ' · partial' : ''}</span>`;
}

function emptyCard(p) {
  const text = p.source_issue || p.enable_hint || 'No reading yet.';
  const command = p.enable_command ? `<button type="button" class="sp-copy" data-copy="${escape(p.enable_command)}"><code>${escape(p.enable_command)}</code><span>Copy</span></button>` : '';
  return `<article class="sp-card sp-provider-card sp-provider-card--empty" data-provider="${escape(p.provider)}"><header class="sp-provider-head"><div class="sp-provider-id">${providerMark(p)}<div><span class="sp-provider-name">${escape(providerName(p))}</span>${p.plan ? `<span class="sp-provider-plan">${escape(p.plan)}</span>` : ''}</div></div>${chip('unknown')}</header><p class="sp-provider-empty">${escape(text)}</p>${command}</article>`;
}

/** One provider as a `<details>` card, collapsed by default; the summary is the one-line overview. */
export function renderProviderCard(p, { compact = false, cost = true } = {}) {
  const windows = Array.isArray(p?.windows) ? p.windows : [];
  if (!windows.length) return emptyCard(p || {});
  const state = stateOf(p);
  const multiPool = windows.some(win => win.pool);
  const strip = windows.map(win => `<i class="is-${stateOf(win)}"></i>`).join('');
  const body = multiPool ? renderPoolGroup(windows) : `<div class="sp-windows">${windows.map(win => renderMeterRow(win, { detail: !compact })).join('')}</div>`;
  const issue = multiPool && p.source_issue ? `<p class="sp-provider-issue">${escape(p.source_issue)}</p>` : '';
  const spend = cost && !compact ? costLine(p) : '';
  const source = p.source_label ? `${escape(p.source_label)}${Number.isFinite(Date.parse(p.updated_at)) ? ` · Seen ${escape(relativeActivity(p.updated_at))}` : ''}` : '';
  const foot = !compact && (spend || source) ? `<footer class="sp-provider-foot"${p.source_label ? ` title="Limit source: ${escape(p.source_label)}"` : ''}>${spend}<span>${source}</span></footer>` : '';
  const slug = escape(p.provider);
  return `<details class="sp-card sp-provider-card${compact ? ' sp-provider-card--compact' : ''}" data-key="provider:${slug}" data-provider="${slug}"><summary>${providerMark(p)}<span class="sp-provider-sum"><span class="sp-provider-name">${escape(providerName(p))}${p.plan ? `<span class="sp-provider-plan">${escape(p.plan)}</span>` : ''}</span>${summaryLine(p, windows)}</span><span class="sp-sum-right"><span class="sp-strip" aria-hidden="true">${strip}</span>${chip(state)}</span>${CHEVRON}</summary><div class="sp-provider-body">${issue}${body}${foot}</div></details>`;
}

/** The Limits section: heading, freshness and a grid of provider cards. */
export function renderProviderCards(list, options = {}) {
  const items = Array.isArray(list) ? list : [];
  const received = Date.parse(options.publishedAt);
  const updated = items.map(p => Date.parse(p.updated_at)).filter(Number.isFinite);
  const meta = Number.isFinite(received) ? `Last received ${relativeActivity(received)}` : updated.length ? `Updated ${relativeActivity(Math.max(...updated))}` : '';
  const grid = items.length ? `<div class="sp-card-grid">${items.map(p => renderProviderCard(p, options)).join('')}</div>` : '<div class="sp-empty"><p>No measured quota pools are available. Remaining capacity is unknown.</p></div>';
  return `<section class="sp-section sp-limits" aria-label="Provider limits"><header class="sp-section-head"><h2>Limits</h2><span class="sp-section-meta">${escape(meta)}</span></header>${grid}</section>`;
}

const PROVIDER_OPEN_KEY = 'gnomcode-provider-open';
function readProviderOpen() {
  try { const stored = JSON.parse(localStorage.getItem(PROVIDER_OPEN_KEY)); return stored && typeof stored === 'object' ? stored : {}; } catch (_) { return {}; }
}
function writeProviderOpen(slug, open) {
  try { localStorage.setItem(PROVIDER_OPEN_KEY, JSON.stringify({ ...readProviderOpen(), [slug]: open })); } catch (_) { /* storage unavailable */ }
}
const toggleListeners = new WeakSet();

/** Re-renders the cards without collapsing the ones the user opened; open state survives reloads. */
export function updateProviderCards(element, list, options) {
  const first = !element.querySelector('details[data-provider]');
  updateMarkup(element, renderProviderCards(list, options));
  if (first) {
    element.querySelector('.sp-limits')?.setAttribute('data-enter', '');
    const stored = readProviderOpen();
    for (const card of element.querySelectorAll('details[data-provider]')) card.open = stored[card.dataset.provider] === true;
  }
  if (toggleListeners.has(element)) return;
  toggleListeners.add(element);
  // toggle does not bubble, so listen during the capture phase.
  element.addEventListener('toggle', event => {
    const card = event.target;
    if (!card?.dataset?.provider || !card.matches?.('details')) return;
    writeProviderOpen(card.dataset.provider, card.open);
    for (const other of document.querySelectorAll('details[data-provider]')) if (other !== card && other.dataset.provider === card.dataset.provider && other.open !== card.open) other.open = card.open;
  }, true);
}

// Raw pools (web console, Remote, tour) carry per-window Rust fields but no provider overview.
// Group them by provider and pick the headline from the Rust-supplied states.
const WORST_FIRST = ['exhausted', 'low', 'unknown', 'reset_awaiting', 'ok'];
function quotaOverviews(pools) {
  const groups = new Map();
  for (const pool of Array.isArray(pools) ? pools : []) {
    if (!groups.has(pool.provider)) groups.set(pool.provider, []);
    groups.get(pool.provider).push(pool);
  }
  return [...groups].map(([provider, windows]) => {
    const states = windows.map(stateOf);
    const state = WORST_FIRST.find(candidate => states.includes(candidate));
    const observed = windows.map(win => Date.parse(win.observed_at)).filter(Number.isFinite);
    return { provider, display_name: titleCase(provider), plan: windows.find(win => win.plan)?.plan || null, state, windows, headline_window: states.indexOf(state), updated_at: observed.length ? new Date(Math.max(...observed)).toISOString() : null };
  });
}

export function renderQuota(pools, publishedAt) { return renderProviderCards(quotaOverviews(pools), { cost: false, publishedAt }); }


export function renderSessionList(sessions, selected = '', query = '') {
  const matching = sessions.filter(session => `${session.title || ''} ${session.agent} ${session.repo_path || session.project_name || ''}`.toLowerCase().includes(query.trim().toLowerCase()));
  return matching.map(session => {
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    return `<button type="button" class="ide-session" data-action="load-chat" data-arg="${escape(session.id)}" aria-current="${session.id === selected}"><strong>${escape(session.title || session.project_name || session.repo_path || 'Untitled session')}</strong><span>${escape(session.agent)}${isSubagent ? ' · Subagent' : ''} · ${escape(sessionStateLabel(session.state))}</span>${Number.isFinite(Date.parse(session.last_event_at)) ? `<time datetime="${escape(session.last_event_at)}">${escape(relativeActivity(session.last_event_at))}</time>` : ''}</button>`;
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
  if (!limits.length) return '<span>Limits unavailable</span>';
  return limits.map(limit => `<span class="sp-chip sp-chip--${CHIP_CLASS[stateOf(limit)]} sp-conversation-limit" title="${escape(meterTitle(limit))}">${escape(limit.label)} ${escape(valueText(limit))}</span>`).join('');
}
