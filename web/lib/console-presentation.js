import MarkdownIt from "./markdown-it.mjs";
import { PROVIDER_LOGOS } from "./provider-logos.js";

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

// ---- Project activity ----
// Rust supplies each project (name, folder, session count, last activity, agents). This only lays them out:
// newest first, ten rows, a filter once there are more than ten.
const PROJECT_TOP = 10;
const PROJECT_HELPER = 'Most recent first.';
const FOLDER_ICON = '<svg class="sp-project-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M3 7.5A1.5 1.5 0 0 1 4.5 6H9l2 2.5h8.5A1.5 1.5 0 0 1 21 10v8.5a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 18.5z"/></svg>';
const AGENT_NAMES = { 'claude-code': 'Claude Code', codex: 'Codex', antigravity: 'Antigravity' };
// Agents removed in 1.0 (E4) keep their stored rows; they show as unsupported.
const agentLabel = id => AGENT_NAMES[id] || 'Unsupported agent';
const projectTime = project => { const time = Date.parse(project.last_active); return Number.isFinite(time) ? time : -Infinity; };

/** The folder a project lives in, shortened for one line: home becomes ~, then at most the last three folders. */
export function shortPath(path) {
  const parts = String(path ?? '').replace(/\\/g, '/').replace(/\/+/g, '/').replace(/\/$/, '').split('/').filter(Boolean);
  if (parts.length < 2) return '';
  parts.pop();
  const home = /^[A-Za-z]:$/.test(parts[0]) ? 1 : 0;
  if (/^(users|home)$/i.test(parts[home] ?? '') && parts.length > home + 1) parts.splice(0, home + 2, '~');
  return parts.length > 3 ? '.../' + parts.slice(-3).join('/') : parts.join('/');
}

/** Projects from a synced session list (the cloud snapshot carries no folder paths). Sessions without a project are skipped. */
export function projectsFromSessions(sessions) {
  const groups = new Map();
  for (const session of Array.isArray(sessions) ? sessions : []) {
    const name = String(session?.project_name ?? '').trim();
    if (!name || name.toLowerCase() === 'unknown') continue;
    const group = groups.get(name) || { id: name, name, sessions: 0, last_active: null, agents: [] };
    group.sessions += 1;
    if (projectTime({ last_active: session.last_event_at }) > projectTime(group)) group.last_active = session.last_event_at;
    if (session.agent && !group.agents.includes(session.agent)) group.agents.push(session.agent);
    groups.set(name, group);
  }
  return [...groups.values()];
}

const listedProjects = projects => (Array.isArray(projects) ? projects : [])
  .filter(project => project?.name && String(project.name).toLowerCase() !== 'unknown')
  .sort((a, b) => projectTime(b) - projectTime(a) || String(a.name).localeCompare(String(b.name)));

function projectRow(project) {
  const path = shortPath(project.repo_root);
  const agents = (Array.isArray(project.agents) ? project.agents : []).map(agentLabel).join(', ');
  const sessions = Number.isFinite(project.sessions) ? project.sessions : 0;
  const at = new Date(project.last_active);
  const known = Number.isFinite(at.getTime());
  const second = path || agents;
  // Local rollups also carry recorded usage; it stays in the tooltip, with its price coverage, to keep the row to one line.
  const usage = Number.isFinite(project.total_tokens) ? `\n${tokens(project.total_tokens)} recorded tokens\n${cost(project, project.cost_usd)} · ${priced(project) ? coverage(project) : 'Price unavailable'}` : '';
  return `<button type="button" class="sp-project-row" data-project-id="${escape(project.id)}" title="${escape((project.repo_root || project.name) + usage)}">${FOLDER_ICON}<span class="sp-project-main"><strong>${escape(project.name)}</strong>${second ? `<span>${escape(second)}</span>` : ''}</span><span class="sp-project-agents">${path ? escape(agents) : ''}</span><span class="sp-project-count"><b>${sessions}</b><span> ${sessions === 1 ? 'session' : 'sessions'}</span></span><time class="sp-project-time" datetime="${known ? escape(project.last_active) : ''}" title="${known ? escape(at.toLocaleString()) : 'Unknown'}">${known ? escape(shortAge(project.last_active)) : '-'}</time>${CHEVRON_RIGHT}</button>`;
}

/** The rows and the Show all control; `query` filters by name or folder and always shows every match. */
export function renderProjectRows(projects, { query = '', expanded = false } = {}) {
  const all = listedProjects(projects);
  if (!all.length) return '<div class="sp-empty"><p>No project activity recorded yet.</p></div>';
  const needle = String(query).trim().toLowerCase();
  const matches = needle ? all.filter(project => `${project.name} ${project.repo_root || ''}`.toLowerCase().includes(needle)) : all;
  if (!matches.length) return `<div class="sp-empty"><p>No project matches "${escape(String(query).trim())}".</p></div>`;
  const shown = needle || expanded ? matches : matches.slice(0, PROJECT_TOP);
  const more = !needle && all.length > PROJECT_TOP ? `<button type="button" class="sp-link sp-project-more" aria-expanded="${expanded}">${expanded ? 'Show fewer' : `Show all ${all.length}`}</button>` : '';
  return `<div class="sp-project-list">${shown.map(projectRow).join('')}</div>${more}`;
}

export function renderProjects(projects, { query = '', expanded = false, helper = PROJECT_HELPER, heading = true } = {}) {
  if (!Array.isArray(projects)) return '<section class="sp-section sp-projects"><header class="sp-section-head"><h2>Project activity</h2></header><p class="sp-project-helper">Project rollups remain local. This cloud source does not publish repository identity or project totals.</p></section>';
  const search = listedProjects(projects).length > PROJECT_TOP ? `<input type="search" class="sp-project-search" aria-label="Filter projects" placeholder="Filter projects" autocomplete="off" value="${escape(query)}">` : '';
  return `<section class="sp-section sp-projects" aria-label="Project activity">${heading ? '<header class="sp-section-head"><h2>Project activity</h2></header>' : ''}<div class="sp-project-tools"><p class="sp-project-helper">${escape(helper)}</p>${search}</div><div class="sp-project-rows">${renderProjectRows(projects, { query, expanded })}</div></section>`;
}

const projectViews = new WeakMap();

