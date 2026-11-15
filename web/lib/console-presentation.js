import MarkdownIt from "./markdown-it.mjs";
import { PROVIDER_LOGOS } from "./provider-logos.js";

export const COPY_ICON_SVG = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 5.5V3.5A1.5 1.5 0 0 0 9 2H3.5A1.5 1.5 0 0 0 2 3.5V9A1.5 1.5 0 0 0 3.5 10.5H5.5"/></svg>';
export const CHECK_ICON_SVG = '<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.5 8.5L6.5 11.5L12.5 4.5"/></svg>';

export async function copyTextToClipboard(text) {
  const value = String(text ?? '');
  if (typeof navigator !== 'undefined' && navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
    try {
      await navigator.clipboard.writeText(value);
      return true;
    } catch {
      // Fall back to execCommand when clipboard API is blocked or unavailable.
    }
  }
  if (typeof document !== 'undefined' && document.body && typeof document.execCommand === 'function') {
    const area = document.createElement('textarea');
    area.value = value;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.top = '-9999px';
    area.style.left = '-9999px';
    area.style.opacity = '0';
    document.body.appendChild(area);
    area.select();
    area.setSelectionRange(0, value.length);
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch {
      ok = false;
    }
    area.remove();
    return ok;
  }
  return false;
}

export const sessionStateLabel = state => ({ thinking: 'Running', tool: 'Running', running: 'Running', awaiting_approval: 'Waiting', waiting: 'Waiting', idle: 'Idle', error: 'Error', ended: 'Ended', waiting_for_history: 'Starting' }[state] || 'Unknown');
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
export const agentLabel = id => AGENT_NAMES[id === 'claude' ? 'claude-code' : id === 'agy' ? 'antigravity' : id] || 'Unsupported agent';
// Wording for a held tool approval. Claude Code is held by a hook, so its own prompt stays
// available; Codex and Antigravity protocol sessions have no other prompt and cancel the call.
export function approvalCopy(agent) {
  const name = AGENT_NAMES[agent === 'claude' ? 'claude-code' : agent === 'agy' ? 'antigravity' : agent] || 'The agent';
  const native = name === 'Claude Code';
  return {
    agent: name,
    unanswered: native ? 'If you do not answer, Claude asks in its own window.' : `If you do not answer, ${name} cancels this tool call.`,
    unansweredPhone: native ? 'Then Claude asks on your laptop if you do not answer.' : `${name} cancels this tool call if you do not answer.`,
    unreachable: native ? 'Could not reach your laptop. Claude will ask there instead.' : `Could not reach your laptop. ${name} will cancel this tool call.`,
  };
}
const projectTime = project => { const time = Date.parse(project.last_active); return Number.isFinite(time) ? time : -Infinity; };

/** The folder a project lives in, shortened for one line: home becomes ~, then at most the last three folders. */
/** Distro named by a WSL share folder (wsl.localhost or wsl$, ADR 0060), else ''. */
export function wslDistro(path) {
  const m = /^(?:[\\/]{2}(?:\?[\\/]UNC[\\/])?)(?:wsl\.localhost|wsl\$)[\\/]+([^\\/]+)/i.exec(String(path ?? ''));
  return m ? m[1] : '';
}

/** UNC folder for a Linux path inside a distro: /home/a in Ubuntu becomes the wsl.localhost share path. */
export function wslFolder(distro, linuxPath) {
  const clean = String(linuxPath ?? '').trim().replace(/\/+$/, '');
  if (!distro || !clean.startsWith('/')) return '';
  return `\\\\wsl.localhost\\${distro}${clean.replace(/\//g, '\\')}`;
}

/** Keeps both ends of a long path, eliding the middle. */
export function middleTruncate(text, max = 48) {
  const s = String(text ?? '');
  if (s.length <= max) return s;
  const keep = max - 3;
  const head = Math.ceil(keep / 2);
  return `${s.slice(0, head)}...${s.slice(s.length - (keep - head))}`;
}

/** `WSL: <distro>` badge for a session row, or ''. */
export function wslBadge(session) {
  const distro = wslDistro(session?.repo_path || session?.cwd);
  return distro ? `<span class="sp-badge sp-wsl-badge" title="Runs in WSL distro ${escape(distro)}">WSL: ${escape(distro)}</span>` : '';
}