/** Renders the list into `element` and keeps the filter text and Show all choice across refreshes. Rows carry `data-project-id`; opening one is the caller's job. */
export function mountProjectList(element, projects, options = {}) {
  let view = projectViews.get(element);
  if (!view) {
    view = { query: '', expanded: false };
    projectViews.set(element, view);
    element.addEventListener('input', event => {
      if (!event.target.matches?.('.sp-project-search')) return;
      view.query = event.target.value;
      element.querySelector('.sp-project-rows').innerHTML = renderProjectRows(view.projects, view);
    });
    element.addEventListener('click', event => {
      if (!event.target.closest?.('.sp-project-more')) return;
      view.expanded = !view.expanded;
      element.querySelector('.sp-project-rows').innerHTML = renderProjectRows(view.projects, view);
      element.querySelector('.sp-project-more')?.focus({ preventScroll: true });
    });
  }
  view.projects = projects;
  const markup = renderProjects(projects, { ...options, query: view.query, expanded: view.expanded });
  if (view.markup === markup && element.querySelector('.sp-projects')) return;
  const typing = element.querySelector('.sp-project-search') === document.activeElement;
  if (typing && Array.isArray(projects)) {
    element.querySelector('.sp-project-rows').innerHTML = renderProjectRows(projects, view);
    view.markup = null;
    return;
  }
  element.innerHTML = markup;
  view.markup = markup;
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
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.title || session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}${isSubagent ? ' · Subagent' : ''}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.parent_session_id ? `<dt>Parent session</dt><dd>${escape(session.parent_session_id)}</dd>` : ''}${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input</dt><dd>${detail(session.input_tokens)}</dd><dt>Output</dt><dd>${detail(session.output_tokens)}</dd><dt>Cache read</dt><dd>${detail(session.cache_read_tokens)}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button><button type="button" data-action="load-chat" data-arg="${escape(session.id)}">View chat</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><div style="display:flex;align-items:center;justify-content:space-between;gap:8px;"><div><strong>${escape(session.agent)}</strong><span>${session.is_ongoing ? 'Ongoing · ' : ''}${escape(sessionStateLabel(session.state))}</span>${session.state === 'ended' ? '<small>Session ended</small>' : ''}</div><button type="button" class="btn-control" style="padding:2px 8px;font-size:var(--sp-fs-meta);" data-action="load-chat" data-arg="${escape(session.id)}">Chat</button></div></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong></td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><small>Window ${escape(tokens(session.context_window))} · ${Number.isFinite(session.context_window) ? 'estimated capacity' : 'capacity unknown'}</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% estimated</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><small>${knownCost && session.unpriced_turns > 0 ? 'Partial estimate' : ''}</small></td><td><time datetime="${escape(session.last_event_at || '')}" title="${Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}">${illustrative ? 'Sample' : escape(relativeActivity(session.last_event_at))}</time><small>${illustrative ? '' : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'Unknown'}</small></td></tr>`;
  }).join('')}</tbody></table>`;
}

// `totals: false` drops the token total when the page already shows it elsewhere (the local Usage grid).
export function renderActivity(report, { totals = true } = {}) {
  if (!Array.isArray(report?.daily)) return '<section class="sp-activity"><h2>Activity</h2><p>Measurements are unavailable. Update the paired desktop app to publish this view.</p></section>';
  if (!report.daily.length) return '<section class="sp-activity"><h2>Activity</h2><p>No recorded usage in this period. Unobserved intervals are not counted as zero.</p></section>';
  const hours = report.range_label === 'Today';
  const interval = report.activity_interval || (hours ? 'hour' : 'day');
  const days = report.daily.slice(-84);
  const chartDate = value => new Date(value).toLocaleDateString('en-US', {month:'short',day:'numeric',year:'numeric',...(hours ? {} : {timeZone:'UTC'})});
  const chartWindow = `${chartDate(report.earliest_at || days[0].date)} – ${chartDate(report.latest_at || days.at(-1).date)}`;
  const weekLabel = value => { const date=new Date(value+'T00:00:00Z'); date.setUTCDate(date.getUTCDate()+3); const year=date.getUTCFullYear(); const first=new Date(Date.UTC(year,0,4)); const firstMonday=new Date(first); firstMonday.setUTCDate(first.getUTCDate()-((first.getUTCDay()+6)%7)); return year+' · Week '+(Math.floor((date-firstMonday)/604800000)+1); };
  const label = day => interval === 'week' ? weekLabel(day.date) : interval === 'year' ? day.date.slice(0,4) : interval === 'month' ? new Date(day.date).toLocaleDateString('en-US',{month:'short',year:'numeric',timeZone:'UTC'}) : hours ? new Date(day.date).toLocaleTimeString([], {hour:'2-digit', minute:'2-digit', hour12:false}) : day.date.slice(5);
  return `<section class="sp-activity${totals || hours ? '' : ' sp-activity--merged'}" aria-label="Recorded token activity">
    ${totals || hours ? `<div class="sp-activity-heading"><div><h2>${hours ? 'Today, by hour' : 'Recorded activity'}</h2><p>${hours ? 'Since local midnight · hours in your local time' : `${escape(report.range_label || 'Selected period')} · ${interval === 'week' ? 'weekly' : interval === 'month' ? 'monthly' : interval === 'year' ? 'yearly' : 'daily'} totals · days in UTC`}</p></div>${totals ? `<div class="sp-activity-totals"><div><strong>${escape(Number.isFinite(report.total_tokens) ? new Intl.NumberFormat('en', {notation:'compact', compactDisplay:'long', maximumFractionDigits:1}).format(report.total_tokens) : 'Unknown')}</strong><span class="sp-data-unit">Recorded tokens</span></div></div>` : ''}</div>
    <p class="sp-activity-caption">${escape(chartWindow)}</p>` : `<p class="sp-activity-note">${escape(report.range_label || 'Selected period')} · ${interval === 'week' ? 'weekly' : interval === 'month' ? 'monthly' : interval === 'year' ? 'yearly' : 'daily'} totals · ${escape(chartWindow)} · days in UTC</p>`}
    ${hours ? renderHourStrip(report) : ''}
    <details class="sp-activity-breakdown"><summary>Activity details</summary><p>Recorded totals; gaps are not counted as zero. Costs are estimated. ${days.length < report.daily.length ? 'Showing the latest 84 intervals; the total covers the full period.' : ''}</p><div class="sp-activity-grid">${days.map(day => `<details data-date="${escape(day.date)}"><summary><time datetime="${escape(day.date)}">${escape(label(day))}</time><strong>${escape(tokens(day.total_tokens))}</strong></summary><div><span>API equivalent</span><strong>${cost(day, day.cost_usd)}</strong><span>${priced(day) ? coverage(day) : 'Price unavailable'}</span><span>${escape(day.turns)} recorded invocations</span></div></details>`).join('')}</div>
    </details>
  </section>`;
}

const usd = value => Number.isFinite(value) ? '$' + value.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) : 'Unknown';
const multiple = value => Number.isFinite(value) ? value.toLocaleString('en-US', { minimumFractionDigits: 1, maximumFractionDigits: 1 }) + 'x' : 'Unknown';

/** Subscription value: one tile per provider plus a combined tile. Rust supplies every figure; this only formats. */
export function renderValue(byProvider = [], combined = null) {
  const rows = (Array.isArray(byProvider) ? byProvider : []).filter(row => row?.spend_usd > 0);
  const tile = (row, label, total = false) => `<div class="sp-tile sp-tile--value${total ? ' sp-tile--total' : ''}"><span class="sp-tile-label">${escape(label)}</span><strong class="sp-tile-value">${escape(multiple(row.multiplier))}<span class="sp-tile-unit">est.</span></strong><span class="sp-tile-meta">${escape(usd(row.spend_usd))} value · ${escape(usd(row.plan_usd))} plan</span></div>`;
  const total = rows.length > 1 && combined?.spend_usd > 0 ? tile(combined, 'All providers', true) : '';
  return `<header class="sp-section-head"><h2>Subscription value</h2><button type="button" class="sp-link-quiet" data-action="view:settings" data-manage-plans>Manage plans</button></header><p class="sp-section-note">API-equivalent estimate, not an invoice.</p>${rows.length ? `<div class="sp-tiles sp-tiles--value">${rows.map(row => tile(row, row.provider)).join('')}${total}</div>` : '<p class="sp-section-note">No subscription usage recorded in this period.</p>'}`;
}
export function updateValue(element, byProvider, combined) { updateMarkup(element, renderValue(byProvider, combined)); }

// ---- Activity grid (SPEC 3.16) ----
// Rust supplies every day, level and statistic. This code lays the cells out and formats dates and numbers.
const heatLevel = value => [0, 1, 2, 3, 4].includes(value) ? value : 0;
const monthDay = iso => new Date(iso + 'T00:00:00Z').toLocaleDateString('en-US', { month: 'short', day: 'numeric', timeZone: 'UTC' });
const heatFigures = day => {
  const spend = priced(day) && Number.isFinite(day.cost_usd) ? ` · $${day.cost_usd.toFixed(2)} est.` : '';
  const turns = Number.isFinite(day.turns) ? ` · ${day.turns} ${day.turns === 1 ? 'turn' : 'turns'}` : '';
  return `${tokens(day.total_tokens)} tokens${spend}${turns}`;
};
const heatEmpty = (label, text) => `<section class="sp-section sp-heatmap" aria-label="${escape(label)}"><header class="sp-section-head"><h2>Activity</h2><span class="sp-section-meta">Days in UTC</span></header><div class="sp-empty"><p>${escape(text)}</p></div></section>`;

export function renderHeatmap(data, { weeks = 53, stats = true, legend = true, months = true } = {}) {
  const label = 'Token activity, last year';
  if (!Array.isArray(data?.days)) return heatEmpty(label, 'Activity is unavailable.');
  if (!data.first_recorded || !data.days.length) return heatEmpty(label, 'No activity recorded yet.');
  let days = data.days;
  if (weeks < 53) {
    const weekday = (new Date(days.at(-1).date + 'T00:00:00Z').getUTCDay() + 6) % 7;
    days = days.slice(Math.max(0, days.length - ((weeks - 1) * 7 + weekday + 1)));
  }
  const cells = days.map(day => day.date >= data.first_recorded
    ? `<i class="sp-heat" data-level="${heatLevel(day.level)}" data-date="${escape(day.date)}" title="${escape(monthDay(day.date))}: ${day.total_tokens > 0 ? escape(heatFigures(day)) : 'no recorded activity'}"></i>`
    : '<i class="sp-heat sp-heat--before"></i>').join('');
  const marks = [];
  if (months) {
    let last = '';
    for (let index = 0; index < days.length; index += 7) {
      const month = days[index].date.slice(0, 7);
      if (days[Math.min(index + 6, days.length - 1)].date < data.first_recorded) continue;
      if (month !== last) { marks.push({ col: index / 7, text: new Date(days[index].date + 'T00:00:00Z').toLocaleDateString('en-US', { month: 'short', timeZone: 'UTC' }) }); last = month; }
    }
  }
  // Drop a label that would collide with the next one (a partial first month).
  const labels = marks.filter((mark, index) => !marks[index + 1] || marks[index + 1].col - mark.col >= 3);
  const summary = data.stats;
  const busiest = summary?.busiest;
  const streakTitle = Number.isFinite(summary?.longest_streak) ? ` title="Longest streak: ${escape(summary.longest_streak)} days"` : '';
  const figures = stats && summary ? `<div class="sp-heatmap-stats"><div><strong>${escape(tokens(summary.total_tokens))}</strong><span>tokens, past year</span></div><div><strong>${escape(summary.active_days ?? 'Unknown')}</strong><span>active days</span></div><div${streakTitle}><strong>${escape(summary.current_streak ?? 'Unknown')}</strong><span>day streak</span></div><div${busiest ? ` title="${escape(tokens(busiest.total_tokens))} tokens"` : ''}><strong>${busiest ? escape(monthDay(busiest.date)) : 'None'}</strong><span>busiest day</span></div></div>` : '';
  const description = summary ? `${tokens(summary.total_tokens)} tokens over ${summary.active_days} active days. Details below.` : 'Token activity by day.';
  const key = legend ? '<div class="sp-heatmap-legend" aria-hidden="true"><span>Less</span><i class="sp-heat" data-level="0"></i><i class="sp-heat" data-level="1"></i><i class="sp-heat" data-level="2"></i><i class="sp-heat" data-level="3"></i><i class="sp-heat" data-level="4"></i><span>More</span></div>' : '';
  return `<section class="sp-section sp-heatmap${stats ? '' : ' sp-heatmap--mini'}" aria-label="${label}"><header class="sp-section-head"><h2>${weeks < 53 ? `Last ${weeks} weeks` : 'Activity'}</h2><span class="sp-section-meta">Days in UTC</span></header><div class="sp-card sp-heatmap-card">${figures}<div class="sp-heatmap-scroll" tabindex="0" role="group" aria-label="Activity grid, scrollable">${labels.length ? `<div class="sp-heatmap-months" aria-hidden="true">${labels.map(mark => `<span style="--col:${mark.col}">${escape(mark.text)}</span>`).join('')}</div>` : ''}<div class="sp-heatmap-body"><div class="sp-heatmap-days" aria-hidden="true"><span>Mon</span><span></span><span>Wed</span><span></span><span>Fri</span><span></span><span></span></div><div class="sp-heatmap-grid" role="img" aria-label="${escape(description)}">${cells}</div></div></div>${key}</div></section>`;
}

export function renderHourStrip(report) {
  const rows = Array.isArray(report?.daily) ? report.daily : [];
  const byHour = new Map(rows.map(row => [new Date(row.date).getHours(), row]));
  const cells = Array.from({ length: 24 }, (_, hour) => {
    const row = byHour.get(hour);
    const name = `${String(hour).padStart(2, '0')}:00`;
    return `<i class="sp-heat" data-level="${row ? heatLevel(row.level) : 0}" title="${name}: ${row && row.total_tokens > 0 ? escape(heatFigures(row)) : 'no recorded activity'}"></i>`;
  }).join('');
  return `<div class="sp-heatmap sp-heatmap--hours"><div class="sp-heatstrip" role="img" aria-label="Token activity by local hour">${cells}</div><div class="sp-heatstrip-hours" aria-hidden="true"><span>00</span><span>06</span><span>12</span><span>18</span></div></div>`;
}