export function shortPath(path) {
  // A WSL share folder reads as `WSL: Ubuntu · ~/src` instead of share segments.
  const distro = wslDistro(path);
  if (distro) {
    const inner = '/' + String(path).replace(/\\/g, '/').split('/').filter(Boolean).slice(2).join('/');
    const rest = shortPath(inner);
    return `WSL: ${distro}${rest ? ` · ${rest}` : ''}`;
  }
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
  const figures = stats && summary ? `<div class="sp-heatmap-stats"><div><strong>${escape(tokens(summary.total_tokens))}</strong><span>tokens, past year</span></div><div><strong>${escape(summary.active_days ?? 'Unknown')}</strong><span>active days</span></div><div${streakTitle}><strong>${escape(summary.current_streak ?? 'Unknown')}</strong><span>day streak</span></div>${Number.isFinite(summary.longest_streak) ? `<div><strong>${escape(summary.longest_streak)}</strong><span>longest streak</span></div>` : ''}<div${busiest ? ` title="${escape(tokens(busiest.total_tokens))} tokens"` : ''}><strong>${busiest ? escape(monthDay(busiest.date)) : 'None'}</strong><span>busiest day</span></div></div>` : '';
  const description = summary ? `${tokens(summary.total_tokens)} tokens over ${summary.active_days} active days. Details below.` : 'Token activity by day.';
  const key = legend ? '<div class="sp-heatmap-legend" aria-hidden="true"><span>Less</span><i class="sp-heat" data-level="0"></i><i class="sp-heat" data-level="1"></i><i class="sp-heat" data-level="2"></i><i class="sp-heat" data-level="3"></i><i class="sp-heat" data-level="4"></i><span>More</span></div>' : '';
  const cols = Math.max(1, Math.ceil(days.length / 7));
  return `<section class="sp-section sp-heatmap${stats ? '' : ' sp-heatmap--mini'}" aria-label="${label}"><header class="sp-section-head"><h2>${weeks < 53 ? `Last ${weeks} weeks` : 'Activity'}</h2><span class="sp-section-meta">Days in UTC</span></header><div class="sp-card sp-heatmap-card"><div class="sp-heatmap-scroll" role="group" aria-label="Activity grid" style="--cols:${cols}">${labels.length ? `<div class="sp-heatmap-months" aria-hidden="true">${labels.map(mark => `<span style="--col:${mark.col}">${escape(mark.text)}</span>`).join('')}</div>` : ''}<div class="sp-heatmap-body"><div class="sp-heatmap-days" aria-hidden="true"><span>Mon</span><span></span><span>Wed</span><span></span><span>Fri</span><span></span><span></span></div><div class="sp-heatmap-grid" role="img" aria-label="${escape(description)}">${cells}</div></div></div>${figures || key ? `<div class="sp-heatmap-foot">${figures}${key}</div>` : ''}</div></section>`;
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


const LIVE_LABELS = { working: 'Working', waiting_for_you: 'Waiting for you', idle: 'Idle', stopped: 'Stopped' };

/** Live chat state from the daemon: text plus a shape, never color alone. Inferred states say so. */
export function renderLiveBadge(live) {
  if (!live || !LIVE_LABELS[live.state]) return '';
  const inferred = live.basis === 'exact' ? '' : '<span class="sp-live-basis" title="Inferred from recent log activity, not confirmed by a running process">Inferred</span>';
  return `<span class="sp-live" data-live="${escape(live.state)}"><span class="sp-live-shape" aria-hidden="true"></span><span class="sp-live-label">${LIVE_LABELS[live.state]}</span>${inferred}</span>`;
}

export function formatElapsed(fromIso, nowMs = Date.now()) {
  const start = Date.parse(fromIso || '');
  if (!Number.isFinite(start)) return '';
  const secs = Math.max(0, Math.floor((nowMs - start) / 1000));
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${String(secs % 60).padStart(2, '0')}s`;
  return `${Math.floor(secs / 3600)}h ${String(Math.floor(secs / 60) % 60).padStart(2, '0')}m`;
}

/** Bottom-of-transcript activity line while the agent works; empty otherwise. */
export function renderActivityIndicator(live, turnStartedAt, nowMs = Date.now()) {
  if (live?.state !== 'working') return '';
  const elapsed = formatElapsed(turnStartedAt, nowMs);
  const inferred = live.basis === 'exact' ? '' : '<span class="sp-live-basis">Inferred</span>';
  return `<span class="sp-live" data-live="working"><span class="sp-live-shape" aria-hidden="true"></span><span class="sp-live-label">${escape(live.step || 'Working')}</span></span>${elapsed ? `<time class="sp-activity-elapsed">${escape(elapsed)}</time>` : ''}${inferred}`;
}

export function updateOngoing(element, sessions) { updateMarkup(element, renderOngoing(sessions)); }

const HOME_GROUPS = [['needs_you', 'Needs you'], ['running', 'Running'], ['recent', 'Recent']];
const HOME_RECENT_MAX = 6;
const CHEVRON_RIGHT = '<svg class="sp-session-chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m9 6 6 6-6 6"/></svg>';
const shortAge = value => { const date = new Date(value); if (!Number.isFinite(date.getTime())) return ''; const age = Math.max(0, Math.floor((Date.now() - date.getTime()) / 1000)); return age < 60 ? 'now' : age < 3600 ? Math.floor(age / 60) + 'm' : age < 86400 ? Math.floor(age / 3600) + 'h' : Math.floor(age / 86400) + 'd'; };

const SUBAGENT_ROWS_MAX = 5;
export const SUBAGENT_PREF_KEY = 'gnomcode.includeSubagents';
// Display preference only: on by default so the collapsed group can expand; off leaves a count.
export function subagentsIncluded() {
  try { return typeof localStorage === 'undefined' || localStorage.getItem(SUBAGENT_PREF_KEY) !== 'false'; } catch { return true; }
}
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
  const summary = `${live ? `${live} running · ` : ''}${count} subagent${count === 1 ? '' : 's'}`;
  if (!list.length || !subagentsIncluded()) return `<p class="sp-subagent-count">${summary}</p>`;
  const key = `sub:${session.id}`;
  const shown = list.slice(0, SUBAGENT_ROWS_MAX);
  const rows = shown.map(child => {
    const group = child.home_group || (isLive(child) ? 'running' : 'recent');
    const state = child.presence === 'recently_idle' ? 'Idle' : child.state === 'unknown' ? 'No recent activity' : sessionStateLabel(child.state);
    const meta = [state, child.model && child.model !== 'Unspecified' ? child.model : ''].filter(Boolean).join(' · ');
    return `<li><button type="button" class="sp-subagent-row" data-action="${action}" data-arg="${escape(child.id)}"><span class="sp-session-dot sp-session-dot--${group}" aria-hidden="true"></span><span class="sp-session-main"><strong>${escape(child.name || (child.title && child.title.toLowerCase() !== 'unknown' ? child.title : 'Subagent'))}</strong><span>${escape(meta)}</span></span><time class="sp-session-time" datetime="${escape(child.last_event_at || '')}">${escape(shortAge(child.last_event_at))}</time></button></li>`;
  }).join('');
  const more = count > shown.length ? `<li class="sp-subagent-more"><button type="button" class="sp-link sp-subagent-more-btn" data-action="open-all-chats">View ${count - shown.length} more in All chats &#8250;</button></li>` : '';
  return `<details class="sp-subagents" data-key="${escape(key)}"${subagentOpen.has(key) ? ' open' : ''}><summary>${summary}</summary><ul>${rows}${more}</ul></details>`;
}

function homeRow(session, group) {
  const project = session.project_name && session.project_name.toLowerCase() !== 'unknown' ? session.project_name : 'Private project';
  const title = session.title && session.title.toLowerCase() !== 'unknown' ? session.title : project === 'Private project' ? 'Private session' : project;
  const fill = Number.isFinite(session.context_fill_pct) ? Math.round(session.context_fill_pct) : null;
  const totalTok = Number.isFinite(session.total_tokens) ? session.total_tokens : (session.input_tokens ?? 0) + (session.output_tokens ?? 0);
  const subagentBadge = session.subagent_count > 0 ? `${session.subagent_count} subagent${session.subagent_count === 1 ? '' : 's'}` : '';
  const meta = [project, session.surface_label || agentLabel(session.agent), session.model && session.model !== 'Unspecified' ? session.model : '', subagentBadge].filter(Boolean).join(' · ');
  const tokenBadge = `<span class="sp-session-tokens">${totalTok > 0 ? escape(tokens(totalTok)) : ''}</span>`;
  const context = fill === null ? '<span class="sp-session-context" aria-hidden="true"></span>' : `<span class="sp-session-context" title="Context ${fill}% full"><i style="--fill:${fill}%"></i></span>`;
  return `<button type="button" class="sp-session-row" data-action="open-home-session" data-arg="${escape(session.id)}"><span class="sp-session-dot sp-session-dot--${group}" aria-hidden="true"></span><span class="sp-session-main"><strong>${escape(title)}</strong><span>${wslBadge(session)}${escape(meta)}</span></span>${tokenBadge}${context}<time class="sp-session-time" datetime="${escape(session.last_event_at || '')}" title="${escape(new Date(session.last_event_at).toLocaleString())}">${escape(shortAge(session.last_event_at))}</time>${CHEVRON_RIGHT}</button>`;
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
const WINDOW_STATES = ['ok', 'low', 'exhausted', 'reset_unconfirmed', 'unknown'];
// No Rust state (older desktop or synced payload): a window with a reading renders neutral ("plain"), not as No data.
const stateOf = value => {
  if (value?.state === 'reset_awaiting') return 'reset_unconfirmed';
  return WINDOW_STATES.includes(value?.state) ? value.state : Number.isFinite(value?.remaining_pct) ? 'plain' : 'unknown';
};
// Payloads synced from an older desktop have no state but still say whether the reset has passed.
const awaiting = win => stateOf(win) === 'reset_unconfirmed' || win.reset_passed === true;
const CHIP_TEXT = { ok: 'OK', low: 'Low', exhausted: 'Out', reset_unconfirmed: 'Resetting', unknown: 'No data' };
const CHIP_CLASS = { ok: 'ok', low: 'low', exhausted: 'exhausted', reset_unconfirmed: 'unknown', unknown: 'unknown', plain: 'neutral' };
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
  if (awaiting(win)) return 'Reset, not confirmed';
  return Number.isFinite(win.remaining_pct) && win.basis !== 'unknown' ? `${percent(win.remaining_pct)} left` : 'Unknown';
}

function detailText(win) {
  const seen = win.stale || win.source_issue || awaiting(win) ? seenText(win) : '';
  if (win.source_issue) return seen ? `${win.source_issue} · ${seen}` : win.source_issue;
  const state = stateOf(win);
  if (awaiting(win)) return seen ? `0% used · awaiting fresh reading · ${seen}` : '0% used · awaiting fresh reading';
  const when = whenPart(win.resets_at);
  const reset = state === 'exhausted'
    ? (when ? `Back ${when}` : 'Reset time unknown')
    : (when ? `Resets ${when}` : win.reset_label ? `Resets ${win.reset_label}` : 'Reset time unknown');
  return seen ? `${reset} · ${seen}` : reset;
}

function paceMarkup(win) {
  const pace = win.pace;
  if (!pace) return '';
  if (pace.stale || win.stale) {
    const age = Number.isFinite(Date.parse(win.observed_at)) ? relativeActivity(win.observed_at) : null;
    const note = age ? `Based on data from ${age}` : (pace.stale_reason || 'Stale data');
    return `<span class="sp-pace-stale">${escape(note)}</span>`;
  }
  if (pace.verdict === 'on_pace') return 'On pace';
  if (pace.verdict === 'behind') return `${Math.round(Math.abs(pace.delta_pct))}% in reserve`;
  const runsOut = pace.runs_out_at ? whenPart(pace.runs_out_at) : null;
  return runsOut ? `Runs out ${escape(runsOut)}` : `${Math.round(Math.abs(pace.delta_pct))}% ahead of pace`;
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
  // The tick is the even-pace mark: what would be left now if the window were used at a steady rate.
  const paceNote = pace ? `. Tick: even pace would leave ${Math.round(win.pace.pace_remaining_pct)}% now` : '';
  return `<div class="sp-meter" ${aria} aria-label="${escape(`${name}: ${valueText(win)}${paceNote}`)}"${pace ? ` title="${escape(`Tick marks even pace: ${Math.round(win.pace.pace_remaining_pct)}% would be left now at a steady rate. Fill left of the tick means you are using it faster than that.`)}"` : ''}><i class="sp-meter-fill" style="--fill:${width}%"></i>${pace}</div>`;
};

export function renderMeterRow(win, { detail = true } = {}) {
  const state = stateOf(win);
  const name = windowLabel(win);
  const estTag = awaiting(win) ? ' <span class="sp-trust-tag">EST</span>' : '';
  const seenBadge = !detail && win.stale && seenText(win) ? ` <span class="sp-seen-inline">${escape(seenText(win))}</span>` : '';
  const staleClass = win.stale ? ' sp-meter-row--stale' : '';
  return `<div class="sp-meter-row sp-meter-row--${state}${staleClass}${win.source_issue ? ' sp-meter-row--issue' : ''}" title="${escape(meterTitle(win))}"><div class="sp-meter-top"><span class="sp-meter-label">${escape(name)}</span><span class="sp-meter-value">${escape(valueText(win))}${estTag}${seenBadge}</span></div>${meterBar(win, name)}${detail ? `<div class="sp-meter-bottom"><span>${escape(detailText(win))}</span><span>${paceMarkup(win)}</span></div>` : ''}</div>`;
}

function renderMiniRow(win) {
  const state = stateOf(win);
  const name = windowName(win);
  const when = whenPart(win.resets_at, { short: true, bare: true }) || '';
  const value = state === 'exhausted' ? 'Out' : awaiting(win) ? 'Reset, not confirmed · EST' : Number.isFinite(win.remaining_pct) && win.basis !== 'unknown' ? percent(win.remaining_pct) : 'Unknown';
  const staleClass = win.stale ? ' sp-mini--stale' : '';
  const rightMeta = awaiting(win) ? '' : (win.stale && seenText(win) ? seenText(win) : when);
  return `<div class="sp-mini sp-mini--${state}${staleClass}${win.source_issue ? ' sp-mini--issue' : ''}" title="${escape(meterTitle(win))}"><span>${escape(name)}</span>${meterBar(win, windowLabel(win))}<span class="sp-mini-value"><b>${escape(value)}</b>${escape(rightMeta)}</span></div>`;
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
  const label = escape(head.pool ? `${head.window_label || windowLabel(head)}` : windowLabel(head));
  const when = whenPart(head.resets_at, { short: true });
  const exhaustedPools = windows.filter(win => stateOf(win) === 'exhausted').length;
  const outNote = exhaustedPools > 0 && exhaustedPools < windows.length ? ` · ${exhaustedPools} pool${exhaustedPools === 1 ? '' : 's'} out` : '';
  const seen = seenText(head);
  const seenSuffix = (head.stale || awaiting(head)) && seen ? ` · <span class="sp-seen-inline">${escape(seen)}</span>` : '';
  const poolPrefix = head.pool ? `${escape(head.pool)} · ` : '';
  const text = state === 'exhausted' ? `${poolPrefix}${label} <b>exhausted</b>${when ? ` · back ${escape(whenPart(head.resets_at))}` : ''}${seenSuffix}`
    : awaiting(head) ? `${poolPrefix}${label} <span class="sp-reset-muted">Reset, not confirmed</span> <span class="sp-trust-tag">EST</span>${seenSuffix}`
      : `${poolPrefix}${label} <b>${escape(valueText(head))}</b>${outNote}${when && !outNote ? ` · resets ${escape(when)}` : ''}${seenSuffix}`;
  return `<span class="sp-provider-line">${text}</span>`;
}

function costLine(p) {
  if (!p.cost) return '';
  return `<span>Today ${escape(money(p.cost.today_usd, p.cost.basis))} · 30 days ${escape(money(p.cost.last_30_days_usd, p.cost.basis))}${p.cost.basis === 'unknown' ? ' · partial' : ''}</span>`;
}

function emptyCard(p) {
  const text = p.source_issue || p.enable_hint || 'No reading yet.';
  const command = p.enable_command ? `<button type="button" class="sp-copy" data-copy="${escape(p.enable_command)}" aria-label="Copy command ${escape(p.enable_command)}"><code>${escape(p.enable_command)}</code><span class="sp-copy-action">${COPY_ICON_SVG}<span class="sp-copy-label">Copy</span></span></button>` : '';
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
  const wslNote = Array.isArray(p.wsl_sources) && p.wsl_sources.length ? p.wsl_sources.map(label => `<span class="sp-badge sp-wsl-badge" title="Also reads this agent's logs from ${escape(label)}; limits count once per account">${escape(label)}</span>`).join('') : '';
  const source = p.source_label ? `${wslNote}${escape(p.source_label)}${Number.isFinite(Date.parse(p.updated_at)) ? ` · Seen ${escape(relativeActivity(p.updated_at))}` : ''}` : '';
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
const providerCardState = new WeakMap();

/** Re-renders the cards without collapsing the ones the user opened; open state survives reloads. */
export function updateProviderCards(element, list, options = {}) {
  const first = !element.querySelector('details[data-provider]');
  updateMarkup(element, renderProviderCards(list, options));
  if (first) {
    element.querySelector('.sp-limits')?.setAttribute('data-enter', '');
    const stored = readProviderOpen();
    for (const card of element.querySelectorAll('details[data-provider]')) card.open = stored[card.dataset.provider] === true;
  }

  let entry = providerCardState.get(element);
  if (!entry) {
    entry = { list, options, tickId: null, resetId: null };
    entry.tickId = setInterval(() => {
      if (!element.isConnected) {
        clearInterval(entry.tickId);
        clearTimeout(entry.resetId);
        providerCardState.delete(element);
        return;
      }
      updateMarkup(element, renderProviderCards(entry.list, entry.options));
    }, 30_000);
    providerCardState.set(element, entry);
  } else {
    entry.list = list;
    entry.options = options;
  }

  clearTimeout(entry.resetId);
  entry.resetId = null;
  if (typeof options.onResetBoundary === 'function') {
    const nowMs = Date.now();
    let earliest = Infinity;
    for (const p of Array.isArray(list) ? list : []) {
      for (const win of Array.isArray(p?.windows) ? p.windows : []) {
        if (awaiting(win)) continue;
        const at = Date.parse(win?.resets_at);
        if (Number.isFinite(at) && at > nowMs && at < earliest) earliest = at;
      }
    }
    if (Number.isFinite(earliest)) {
      const delay = Math.min(2_147_483_647, Math.max(250, earliest - nowMs + 250));
      entry.resetId = setTimeout(() => {
        if (element.isConnected) options.onResetBoundary();
      }, delay);
    }
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
const WORST_FIRST = ['exhausted', 'low', 'unknown', 'reset_unconfirmed', 'ok', 'plain'];
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
    return `${heading}<div class="ide-session-row" data-session-id="${escape(session.id)}"><button type="button" class="ide-session" data-action="load-chat" data-arg="${escape(session.id)}" data-subagent="${isSubagent ? 'true' : 'false'}" aria-current="${session.id === selected}"><strong class="ide-session-title">${escape(session.title || session.project_name || session.repo_path || 'Untitled session')}</strong><span class="ide-session-meta">${wslBadge(session)}${escape(session.surface_label || session.agent)}${isSubagent ? ' · Subagent' : ''} · ${session.live && !session.state_label ? renderLiveBadge(session.live) : escape(session.state_label || sessionStateLabel(session.state))}</span>${Number.isFinite(Date.parse(session.last_event_at)) ? `<time datetime="${escape(session.last_event_at)}">${escape(relativeActivity(session.last_event_at))}</time>` : ''}</button><button type="button" class="ide-session-more" data-action="session-menu" data-arg="${escape(session.id)}" aria-label="Actions for ${escape(session.title || session.project_name || 'session')}" title="More actions">&#8942;</button></div>${renderSubagents(session, 'load-chat')}`;
  }).join('') || '<p class="local-view-message">No matching sessions. Clear search or launch an installed agent.</p>';
}


const IMAGE_DATA_URL_RE = /^data:image\/(?:png|jpe?g|webp|gif|svg\+xml);base64,[A-Za-z0-9+/=]+$/i;
const CODE_KEYWORDS = new Set('as async await break case catch class const continue crate def default defer del do elif else enum export extends false fi final finally fn for from func function go if impl import in interface let loop match mod move mut new nil None null package pass pub raise return self Self static struct super switch then this throw trait true True False try type typeof undefined unsafe use var void where while with yield'.split(' '));
const CODE_ALIASES = { js:'javascript', mjs:'javascript', jsx:'javascript', ts:'typescript', tsx:'typescript', py:'python', rs:'rust', sh:'shell', bash:'shell', zsh:'shell', ps1:'powershell', yml:'yaml', md:'markdown', patch:'diff' };
const CODE_TOKEN_RE = /(\/\/[^\n]*|#(?![[!])[^\n]*|\/\*[\s\S]*?\*\/|--[^\n]*)|("(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|`(?:\\.|[^`\\])*`)|\b(\d[\d_.]*(?:e[+-]?\d+)?[a-z]*)\b|([A-Za-z_$][\w$]*)/gi;
const HASH_COMMENT_LANGS = new Set(['python','shell','yaml','toml','ruby','powershell','perl','r','dockerfile','makefile','']);
const DASH_COMMENT_LANGS = new Set(['sql','lua','haskell']);
const DIFF_RE = /^(diff --git|@@ |--- a\/)/m;

/** Display-only highlighter: comments, strings, numbers, keywords, and diff lines. */
export function highlightCode(code, language = '') {
  const lang = CODE_ALIASES[language.toLowerCase()] || language.toLowerCase();
  if (lang === 'diff') {
    return code.split('\n').map(line => {
      const cls = /^(\+\+\+|---)/.test(line) || /^(diff |index )/.test(line) ? 'meta' : line.startsWith('+') ? 'add' : line.startsWith('-') ? 'del' : line.startsWith('@@') ? 'hunk' : '';
      return cls ? `<span class="sp-diff-${cls}">${escape(line)}</span>` : escape(line);
    }).join('\n');
  }
  if (['text','plain','txt','markdown','csv','log'].includes(lang)) return escape(code);
  let out = '', last = 0;
  for (const match of code.matchAll(CODE_TOKEN_RE)) {
    const [text, comment, string, number, word] = match;
    let cls = '';
    if (comment) {
      const wrongHash = text.startsWith('#') && !HASH_COMMENT_LANGS.has(lang);
      const wrongDash = text.startsWith('--') && !DASH_COMMENT_LANGS.has(lang);
      if (wrongHash || wrongDash) continue;
      cls = 'comment';
    } else if (string) cls = 'string';
    else if (number) cls = 'number';
    else if (word && CODE_KEYWORDS.has(word)) cls = 'keyword';
    if (!cls) continue;
    out += escape(code.slice(last, match.index)) + `<span class="sp-hl-${cls}">${escape(text)}</span>`;
    last = match.index + text.length;
  }
  return out + escape(code.slice(last));
}

/** Code block with a language label, Copy and Wrap controls; scrolls horizontally by default. */
export function renderCodeBlock(code, language = '') {
  const text = String(code ?? '').replace(/\n$/, '');
  const label = language ? language.replace(/[^\w+#.-]/g, '').slice(0, 24) || 'text' : (DIFF_RE.test(text) ? 'diff' : 'text');
  return `<div class="sp-code" data-wrap="false"><div class="sp-code-bar"><span class="sp-code-lang">${escape(label)}</span><button type="button" class="sp-code-btn" data-code-wrap aria-pressed="false">Wrap</button><button type="button" class="sp-code-btn sp-copy-btn" data-chat-copy="${escape(text)}" aria-label="Copy ${escape(label)} code" title="Copy code">${COPY_ICON_SVG}<span class="sp-copy-label">Copy</span></button></div><pre tabindex="0"><code>${highlightCode(text, label)}</code></pre></div>`;
}

const SYSTEM_EVENT_TYPES = new Set(['task_notification','reminder','command','subagent_report','compact_summary','interrupt']);

function renderSystemEvent(evt) {
  const [summary, ...rest] = String(evt.content).split('\n\n');
  const detail = rest.join('\n\n').trim();
  const attrs = `class="sp-chat-system" data-chat-id="${escape(evt.event_id || '')}" data-kind="${escape(evt.status)}"`;
  const head = `<span class="sp-chat-system-mark" aria-hidden="true"></span><span class="sp-chat-system-text">${escape(summary)}</span>`;
  if (!detail) return `<div ${attrs}>${head}</div>`;
  const body = ['compact_summary','subagent_report'].includes(evt.status) ? chatMarkdown(detail) : `<pre>${escape(detail)}</pre>`;
  return `<details ${attrs}><summary>${head}</summary><div class="sp-chat-body">${body}</div></details>`;
}

function renderToolEvent(evt) {
  const label = `Tool: ${evt.tool_name}${evt.status ? ` (${evt.status})` : ''}`;
  const body = evt.content?.trim() ? `<div class="sp-chat-body">${renderCodeBlock(evt.content, 'text')}</div>` : '<p class="sp-chat-system-note">Tool output is not stored.</p>';
  return `<details class="sp-chat-system sp-chat-tool" data-chat-id="${escape(evt.event_id || '')}" data-kind="tool"><summary><span class="sp-chat-system-mark" aria-hidden="true"></span><span class="sp-chat-system-text">${escape(label)}</span></summary>${body}</details>`;
}

function renderToolGroup(tools) {
  if (tools.length === 1) return renderToolEvent(tools[0]);
  const names = [...new Set(tools.map(t => t.tool_name).filter(Boolean))];
  const summary = `Worked for ${tools.length} steps${names.length ? ` (${names.slice(0, 3).join(', ')}${names.length > 3 ? ', ...' : ''})` : ''}`;
  const items = tools.map(evt => `<li data-chat-id="${escape(evt.event_id || '')}"><strong>${escape(evt.tool_name || 'tool')}</strong>${evt.status ? ` <span class="sp-worked-status">${escape(evt.status)}</span>` : ''}</li>`).join('');
  return `<details class="sp-chat-system sp-chat-tool sp-chat-worked" data-kind="tool-group"><summary><span class="sp-chat-system-mark" aria-hidden="true"></span><span class="sp-chat-system-text">${escape(summary)}</span></summary><ul class="sp-worked-list">${items}</ul></details>`;
}

const ATTACHMENT_REF_RE = /^\[Attachment: ([^\]\n]+)\]$/gm;
const UNAVAILABLE_IMG_TEXT = '[Image attachment: preview unavailable in imported history]';

function renderUserBody(content) {
  const refs = [...content.matchAll(ATTACHMENT_REF_RE)].map(m => m[1].trim());
  let text = content.replace(ATTACHMENT_REF_RE, '').trim();
  const unavailableCount = text.split(UNAVAILABLE_IMG_TEXT).length - 1;
  if (unavailableCount > 0) {
    text = text.replaceAll(UNAVAILABLE_IMG_TEXT, '').trim();
  }
  const thumbs = refs.map((path, i) => {
    const isImportedApi = path.startsWith('/api/v1/conversations/');
    const name = isImportedApi ? `Image #${i + 1}` : path.split('/').pop();
    return `<button type="button" class="sp-chat-thumb sp-chat-thumb--pending" data-attachment-path="${escape(path)}" aria-label="Open attachment ${escape(name)}"><span>${escape(name)}</span></button>`;
  }).join('');
  const unavailable = Array.from({ length: unavailableCount }, (_, i) =>
    `<span class="sp-chat-thumb sp-chat-thumb--unavailable" role="status"><span>Image #${i + 1} unavailable</span></span>`,
  ).join('');
  return `${escape(text)}${thumbs || unavailable ? `<div class="sp-chat-thumbs">${thumbs}${unavailable}</div>` : ''}`;
}

/** Binds copy, wrap and image-open controls inside rendered chat markup once per container. */
export function bindChatInteractions(element) {
  if (!element || element.dataset.chatBound) return;
  element.dataset.chatBound = 'true';
  element.addEventListener('click', event => {
    const origin = event.target;
    const copy = origin?.closest?.('[data-chat-copy]');
    if (copy) {
      event.stopPropagation();
      const label = copy.querySelector('.sp-copy-label');
      const resetText = label ? label.textContent : copy.textContent;
      const resetAria = copy.getAttribute('aria-label');
      const setLabel = (next) => {
        if (label) label.textContent = next;
        else copy.textContent = next;
        copy.setAttribute('aria-label', next);
      };
      copyTextToClipboard(copy.dataset.chatCopy ?? '')
        .then((ok) => {
          copy.dataset.copyState = ok ? 'copied' : 'error';
          setLabel(ok ? 'Copied' : 'Copy failed');
        })
        .catch(() => {
          copy.dataset.copyState = 'error';
          setLabel('Copy failed');
        })
        .finally(() => setTimeout(() => {
          delete copy.dataset.copyState;
          if (label) label.textContent = resetText;
          else copy.textContent = resetText;
          if (resetAria !== null) copy.setAttribute('aria-label', resetAria);
          else copy.removeAttribute('aria-label');
        }, 1600));
      return;
    }
    const wrap = origin?.closest?.('[data-code-wrap]');
    if (wrap) {
      const block = wrap.closest('.sp-code');
      const next = block.dataset.wrap !== 'true';
      block.dataset.wrap = String(next);
      wrap.setAttribute('aria-pressed', String(next));
      return;
    }
    const image = origin?.closest?.('[data-image-open]');
    if (image) { event.stopPropagation(); openImageViewer(image.dataset.imageOpen, image.querySelector('img')?.alt || 'Image'); }
  });
}

/** Full-size image viewer in a modal dialog; only data: and blob: sources are shown. */
export function openImageViewer(src, alt = 'Image') {
  if (!/^(data:image\/|blob:)/i.test(String(src || ''))) return;
  const dialog = document.createElement('dialog');
  dialog.className = 'sp-image-viewer';
  dialog.innerHTML = `<form method="dialog"><button type="submit" class="sp-code-btn">Close</button></form><img alt="${escape(alt)}">`;
  dialog.querySelector('img').src = src;
  dialog.addEventListener('close', () => dialog.remove());
  dialog.addEventListener('click', event => { if (event.target === dialog) dialog.close(); });
  document.body.append(dialog);
  dialog.showModal();
}

// Raw HTML and remote image fetching stay disabled for agent-supplied Markdown.
const markdown = new MarkdownIt({html:false,breaks:true,linkify:false});
// Raster data-URL images render as thumbnails; external images need a click that names the host.
markdown.renderer.rules.image = (tokens,index) => {
  const src=tokens[index].attrGet('src')||'',alt=tokens[index].content||'image';
  if(IMAGE_DATA_URL_RE.test(src)) return `<button type="button" class="sp-chat-thumb" data-image-open="${escape(src)}" aria-label="Open ${escape(alt)} full size"><img src="${escape(src)}" alt="${escape(alt)}" loading="lazy"></button>`;
  let host='';try{host=new URL(src).host;}catch{}
  return host ? `<a class="sp-chat-attachment" href="${escape(src)}" target="_blank" rel="noopener noreferrer">External image not loaded: ${escape(alt)} (opens ${escape(host)})</a>` : `<span class="sp-chat-attachment">Image: ${escape(alt)}</span>`;
};
markdown.renderer.rules.fence = (tokens,index) => renderCodeBlock(tokens[index].content, tokens[index].info.trim().split(/\s+/)[0]);
markdown.renderer.rules.code_block = (tokens,index) => renderCodeBlock(tokens[index].content, '');
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
  events = events?.filter(evt => (evt.role === 'tool' && evt.tool_name) || ((evt.role === 'user' || evt.role === 'assistant' || (evt.role === 'system' && (['model_active','model_changed'].includes(evt.status) || SYSTEM_EVENT_TYPES.has(evt.status)))) && evt.content?.trim()));
  if (!Array.isArray(events) || !events.length) {
    return '<div class="sp-chat-empty"><p>No conversation history recorded for this session.</p></div>';
  }
  const chunks = [];
  for (let i = 0; i < events.length; i++) {
    const evt = events[i];
    if (evt.role === 'tool') {
      const group = [evt];
      while (i + 1 < events.length && events[i + 1].role === 'tool') {
        group.push(events[++i]);
      }
      chunks.push(renderToolGroup(group));
      continue;
    }
    if (evt.role === 'system' && SYSTEM_EVENT_TYPES.has(evt.status)) { chunks.push(renderSystemEvent(evt)); continue; }
    if (evt.role === 'system') { chunks.push(`<div class="sp-chat-notice" role="status">${escape(evt.content)}<time datetime="${escape(evt.created_at)}">${escape(new Date(evt.created_at).toLocaleString())}</time></div>`); continue; }
    const role = evt.role || 'system';
    const roleLabel = role === 'user' ? 'You' : 'Agent';
    const statusBadge = evt.status ? `<span class="sp-chat-badge sp-badge-${escape(evt.status)}">${escape(evt.status.toUpperCase())}</span>` : '';
    const approvalPrompt = evt.status === 'pending' ? `<p class="sp-approval-banner">Approval is pending in the agent.</p>` : '';
    const copyBtn = evt.waiting ? '' : `<button type="button" class="sp-code-btn sp-copy-btn sp-chat-copy" data-chat-copy="${escape(evt.content)}" aria-label="Copy ${role === 'user' ? 'your message' : 'assistant reply'}" title="Copy message">${COPY_ICON_SVG}<span class="sp-copy-label">Copy</span></button>`;
    chunks.push(`<article data-chat-id="${escape(evt.event_id || '')}" class="sp-chat-card sp-chat-${escape(role)}${evt.waiting ? ' sp-chat-waiting' : ''}">
      <header>
        <strong>${escape(roleLabel)}</strong>
        ${statusBadge}
        <time datetime="${escape(evt.created_at || '')}" title="${escape(new Date(evt.created_at).toLocaleString())}">${escape(new Date(evt.created_at).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }))}</time>
        ${copyBtn}
      </header>
      <div class="sp-chat-body">${role === 'assistant' ? chatMarkdown(evt.content) : renderUserBody(evt.content)}</div>
      ${evt.delivery_note ? `<p class="sp-chat-delivery-note" role="status">${escape(evt.delivery_note)}</p>` : ''}
      ${approvalPrompt}
    </article>`);
  }
  return `<div class="sp-chat-stream">${chunks.join('')}</div>`;
}

/** Shows or hides the time and Copy of the message a person clicked; other messages hide theirs. */
export function toggleChatMeta(target) {
  if (!target?.closest || target.closest('a, button, summary, input, textarea, .sp-code')) return false;
  if (globalThis.getSelection?.()?.toString()) return false;
  const card = target.closest('.sp-chat-card');
  if (!card) return false;
  const shown = card.dataset.meta === 'shown';
  card.parentElement?.querySelectorAll('.sp-chat-card[data-meta="shown"]').forEach(other => { delete other.dataset.meta; });
  if (!shown) card.dataset.meta = 'shown';
  return true;
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
  bindChatInteractions(element);
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

/** Splits a unified diff into per-file blocks keyed by the post-image path (`b/...`). Display only. */
function diffBlocks(diffText) {
  const blocks = new Map();
  let path = '';
  for (const line of diffText.split(/\r?\n/)) {
    if (line.startsWith('diff --git ')) path = line.slice(line.lastIndexOf(' b/') + 3);
    if (!path) continue;
    if (!blocks.has(path)) blocks.set(path, []);
    blocks.get(path).push(line);
  }
  return blocks;
}

export function renderChangesView(data, { selected = '' } = {}) {
  if (!data) return '<p class="sp-provider-empty">Select a conversation workspace to inspect uncommitted changes.</p>';
  if (data.loading) return '<p class="sp-provider-empty" role="status">Loading workspace changes...</p>';
  if (data.error) return `<p class="sp-field-note sp-tone-red" role="alert">${escape(data.error)}</p>`;
  const files = parseGitStatusLines(data.status || '');
  const diffText = typeof data.diff === 'string' ? data.diff : '';
  const badges = [
    '<span class="sp-chip sp-chip--neutral">Read-only</span>',
    data.lossy ? '<span class="sp-chip sp-chip--low" title="Non-UTF-8 bytes were replaced with U+FFFD by Rust">Lossy UTF-8 decoding</span>' : '',
    data.repo ? `<span class="sp-changes-repo" title="${escape(data.repo)}">${escape(wslDistro(data.repo) ? middleTruncate(data.repo, 48) : shortPath(data.repo))}</span>` : '',
  ].filter(Boolean).join('');
  if (!files.length && !diffText.trim()) {
    return `<div class="sp-changes-meta">${badges}</div><p class="sp-provider-empty">Working tree clean: no modified or untracked files.</p>`;
  }
  const fileButton = (path, label, text) => `<li><button type="button" class="sp-changes-file" data-change-file="${escape(path)}" aria-pressed="${selected === path}"><span class="sp-chip sp-chip--neutral">${escape(label)}</span><code>${escape(text)}</code></button></li>`;
  const fileMarkup = files.length
    ? `<ul class="sp-changes-files" aria-label="Changed files">${fileButton('', 'All', `All files (${files.length})`)}${files.map(file => fileButton(file.path, file.label, file.path)).join('')}</ul>`
    : '';
  const shown = selected ? (diffBlocks(diffText).get(selected) || []).join('\n') : diffText;
  const diffLines = shown.trim()
    ? shown.split(/\r?\n/).map(line => `<span class="${classifyDiffLine(line)}">${escape(line) || ' '}</span>`).join('')
    : '';
  const diffMarkup = diffLines
    ? `<pre class="sp-diff-code" tabindex="0" aria-label="${selected ? `Diff for ${escape(selected)}` : 'Unified git diff'}">${diffLines}</pre>`
    : selected
      ? `<p class="sp-provider-empty">No diff for ${escape(selected)}. Untracked files have no diff until git tracks them.</p>`
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

const CSV_PREVIEW_ROWS = 200;

function parseCsvRows(text, limit) {
  const rows = [];
  let row = [], field = '', quoted = false;
  for (let i = 0; i < text.length && rows.length < limit; i++) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"' && text[i + 1] === '"') { field += '"'; i++; }
      else if (ch === '"') quoted = false;
      else field += ch;
    } else if (ch === '"') quoted = true;
    else if (ch === ',') { row.push(field); field = ''; }
    else if (ch === '\n' || ch === '\r') {
      if (ch === '\r' && text[i + 1] === '\n') i++;
      row.push(field); rows.push(row); row = []; field = '';
    } else field += ch;
  }
  if ((field || row.length) && rows.length < limit) { row.push(field); rows.push(row); }
  return rows;
}

function previewSize(size) {
  return Number.isFinite(size) ? formatAttachmentBytes(size) : 'Unknown size';
}

function downloadHref(fileData) {
  if (fileData.data_url) return fileData.data_url;
  if (typeof fileData.content === 'string' && !fileData.truncated) return `data:${fileData.mime_type || 'text/plain'};charset=utf-8,${encodeURIComponent(fileData.content)}`;
  return '';
}

const MAX_PREVIEW_LINES = 500;
const MAX_HIGHLIGHT_CHARS = 65536;

function previewBody(fileData, content) {
  const kind = fileData.kind || 'text';
  const name = fileData.name || fileData.path || 'file';
  if ((kind === 'image' || kind === 'svg') && isSafeArtifactImageSrc(fileData.data_url)) {
    // SVG is shown only through <img>, which never runs its scripts or loads its external references.
    return `<button type="button" class="sp-preview-image" data-image-open="${escape(fileData.data_url)}" aria-label="Open ${escape(name)} full size"><img src="${escape(fileData.data_url)}" alt="${escape(name)}"></button>`;
  }
  if (kind === 'pdf' && /^data:application\/pdf;base64,/.test(fileData.data_url || '')) {
    return `<div class="sp-preview-binary"><p>PDF document. It opens in a new tab with the browser's viewer.</p><button type="button" class="sp-code-btn" data-pdf-open="${escape(fileData.data_url)}">Open PDF</button></div>`;
  }
  if (kind === 'markdown') {
    const capped = content.length > MAX_HIGHLIGHT_CHARS ? content.slice(0, MAX_HIGHLIGHT_CHARS) : content;
    const note = content.length > MAX_HIGHLIGHT_CHARS ? `<p class="sp-field-note">Showing first ${previewSize(MAX_HIGHLIGHT_CHARS)} of Markdown preview. Download for full file.</p>` : '';
    return `<div class="sp-chat-body sp-preview-markdown">${chatMarkdown(capped)}</div>${note}`;
  }
  if (kind === 'csv') {
    const rows = parseCsvRows(content, CSV_PREVIEW_ROWS + 1);
    const [head = [], ...body] = rows;
    const more = body.length > CSV_PREVIEW_ROWS ? `<p class="sp-field-note">Showing the first ${CSV_PREVIEW_ROWS} rows.</p>` : '';
    return `<div class="sp-chat-table sp-preview-csv"><table><thead><tr>${head.map(cell => `<th>${escape(cell)}</th>`).join('')}</tr></thead><tbody>${body.slice(0, CSV_PREVIEW_ROWS).map(r => `<tr>${r.map(cell => `<td>${escape(cell)}</td>`).join('')}</tr>`).join('')}</tbody></table></div>${more}`;
  }
  if (kind === 'json') {
    let pretty = content;
    if (!fileData.truncated && content.length <= MAX_HIGHLIGHT_CHARS) { try { pretty = JSON.stringify(JSON.parse(content), null, 2); } catch { /* show as stored */ } }
    if (pretty.length > MAX_HIGHLIGHT_CHARS) {
      const lines = pretty.slice(0, MAX_HIGHLIGHT_CHARS).split(/\r?\n/).slice(0, MAX_PREVIEW_LINES);
      return `<p class="sp-field-note">Showing first ${lines.length} lines of JSON preview. Download for full file.</p><pre class="sp-file-code" tabindex="0" aria-label="Workspace file contents">${escape(lines.join('\n'))}</pre>`;
    }
    return `<div class="sp-chat-body">${renderCodeBlock(pretty, 'json')}</div>`;
  }
  if (kind === 'binary' || kind === 'image' || kind === 'svg' || kind === 'pdf') {
    return `<div class="sp-preview-binary"><p>${fileData.truncated ? 'Too large to preview here.' : 'No preview for this file type.'}</p></div>`;
  }
  const language = kind === 'code' && content.length <= MAX_HIGHLIGHT_CHARS ? fileData.language || '' : '';
  const allLines = content.split(/\r?\n/);
  const lines = allLines.slice(0, MAX_PREVIEW_LINES);
  const body = lines.map((line, index) => `<span class="sp-file-line"><span class="sp-file-ln">${index + 1}</span><span>${(language ? highlightCode(line, language) : escape(line)) || ' '}</span></span>`).join('');
  const lineNotice = allLines.length > MAX_PREVIEW_LINES
    ? `<p class="sp-field-note">Showing first ${MAX_PREVIEW_LINES} of ${allLines.length.toLocaleString()} lines. Download for full file.</p>`
    : '';
  return `${lineNotice}<pre class="sp-file-code" tabindex="0" aria-label="Workspace file contents">${body}</pre>`;
}

export function renderFileViewer(fileData) {
  if (!fileData) return '<p class="sp-provider-empty">Select a file or enter a workspace-relative path to inspect it read-only.</p>';
  if (fileData.loading) return '<p class="sp-provider-empty" role="status">Reading file...</p>';
  if (fileData.error) return `<p class="sp-field-note sp-tone-red" role="alert">${escape(fileData.error)}</p>`;
  const content = typeof fileData.content === 'string' ? fileData.content : '';
  const name = fileData.name || String(fileData.path || '').split('/').pop() || 'file';
  const href = downloadHref(fileData);
  const badges = [
    '<span class="sp-chip sp-chip--neutral">Read-only</span>',
    fileData.lossy ? '<span class="sp-chip sp-chip--low">Lossy UTF-8 decoding</span>' : '',
    fileData.truncated ? `<span class="sp-chip sp-chip--low">${typeof fileData.content === 'string' ? 'Preview truncated at 512 KiB' : 'Over the 1 MiB preview limit'}</span>` : '',
    Number.isFinite(fileData.size) ? `<span class="sp-chip sp-chip--neutral">${escape(previewSize(fileData.size))}</span>` : '',
    fileData.path ? `<code class="sp-file-path">${escape(fileData.path)}</code>` : '',
    href ? `<a class="sp-code-btn sp-file-download" href="${escape(href)}" download="${escape(name)}">Download</a>` : '',
  ].filter(Boolean).join('');
  return `<div class="sp-changes-meta">${badges}</div><div class="sp-file-preview" data-kind="${escape(fileData.kind || 'text')}">${previewBody(fileData, content)}</div>`;
}

/** Opens a PDF data URL in a new tab through a blob URL, so no network request is made. */
export function openPdfPreview(dataUrl) {
  const match = /^data:application\/pdf;base64,([A-Za-z0-9+/=]+)$/.exec(String(dataUrl || ''));
  if (!match) return;
  const bytes = Uint8Array.from(atob(match[1]), ch => ch.charCodeAt(0));
  const url = URL.createObjectURL(new Blob([bytes], { type: 'application/pdf' }));
  window.open(url, '_blank', 'noopener');
  setTimeout(() => URL.revokeObjectURL(url), 60_000);
}

/** Binds image-open and PDF-open controls in a file preview container once. */
export function bindPreviewInteractions(element) {
  if (!element || element.dataset.previewBound) return;
  element.dataset.previewBound = 'true';
  element.addEventListener('click', event => {
    const image = event.target?.closest?.('[data-image-open]');
    if (image) { openImageViewer(image.dataset.imageOpen, image.querySelector('img')?.alt || 'Image'); return; }
    const pdf = event.target?.closest?.('[data-pdf-open]');
    if (pdf) openPdfPreview(pdf.dataset.pdfOpen);
  });
}

const thumbCache = new Map();

/**
 * Lazily fills attachment thumbnails in a chat container. `fetchPreview(path)` resolves to a
 * file preview from the daemon (local API or E2EE Remote); only image previews become thumbnails.
 */
export function hydrateAttachmentThumbs(element, fetchPreview) {
  if (!element || typeof fetchPreview !== 'function') return;
  const load = async node => {
    if (node.dataset.thumbState) return;
    node.dataset.thumbState = 'loading';
    const path = node.dataset.attachmentPath;
    if (!thumbCache.has(path)) thumbCache.set(path, Promise.resolve().then(() => fetchPreview(path)));
    try {
      const preview = await thumbCache.get(path);
      if ((preview?.kind === 'image' || preview?.kind === 'svg') && isSafeArtifactImageSrc(preview.data_url)) {
        const img = document.createElement('img');
        img.alt = preview.name || 'attachment';
        img.src = preview.data_url;
        node.replaceChildren(img);
        node.dataset.imageOpen = preview.data_url;
        node.classList.remove('sp-chat-thumb--pending');
      }
      node.dataset.thumbState = 'done';
    } catch { node.dataset.thumbState = 'failed'; }
  };
  const nodes = [...element.querySelectorAll('[data-attachment-path]:not([data-thumb-state])')];
  if (typeof IntersectionObserver !== 'function') { nodes.forEach(load); return; }
  const observer = new IntersectionObserver(entries => entries.forEach(entry => {
    if (entry.isIntersecting) { observer.unobserve(entry.target); void load(entry.target); }
  }), { rootMargin: '200px' });
  nodes.forEach(node => observer.observe(node));
}
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
    return `<div class="sp-artifact-image-wrap"><button type="button" class="sp-preview-image" data-image-open="${escape(src)}" aria-label="Open ${escape(title)} full size"><img class="sp-artifact-image" src="${escape(src)}" alt="${escape(title)}" /></button></div>`;
  }
  return `<div class="sp-chat-body">${renderCodeBlock(String(artifact.content || 'Preview unavailable.'), artifact.language || 'text')}</div>`;
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