const heatmapMarkup = new WeakMap();
export function updateHeatmap(element, data, options) {
  const markup = renderHeatmap(data, options);
  if (heatmapMarkup.get(element) === markup) return;
  const scroller = element.querySelector('.sp-heatmap-scroll');
  const kept = scroller ? scroller.scrollLeft : null;
  element.innerHTML = markup;
  const next = element.querySelector('.sp-heatmap-scroll');
  if (next) next.scrollLeft = kept === null ? next.scrollWidth : kept;
  heatmapMarkup.set(element, markup);
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

const HOME_GROUPS = [['needs_you', 'Needs you'], ['running', 'Running'], ['recent', 'Recent']];
const HOME_RECENT_MAX = 6;
const CHEVRON_RIGHT = '<svg class="sp-session-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 6 6 6-6 6"/></svg>';
const shortAge = value => { const date = new Date(value); if (!Number.isFinite(date.getTime())) return ''; const age = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000)); return age < 60 ? 'now' : age < 3600 ? Math.floor(age / 60) + 'm' : age < 86400 ? Math.floor(age / 3600) + 'h' : Math.floor(age / 86400) + 'd'; };

const SUBAGENT_ROWS_MAX = 5;
const subagentOpen = new Set();
let subagentToggleBound = false;
// Re-rendered lists replace their markup, so remember which subagent groups the user expanded.
function bindSubagentToggle() {
  if (subagentToggleBound || typeof document === 'undefined') return;
  subagentToggleBound = true;
  document.addEventListener('toggle', event => {
    const element = event.target;
    if (!element?.classList?.contains('sp-subagents')) return;
    if (element.open) subagentOpen.add(element.dataset.key); else subagentOpen.delete(element.dataset.key);
  }, true);
}

export function renderSubagents(session, action = 'load-chat') {
  const list = Array.isArray(session?.subagents) ? session.subagents : [];
  const count = Number.isFinite(session?.subagent_count) ? session.subagent_count : list.length;
  if (!count) return '';
  bindSubagentToggle();
  const isLive = child => Boolean(child.is_ongoing ?? child.ongoing);
  const live = Number.isFinite(session.subagents_ongoing) ? session.subagents_ongoing : list.filter(isLive).length;
  const key = `sub:${session.id}`;
  const shown = list.slice(0, SUBAGENT_ROWS_MAX);
  const rows = shown.map(child => {
    const group = child.home_group || (isLive(child) ? 'running' : 'recent');
    const state = child.presence === 'recently_idle' ? 'Idle' : child.state === 'unknown' ? 'No recent activity' : sessionStateLabel(child.state);
    const meta = [state, child.model && child.model !== 'Unspecified' ? child.model : ''].filter(Boolean).join(' · ');
    return `<li><button type="button" class="sp-subagent-row" data-action="${action}" data-arg="${escape(child.id)}"><span class="sp-session-dot sp-session-dot--${group}" aria-hidden="true"></span><span class="sp-session-main"><strong>${escape(child.title && child.title.toLowerCase() !== 'unknown' ? child.title : 'Subagent')}</strong><span>${escape(meta)}</span></span><time class="sp-session-time" datetime="${escape(child.last_event_at || '')}">${escape(shortAge(child.last_event_at))}</time></button></li>`;
  }).join('');
  const more = count > shown.length ? `<li class="sp-subagent-more"><button type="button" class="sp-link sp-subagent-more-btn" data-action="open-all-chats">View ${count - shown.length} more in All chats &#8250;</button></li>` : '';
  return `<details class="sp-subagents" data-key="${escape(key)}"${subagentOpen.has(key) ? ' open' : ''}><summary>${count} subagent${count === 1 ? '' : 's'}${live ? ` · ${live} active` : ''}</summary><ul>${rows}${more}</ul></details>`;
}

function homeRow(session, group) {
  const project = session.project_name && session.project_name.toLowerCase() !== 'unknown' ? session.project_name : 'Private project';
  const title = session.title && session.title.toLowerCase() !== 'unknown' ? session.title : project === 'Private project' ? 'Private session' : project;
  const fill = Number.isFinite(session.context_fill_pct) ? Math.round(session.context_fill_pct) : null;
  const subagentBadge = session.subagent_count > 0 ? `${session.subagent_count} subagent${session.subagent_count === 1 ? '' : 's'}` : '';
  const meta = [project, session.surface_label || agentLabel(session.agent), session.model && session.model !== 'Unspecified' ? session.model : '', subagentBadge].filter(Boolean).join(' · ');
  const context = fill === null ? '<span class="sp-session-context" aria-hidden="true"></span>' : `<span class="sp-session-context" title="Context ${fill}% full"><i style="--fill:${fill}%"></i></span>`;
  return `<button type="button" class="sp-session-row" data-action="open-home-session" data-arg="${escape(session.id)}"><span class="sp-session-dot sp-session-dot--${group}" aria-hidden="true"></span><span class="sp-session-main"><strong>${escape(title)}</strong><span>${escape(meta)}</span></span>${context}<time class="sp-session-time" datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">${escape(shortAge(session.last_event_at))}</time>${CHEVRON_RIGHT}</button>`;
}

export function renderHomeSessions(sessions) {
  const head = '<header class="sp-section-head"><h2>Sessions</h2><span class="sp-section-meta"><button type="button" class="sp-link" data-action="view:sessions">View all</button></span></header>';
  if (!Array.isArray(sessions)) return `<section class="sp-section" aria-label="Sessions"><header class="sp-section-head"><h2>Sessions</h2></header><div class="sp-empty"><p>Sessions are unavailable.</p></div></section>`;
  if (!sessions.length) return '<section class="sp-section" aria-label="Sessions"><header class="sp-section-head"><h2>Sessions</h2></header><div class="sp-empty"><p>No sessions yet.</p><button type="button" class="btn-primary" data-action="home-new-session">New session</button></div></section>';
  const groups = HOME_GROUPS.map(([key, label]) => {
    const all = sessions.filter(session => (HOME_GROUPS.some(([name]) => name === session.home_group) ? session.home_group : 'recent') === key);
    const rows = key === 'recent' ? all.slice(0, HOME_RECENT_MAX) : all;
    return rows.length ? `<h3 class="sp-group-label">${label}${key === 'recent' ? '' : ` <span>(${all.length})</span>`}</h3><div class="sp-session-list">${rows.map(session => homeRow(session, key) + renderSubagents(session, 'open-home-session')).join('')}</div>` : '';
  }).join('');
  return `<section class="sp-section" aria-label="Sessions">${head}${groups}</section>`;
}

export function updateHomeSessions(element, sessions) {
  const markup = renderHomeSessions(sessions);
  if (homeMarkup.get(element) === markup) return;
  const focused = element.contains(document.activeElement) ? document.activeElement.dataset.arg : null;
  element.innerHTML = markup;
  if (focused) [...element.querySelectorAll('[data-arg]')].find(item => item.dataset.arg === focused)?.focus({ preventScroll: true });
  homeMarkup.set(element, markup);
}

export function renderTodayTiles({ report = null, activeCount = null } = {}) {
  const tile = (label, value, meta = '') => `<div class="sp-tile"><span class="sp-tile-label">${label}</span><strong class="sp-tile-value">${escape(value)}</strong>${meta ? `<span class="sp-tile-meta">${escape(meta)}</span>` : ''}</div>`;
  return `<header class="sp-section-head"><h2>Today</h2></header><div class="sp-tiles">${
    tile('Tokens today', Number.isFinite(report?.total_tokens) ? tokens(report.total_tokens) : 'Unknown') +
    tile('Est. spend today', cost(report, report?.total_cost_usd), priced(report) ? (report.unpriced_turns > 0 ? 'Partial estimate' : 'API equivalent') : 'Price unavailable') +
    tile('Active sessions', Number.isFinite(activeCount) ? String(activeCount) : 'Unknown')
  }</div>`;
}

/** Totals for the selected period on the web Usage page: tokens, estimated spend and cache savings. */
export function renderPeriodTiles(report = null) {
  const tile = (label, value, meta = '') => `<div class="sp-tile"><span class="sp-tile-label">${label}</span><strong class="sp-tile-value">${escape(value)}</strong>${meta ? `<span class="sp-tile-meta">${escape(meta)}</span>` : ''}</div>`;
  const saved = priced(report) && Number.isFinite(report?.total_cache_savings_usd) ? '$' + report.total_cache_savings_usd.toFixed(2) : 'Unknown';
  return `<header class="sp-section-head"><h2>Totals for ${escape(period(report))}</h2></header><div class="sp-tiles">${
    tile('Tokens', Number.isFinite(report?.total_tokens) ? tokens(report.total_tokens) : 'Unknown') +
    tile('Est. spend', cost(report, report?.total_cost_usd), priced(report) ? (report.unpriced_turns > 0 ? 'Partial estimate' : 'API equivalent') : 'Price unavailable') +
    tile('Cache saved', saved, 'Estimated')
  }</div>`;
}

const homeMarkup = new WeakMap();
export function updatePeriodTiles(element, report) {
  const markup = renderPeriodTiles(report);
  if (homeMarkup.get(element) === markup) return;
  element.innerHTML = markup;
  homeMarkup.set(element, markup);
}
export function updateTodayTiles(element, data) {
  const markup = renderTodayTiles(data);
  if (homeMarkup.get(element) === markup) return;
  element.innerHTML = markup;
  homeMarkup.set(element, markup);
}

const activityMarkup = new WeakMap();
export function updateActivity(element, report, options) {
  const markup = renderActivity(report, options);
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
  element.innerHTML = markup;
  element.querySelectorAll('strong').forEach((value, index) => {
    if (previousValues[index] !== undefined && previousValues[index] !== value.textContent) value.classList.add('sp-value-changed');
  });
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
// No Rust state (older desktop or synced payload): a window with a reading renders neutral ("plain"), not as No data.
const stateOf = value => WINDOW_STATES.includes(value?.state) ? value.state : Number.isFinite(value?.remaining_pct) ? 'plain' : 'unknown';
// Payloads synced from an older desktop have no state but still say whether the reset has passed.
const awaiting = win => stateOf(win) === 'reset_awaiting' || win.reset_passed === true;
const CHIP_TEXT = { ok: 'OK', low: 'Low', exhausted: 'Out', reset_awaiting: 'Resetting', unknown: 'No data' };
const CHIP_CLASS = { ok: 'ok', low: 'low', exhausted: 'exhausted', reset_awaiting: 'unknown', unknown: 'unknown', plain: 'neutral' };
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
  for (const win of withPools(windows)) {
    const key = win.pool || '';
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(win);
  }
  return `<div class="sp-windows">${[...groups].map(([pool, list]) => `<div class="sp-pool">${pool ? `<p class="sp-pool-name">${escape(pool)}</p>` : ''}${list.map(renderMiniRow).join('')}</div>`).join('')}</div>`;
}

const CHEVRON = '<svg class="sp-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>';
const providerName = p => p.display_name || titleCase(p.provider);
const providerMark = p => {
  const path = Object.hasOwn(PROVIDER_LOGOS, p.provider) ? PROVIDER_LOGOS[p.provider] : null;
  const glyph = path ? `<svg viewBox="0 0 24 24" width="18" height="18" fill="currentColor" focusable="false"><path d="${path}"/></svg>` : escape(Array.from(providerName(p))[0]?.toUpperCase() || '?');
  return `<span class="sp-provider-mark${path ? ' sp-provider-mark--logo' : ''}" aria-hidden="true">${glyph}</span>`;
};
const chip = state => state === 'plain' ? '' : `<span class="sp-chip sp-chip--${CHIP_CLASS[state]}">${CHIP_TEXT[state]}</span>`;
// Multi-limit providers (Antigravity) without a Rust pool name: group by limit_id. Formatting only.
function withPools(windows) {
  if (windows.some(win => win.pool) || new Set(windows.map(win => win.limit_id).filter(Boolean)).size < 2) return windows;
  return windows.map(win => win.limit_id ? { ...win, pool: titleCase(win.limit_id) } : win);
}
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
  const windows = Array.isArray(p?.windows) ? withPools(p.windows) : [];
  if (!windows.length) return emptyCard(p || {});
  const state = WINDOW_STATES.includes(p.state) ? p.state : windows.every(win => stateOf(win) === 'plain') ? 'plain' : 'unknown';
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

/** Settings > Providers: one row per provider with its limit source (text from Rust) and, when off, the enable command. */
export function renderProviderSources(list) {
  const items = Array.isArray(list) ? list : [];
  if (!items.length) return '<p class="sp-provider-empty">No providers reported.</p>';
  return `<ul class="sp-source-list">${items.map(p => `<li><span class="sp-provider-name">${escape(providerName(p))}</span><span class="sp-source-label">${escape(p.source_label || 'Unknown')}</span>${p.enable_command ? `<button type="button" class="sp-copy" data-copy="${escape(p.enable_command)}"><code>${escape(p.enable_command)}</code><span>Copy</span></button>` : ''}</li>`).join('')}</ul>`;
}

export function updateProviderSources(element, list) { updateMarkup(element, renderProviderSources(list)); }

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
const WORST_FIRST = ['exhausted', 'low', 'unknown', 'reset_awaiting', 'ok', 'plain'];
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
  // Headings use the Rust-derived home_group; with nothing waiting or running the list stays flat.
  const grouped = matching.some(session => session.home_group === 'needs_you' || session.home_group === 'running');
  const rank = session => Math.max(0, HOME_GROUPS.findIndex(([name]) => name === session.home_group));
  const ordered = grouped ? [...matching].sort((a, b) => rank(a) - rank(b)) : matching;
  let lastGroup = '';
  return ordered.map(session => {
    const isSubagent = Boolean(session.parent_session_id || session.title?.toLowerCase().includes('subagent'));
    const group = HOME_GROUPS.some(([name]) => name === session.home_group) ? session.home_group : 'recent';
    const heading = grouped && group !== lastGroup ? `<h3 class="sp-group-label">${HOME_GROUPS.find(([name]) => name === group)[1]}</h3>` : '';
    lastGroup = group;
    return `${heading}<button type="button" class="ide-session" data-action="load-chat" data-arg="${escape(session.id)}" aria-current="${session.id === selected}"><strong>${escape(session.title || session.project_name || session.repo_path || 'Untitled session')}</strong><span>${escape(session.surface_label || session.agent)}${isSubagent ? ' · Subagent' : ''} · ${escape(sessionStateLabel(session.state))}</span>${Number.isFinite(Date.parse(session.last_event_at)) ? `<time datetime="${escape(session.last_event_at)}">${escape(relativeActivity(session.last_event_at))}</time>` : ''}</button>${renderSubagents(session, 'load-chat')}`;
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

/** The limits sheet in Remote: one collapsed provider card for the conversation's agent. Windows come from Rust as sent. */
export function renderConversationLimitCard(limits = [], agent = '') {
  if (!Array.isArray(limits) || !limits.length) return '<p class="sp-provider-empty">Limits unavailable</p>';
  const windows = limits.map(({ label, ...win }) => ({ ...win, provider: agent }));
  return `<div class="sp-card-grid">${renderProviderCard(quotaOverviews(windows)[0], { cost: false })}</div>`;
}

export function renderConversationLimits(limits = []) {
  if (!limits.length) return '<span>Limits unavailable</span>';
  return limits.map(limit => `<span class="sp-chip sp-chip--${CHIP_CLASS[stateOf(limit)]} sp-conversation-limit" title="${escape(meterTitle(limit))}">${escape(limit.label)} ${escape(valueText(limit))}</span>`).join('');
}

export const MAX_QUICK_ACTIONS = 20;

export const QUICK_ACTIONS_STORAGE_KEY = 'gnomcode-quick-actions-v1';

export const DEFAULT_QUICK_ACTIONS = Object.freeze([
  Object.freeze({ id: 'continue', label: 'Continue', prompt: 'Continue where you left off and complete the current step.', builtin: true }),
  Object.freeze({ id: 'summarize-changes', label: 'Summarize changes', prompt: 'Summarize the changes made so far and list any remaining steps.', builtin: true }),
  Object.freeze({ id: 'run-tests', label: 'Run tests', prompt: 'Run the relevant test suite and summarize any failures.', builtin: true }),
  Object.freeze({ id: 'explain-error', label: 'Explain error', prompt: 'Explain the most recent error or failing check and propose the smallest fix.', builtin: true }),
]);

const DISALLOWED_PROMPT_CONTROL = /[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/;

export function normalizeQuickAction(item) {
  if (!item || typeof item !== 'object') return null;
  const label = typeof item.label === 'string' ? item.label.trim().slice(0, 40) : '';
  const prompt = typeof item.prompt === 'string' ? item.prompt : '';
  if (!label || !prompt.trim()) return null;
  if (prompt.length > 4000 || prompt.trimStart().startsWith('/') || DISALLOWED_PROMPT_CONTROL.test(prompt)) return null;
  const rawId = typeof item.id === 'string' && item.id.trim()
    ? item.id.trim()
    : label.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'custom';
  return { id: rawId.replace(/[^A-Za-z0-9_-]/g, '-').slice(0, 48), label, prompt, builtin: Boolean(item.builtin) };
}

export function mergeQuickActions(items) {
  if (!Array.isArray(items) || !items.length) return DEFAULT_QUICK_ACTIONS.map(item => ({ ...item }));
  const seen = new Set();
  const normalized = [];
  for (const item of items) {
    const clean = normalizeQuickAction(item);
    if (!clean || seen.has(clean.id)) continue;
    seen.add(clean.id);
    normalized.push(clean);
    if (normalized.length >= MAX_QUICK_ACTIONS) break;
  }
  return normalized.length ? normalized : DEFAULT_QUICK_ACTIONS.map(item => ({ ...item }));
}

/** Daemon wire shape: only id, label and prompt (the daemon rejects unknown fields). */
export function toWireQuickActions(items) {
  return mergeQuickActions(items).map(({ id, label, prompt }) => ({ id, label, prompt }));
}

export function loadStoredQuickActions(storage = typeof localStorage !== 'undefined' ? localStorage : null) {
  try {
    const raw = storage?.getItem(QUICK_ACTIONS_STORAGE_KEY);
    return raw ? mergeQuickActions(JSON.parse(raw)) : DEFAULT_QUICK_ACTIONS.map(item => ({ ...item }));
  } catch {
    return DEFAULT_QUICK_ACTIONS.map(item => ({ ...item }));
  }
}

export function saveStoredQuickActions(items, storage = typeof localStorage !== 'undefined' ? localStorage : null) {
  const merged = mergeQuickActions(items);
  try {
    storage?.setItem(QUICK_ACTIONS_STORAGE_KEY, JSON.stringify(merged));
  } catch {}
  return merged;
}

export function renderQuickActionsStrip(actions = DEFAULT_QUICK_ACTIONS, { disabled = false } = {}) {
  const list = mergeQuickActions(actions);
  const disabledAttr = disabled ? ' disabled' : '';
  const buttons = list.map(action => `<button type="button" class="sp-quick-action" data-action="quick-action" data-arg="${escape(action.id)}" title="${escape(action.prompt)}"${disabledAttr}>${escape(action.label)}</button>`).join('');
  return `<div class="sp-quick-actions" role="toolbar" aria-label="Quick prompts">${buttons}<button type="button" class="sp-quick-action sp-quick-action--manage" data-action="manage-quick-actions" aria-label="Manage quick prompts">Customize</button></div>`;}

const STATUS_LABELS = { '??': 'Untracked', A: 'Added', M: 'Modified', D: 'Deleted', R: 'Renamed', C: 'Copied', U: 'Conflict' };

export function parseGitStatusLines(statusText = '') {
  if (typeof statusText !== 'string' || !statusText) return [];
  const rows = [];
  for (const rawLine of statusText.split(/\r?\n/)) {
    if (!rawLine || rawLine.length < 3) continue;
    const xy = rawLine.slice(0, 2);
    const trimmed = xy.trim() || 'M';
    const key = xy === '??' ? '??' : trimmed[0];
    const path = rawLine.slice(3).trim();
    if (!path) continue;
    rows.push({ code: key, label: STATUS_LABELS[key] || 'Changed', path });
  }
  return rows;
}

function classifyDiffLine(line) {
  if (line.startsWith('diff --git ') || line.startsWith('index ') || line.startsWith('+++ ') || line.startsWith('--- ')) return 'sp-diff-line sp-diff-line--meta';
  if (line.startsWith('@@')) return 'sp-diff-line sp-diff-line--hunk';
  if (line.startsWith('+')) return 'sp-diff-line sp-diff-line--add';
  if (line.startsWith('-')) return 'sp-diff-line sp-diff-line--del';
  return 'sp-diff-line';
}

export function renderChangesView(data) {
  if (!data) return '<p class="sp-provider-empty">Select a conversation workspace to inspect uncommitted changes.</p>';
  if (data.loading) return '<p class="sp-provider-empty" role="status">Loading workspace changes...</p>';
  if (data.error) return `<p class="sp-field-note sp-tone-red" role="alert">${escape(data.error)}</p>`;
  const files = parseGitStatusLines(data.status || '');
  const diffText = typeof data.diff === 'string' ? data.diff : '';
  const badges = [
    '<span class="sp-chip sp-chip--neutral">Read-only</span>',
    data.lossy ? '<span class="sp-chip sp-chip--low" title="Non-UTF-8 bytes were replaced with U+FFFD by Rust">Lossy UTF-8 decoding</span>' : '',
    data.repo ? `<span class="sp-changes-repo" title="${escape(data.repo)}">${escape(shortPath(data.repo))}</span>` : '',
  ].filter(Boolean).join('');
  if (!files.length && !diffText.trim()) {
    return `<div class="sp-changes-meta">${badges}</div><p class="sp-provider-empty">Working tree clean: no modified or untracked files.</p>`;
  }
  const fileMarkup = files.length
    ? `<ul class="sp-changes-files" aria-label="Changed files">${files.map(file => `<li class="sp-changes-file"><span class="sp-chip sp-chip--neutral">${escape(file.label)}</span><code>${escape(file.path)}</code></li>`).join('')}</ul>`
    : '';
  const diffLines = diffText.trim()
    ? diffText.split(/\r?\n/).map(line => `<span class="${classifyDiffLine(line)}">${escape(line) || ' '}</span>`).join('')
    : '';
  const diffMarkup = diffLines
    ? `<pre class="sp-diff-code" tabindex="0" aria-label="Unified git diff">${diffLines}</pre>`
    : '<p class="sp-provider-empty">No modified tracked diff hunks (untracked files listed above).</p>';
  return `<div class="sp-changes-meta">${badges}</div>${fileMarkup}${diffMarkup}`;
}

export function normalizeWorkspaceRelPath(input) {
  if (typeof input !== 'string') return { ok: false, error: 'Enter a relative file path inside the workspace.' };
  const raw = input.trim().replaceAll('\\', '/');
  if (!raw || raw.includes('\u0000')) return { ok: false, error: 'Enter a relative file path inside the workspace.' };
  if (raw.startsWith('/') || /^[a-zA-Z]:/.test(raw)) return { ok: false, error: 'Only relative workspace paths are allowed.' };
  const parts = raw.split('/').filter(segment => segment && segment !== '.');
  if (!parts.length || parts.some(segment => segment === '..')) return { ok: false, error: 'Path traversal (..) outside the workspace is blocked.' };
  return { ok: true, path: parts.join('/') };
}

/** Parent folder of a workspace-relative directory path: '' for the root's children, null at the root. */
export function parentWorkspacePath(path) {
  if (typeof path !== 'string' || !path) return null;
  const parts = path.split('/').filter(Boolean);
  return parts.length <= 1 ? '' : parts.slice(0, -1).join('/');
}

export function renderFileEntries(listing) {
  if (!listing) return '';
  if (listing.loading) return '<p class="sp-provider-empty" role="status">Loading workspace files...</p>';
  if (listing.error) return `<p class="sp-field-note sp-tone-red" role="alert">${escape(listing.error)}</p>`;
  const entries = Array.isArray(listing.entries) ? listing.entries : [];
  const cwd = listing.path ? `/${escape(listing.path)}` : '/';
  const parent = typeof listing.parent === 'string' ? listing.parent : parentWorkspacePath(listing.path);
  const up = typeof parent === 'string'
    ? `<button type="button" class="btn-control sp-file-entry sp-file-entry--dir" data-action="open-repo-dir" data-arg="${escape(parent)}">Up one folder</button>`
    : '';
  const items = entries.map(entry => {
    const isDir = entry.kind === 'dir';
    const action = isDir ? 'open-repo-dir' : 'open-repo-file';
    const meta = isDir ? 'Folder' : (Number.isFinite(entry.size) ? `${Math.max(1, Math.round(entry.size / 1024))} KB` : 'File');
    return `<button type="button" class="btn-control sp-file-entry ${isDir ? 'sp-file-entry--dir' : 'sp-file-entry--file'}" data-action="${action}" data-arg="${escape(entry.path)}"><code>${escape(entry.name)}</code><span>${escape(meta)}</span></button>`;
  }).join('');
  return `<div class="sp-file-crumb">Folder: <code>${cwd}</code></div>${listing.note ? `<p class="sp-field-note">${escape(listing.note)}</p>` : ''}<div class="sp-file-entries">${up}${items || '<p class="sp-provider-empty">No files listed here. Enter a relative path above to open a file.</p>'}</div>`;
}

export function renderFileViewer(fileData) {
  if (!fileData) return '<p class="sp-provider-empty">Select a file or enter a workspace-relative path to inspect it read-only.</p>';
  if (fileData.loading) return '<p class="sp-provider-empty" role="status">Reading file...</p>';
  if (fileData.error) return `<p class="sp-field-note sp-tone-red" role="alert">${escape(fileData.error)}</p>`;
  const content = typeof fileData.content === 'string' ? fileData.content : '';
  const badges = [
    '<span class="sp-chip sp-chip--neutral">Read-only</span>',
    fileData.lossy ? '<span class="sp-chip sp-chip--low">Lossy UTF-8 decoding</span>' : '',
    fileData.truncated ? '<span class="sp-chip sp-chip--low">Truncated at 512 KiB</span>' : '',
    fileData.path ? `<code class="sp-file-path">${escape(fileData.path)}</code>` : '',
  ].filter(Boolean).join('');
  const lines = content.split(/\r?\n/);
  const body = lines.map((line, index) => `<span class="sp-file-line"><span class="sp-file-ln">${index + 1}</span><span>${escape(line) || ' '}</span></span>`).join('');
  return `<div class="sp-changes-meta">${badges}</div><pre class="sp-file-code" tabindex="0" aria-label="Workspace file contents">${body}</pre>`;
}
const IMAGE_DATA_URL_RE = /^data:image\/(?:png|jpe?g|webp|gif|svg\+xml);base64,[A-Za-z0-9+/=]+$/i;
const IMAGE_LOCAL_API_RE = /^\/api\/v1\/[^\s"'<>]+$/;

export function classifyArtifactKind(item) {
  const explicit = String(item?.kind || '').toLowerCase();
  if (explicit === 'html' || explicit === 'markdown' || explicit === 'image') return explicit;
  const mime = String(item?.mime_type || '').toLowerCase();
  if (mime === 'text/html') return 'html';
  if (mime === 'text/markdown') return 'markdown';
  if (mime.startsWith('image/')) return 'image';
  const path = String(item?.path || item?.title || '').toLowerCase();
  if (/\.(html?|xhtml)$/.test(path)) return 'html';
  if (/\.(md|markdown|mdx)$/.test(path)) return 'markdown';
  if (/\.(png|jpe?g|webp|gif|svg)$/.test(path)) return 'image';
  return 'other';
}

export function isSafeArtifactImageSrc(src) {
  const value = String(src || '').trim();
  return IMAGE_DATA_URL_RE.test(value) || IMAGE_LOCAL_API_RE.test(value);
}

const ARTIFACT_BADGE = { html: '[HTML]', markdown: '[MD]', image: '[IMG]', other: '[FILE]' };

export function renderArtifactsList(items = [], selectedId = '') {
  if (!Array.isArray(items) || !items.length) {
    return '<p class="sp-empty">No artifacts recorded for this session yet.</p>';
  }
  return items.map((item, idx) => {
    const id = String(item?.id || item?.path || idx);
    const kind = classifyArtifactKind(item);
    const label = item?.title || item?.path || `Artifact ${idx + 1}`;
    const selected = id === String(selectedId);
    const status = item?.status ? `<span class="sp-artifact-status">${escape(item.status)}</span>` : '';
    return `<button type="button" class="sp-artifact-item${selected ? ' is-selected' : ''}" data-artifact-id="${escape(id)}" aria-pressed="${selected ? 'true' : 'false'}"><span class="sp-artifact-badge">${ARTIFACT_BADGE[kind] || '[FILE]'}</span><span class="sp-artifact-name">${escape(label)}</span>${status}</button>`;
  }).join('');
}

// The frame is sandboxed with no permissions; the CSP additionally blocks every network fetch.
const ARTIFACT_HTML_CSP = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'">`;

export function renderArtifactPreview(artifact) {
  if (!artifact || typeof artifact !== 'object') {
    return '<p class="sp-empty">Select an artifact to preview.</p>';
  }
  const kind = classifyArtifactKind(artifact);
  const title = String(artifact.title || artifact.path || 'Artifact preview');
  if (kind === 'html') {
    return `<div class="sp-artifact-html-wrap"><p class="sp-artifact-meta">Sandboxed preview (scripts disabled)</p><iframe class="sp-artifact-frame" title="${escape(title)}" sandbox="" referrerpolicy="no-referrer" srcdoc="${escape(ARTIFACT_HTML_CSP + String(artifact.content || ''))}"></iframe></div>`;
  }
  if (kind === 'markdown') {
    return `<div class="sp-artifact-markdown sp-chat-body">${chatMarkdown(String(artifact.content || ''))}</div>`;
  }
  if (kind === 'image') {
    const src = artifact.data_url || artifact.url || '';
    if (!isSafeArtifactImageSrc(src)) {
      return '<p class="sp-empty">Image preview unavailable for untrusted or empty source.</p>';
    }
    return `<div class="sp-artifact-image-wrap"><img class="sp-artifact-image" src="${escape(src)}" alt="${escape(title)}" /></div>`;
  }
  return `<pre class="sp-artifact-code"><code>${escape(String(artifact.content || 'Preview unavailable.'))}</code></pre>`;
}
export const MAX_ATTACHMENT_BYTES = 15 * 1024 * 1024;
const ALLOWED_ATTACHMENT_EXTS = new Set(['.png', '.jpg', '.jpeg', '.webp', '.gif', '.txt', '.md', '.log', '.json', '.csv', '.pdf']);

function sanitizeAttachmentName(name) {
  const base = String(name || '').split(/[/\\]/).pop() || '';
  const cleaned = base.replace(/[\u0000-\u001f\u007f]+/g, '').replace(/\.\.+/g, '.').trim();
  return cleaned.slice(0, 120) || 'attachment';
}

export function validateAttachmentCandidate(file) {
  if (!file || typeof file.size !== 'number' || file.size <= 0) {
    return { ok: false, reason: 'Choose a non-empty file to attach.' };
  }
  if (file.size > MAX_ATTACHMENT_BYTES) {
    return { ok: false, reason: 'Attachments must be 15 MiB or smaller.' };
  }
  const filename = sanitizeAttachmentName(file.name);
  const dot = filename.lastIndexOf('.');
  const ext = dot >= 0 ? filename.slice(dot).toLowerCase() : '';
  if (!ALLOWED_ATTACHMENT_EXTS.has(ext)) {
    return { ok: false, reason: 'Unsupported file type. Use PNG, JPG, WEBP, GIF, PDF, TXT, MD, LOG, JSON, or CSV.' };
  }
  const mime_type = String(file.type || '').trim() || 'application/octet-stream';
  return { ok: true, filename, size: file.size, mime_type };
}

function formatAttachmentBytes(size) {
  if (!Number.isFinite(size) || size < 1024) return `${Math.max(0, Number(size) || 0)} B`;
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
  return `${(size / (1024 * 1024)).toFixed(1)} MB`;
}

export function renderAttachmentPills(items = []) {
  if (!Array.isArray(items) || !items.length) return '';
  return items.map(item => {
    const id = escape(String(item?.id || ''));
    const name = escape(String(item?.filename || 'attachment'));
    const size = escape(formatAttachmentBytes(item?.size));
    const statusLabel = item?.status === 'saved' ? 'Saved' : item?.status === 'error' ? 'Failed' : 'Uploading';
    return `<span class="sp-attachment-pill sp-attachment-pill--${escape(String(item?.status || 'uploading'))}" data-attachment-id="${id}"><span class="sp-attachment-name">${name}</span><span class="sp-attachment-meta">${size} · ${statusLabel}</span><button type="button" class="sp-attachment-remove" data-attachment-remove="${id}" aria-label="Remove ${name}">Remove</button></span>`;
  }).join('');
}

export function formatPromptWithAttachments(promptText, attachments = []) {
  const base = String(promptText || '').trim();
  const refs = (Array.isArray(attachments) ? attachments : [])
    .filter(item => item?.status === 'saved' && item?.workspace_path)
    .map(item => `[Attachment: ${String(item.workspace_path).trim()}]`);
  if (!refs.length) return base.slice(0, 4000);
  const refText = refs.join('\n');
  // Never cut an attachment reference: shorten the typed message instead.
  return base ? `${base.slice(0, Math.max(0, 4000 - refText.length - 1))}\n${refText}`.slice(0, 4000) : refText.slice(0, 4000);
}
export function isSessionRunningState(state) {
  return state === 'thinking' || state === 'tool' || state === 'running';
}

export function createInterruptRequestId() {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }
  return `int_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 10)}`.slice(0, 64);
}

export function formatInterruptAckMessage(ack) {
  if (ack?.sent) {
    return 'Interrupt signal (Esc) sent to agent terminal.';
  }
  return 'Agent is no longer running a turn.';
}
