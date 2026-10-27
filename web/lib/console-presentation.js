// Shared presentation only. Rust supplies totals, pricing, and chart proportions.
const escape = value => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]));
const tokens = value => Number.isFinite(value) ? new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(value) : 'Unknown';
const trust = value => ({ exact: 'EXACT', estimated: 'EST', api: 'API' }[value] || 'UNKNOWN');

export function renderSummary({ activeCount = null, report = null, quotaCount = null } = {}) {
  const priced = report?.cost_basis && report.cost_basis !== 'unknown';
  const money = value => priced && Number.isFinite(value) ? '$' + value.toFixed(2) : 'Unknown';
  const metric = (label, value, basis, caption) => `<div><span>${label}</span><strong>${escape(value)} <small class="sp-data-trust">${basis}</small></strong><p>${escape(caption)}</p></div>`;
  return `<section class="sp-summary-grid" aria-label="Observed fleet summary">${
    metric('Active sessions', Number.isFinite(activeCount) ? activeCount : 'Unknown', Number.isFinite(activeCount) ? 'EXACT' : 'UNKNOWN', 'At last source observation') +
    metric('API-equivalent spend', money(report?.total_cost_usd), priced && Number.isFinite(report?.total_cost_usd) ? 'EST' : 'UNKNOWN', 'Recorded usage in this period') +
    metric('Cache savings', money(report?.total_cache_savings_usd), priced && Number.isFinite(report?.total_cache_savings_usd) ? 'EST' : 'UNKNOWN', Number.isFinite(report?.cache_share_pct) ? report.cache_share_pct.toFixed(1) + '% cached input · EST' : 'Cached input share unknown') +
    metric('Quota pools', Number.isFinite(quotaCount) ? quotaCount : 'Unknown', Number.isFinite(quotaCount) ? 'EXACT' : 'UNKNOWN', 'At last source observation')
  }</section>`;
}

export function renderProjects(projects) {
  if (!Array.isArray(projects)) return '<section class="sp-activity"><h2>Project activity</h2><p>Project rollups remain local. This cloud source does not publish repository identity or project totals.</p></section>';
  return `<section class="sp-activity"><h2>Project activity</h2><p>Projects with measured usage in the selected period. Repository identity stays on this device.</p><div class="sp-quota-grid">${projects.length ? projects.map(project => `<article><header><strong>${escape(project.name)}</strong></header><div class="sp-quota-number">${escape(tokens(project.total_tokens))}<span class="sp-data-trust">EXACT tokens</span></div><p>${escape(project.sessions)} sessions · EXACT</p><p>${project.cost_basis === 'unknown' ? 'Cost unknown · UNKNOWN' : '$' + Number(project.cost_usd).toFixed(2) + ' API equivalent · EST'}</p></article>`).join('') : '<p>No project usage was recorded in this period.</p>'}</div></section>`;
}

export function renderSessions(sessions, { focus = false, rotate = false, illustrative = false } = {}) {
  if (!sessions?.length) return '<p class="sp-session-empty">No sessions match this view. Usage remains unknown until an agent reports it.</p>';
  return `<table class="sp-session-table"><thead><tr><th>Session</th><th>Agent / state</th><th>Tokens / EXACT</th><th>Active context</th><th>API equivalent</th></tr></thead><tbody>${sessions.map(session => {
    const measured = session.usage_available !== false && Number.isFinite(session.total_tokens);
    const context = session.usage_available !== false && Number.isFinite(session.context_tokens);
    const fill = Number.isFinite(session.context_fill_pct) ? session.context_fill_pct : null;
    const knownCost = session.cost_basis && session.cost_basis !== 'unknown' && Number.isFinite(session.total_cost_usd);
    const activeAt = new Date(session.last_event_at);
    return `<tr><td><details data-key="${escape(session.id)}"><summary><strong>${escape(session.project_name || 'Private project')}</strong><span>${escape(session.model || 'Model unknown')}</span></summary><dl><dt>Session</dt><dd>${escape(session.id)}</dd><dt>Source</dt><dd>${escape(session.surface || 'unknown')} · ${escape(session.device_name || 'local')}</dd><dt>Started</dt><dd>${escape(session.started_at || 'Unknown')}</dd>${session.title ? `<dt>Title</dt><dd>${escape(session.title)}</dd>` : ''}<dt>Input / EXACT</dt><dd>${escape(tokens(session.input_tokens))}</dd><dt>Output / EXACT</dt><dd>${escape(tokens(session.output_tokens))}</dd><dt>Cache read / EXACT</dt><dd>${escape(tokens(session.cache_read_tokens))}</dd></dl>${focus ? `<button type="button" data-session-action="focus" data-session-id="${escape(session.id)}">Focus session</button>` : ''}${rotate && session.surface !== 'cloud' ? `<button type="button" data-session-action="rotate" data-session-id="${escape(session.id)}">Rotate session</button>` : ''}</details></td><td><strong>${escape(session.agent)}</strong><span>${escape((session.state || 'unknown').replaceAll('_', ' '))} · EST</span><small>Last reported ${illustrative ? "sample" : Number.isFinite(activeAt.getTime()) ? escape(activeAt.toLocaleString()) : 'unknown'}</small></td><td><strong>${measured ? escape(tokens(session.total_tokens)) : 'Unknown'}</strong>${!measured ? '<span class="sp-data-trust">UNKNOWN</span>' : ''}</td><td><strong>${context ? escape(tokens(session.context_tokens)) : 'Unknown'}</strong><span class="sp-data-trust">${context ? 'EXACT observed' : 'UNKNOWN'}</span><small>Window ${escape(tokens(session.context_window))} · EST</small>${fill !== null ? `<div class="sp-quota-track"><i style="width:${Math.min(100, Math.max(0, fill))}%"></i></div><small>${fill.toFixed(0)}% · EST</small>` : ''}</td><td><strong>${knownCost ? '$' + session.total_cost_usd.toFixed(2) : 'Unknown'}</strong><span class="sp-data-trust">${knownCost ? trust(session.cost_basis) : 'UNKNOWN'}</span></td></tr>`;
  }).join('')}</tbody></table>`;
}

export function renderActivity(report) {
  if (!Array.isArray(report?.daily)) return '<section class="sp-activity"><h2>Daily activity</h2><p>Daily measurements are unavailable. Update the paired desktop app to publish this view.</p></section>';
  if (!report.daily.length) return '<section class="sp-activity"><h2>Daily activity</h2><p>No recorded usage in this period. Unobserved days are not counted as zero.</p></section>';
  const days = report.daily.slice(-84);
  return `<section class="sp-activity" aria-label="Daily token activity">
    <div class="sp-activity-heading"><div><h2>Daily activity</h2><p>Recorded days · UTC · expand a day for details</p></div><div class="sp-activity-totals"><div><strong>${escape(tokens(report.total_tokens))}</strong><span class="sp-data-trust">EXACT tokens</span></div>${report.total_cost_usd !== undefined ? `<div><strong>${report.cost_basis && report.cost_basis !== 'unknown' && Number.isFinite(report.total_cost_usd) ? '$' + report.total_cost_usd.toFixed(2) : 'Unknown'}</strong><span class="sp-data-trust">${report.cost_basis && report.cost_basis !== 'unknown' ? 'EST API equivalent' : 'UNKNOWN cost'}</span></div>` : ''}</div></div>
    <div class="sp-activity-bars" role="img" aria-label="Daily token volumes; individual measurements are available below">${days.map(day => `<i style="--day-height:${Number.isFinite(day.chart_pct) ? Math.min(100, Math.max(0, day.chart_pct)) : 0}%" title="${escape(day.date)}: ${escape(tokens(day.total_tokens))} tokens"></i>`).join('')}</div>
    <div class="sp-activity-grid">${days.map(day => `<details data-date="${escape(day.date)}"><summary><time datetime="${escape(day.date)}">${escape(day.date.slice(5))}</time><strong>${escape(tokens(day.total_tokens))}</strong><span class="sp-data-trust">EXACT tokens</span></summary><div><span>API equivalent</span><strong>${day.cost_basis === 'unknown' ? 'Unknown' : '$' + Number(day.cost_usd).toFixed(2)}</strong><span class="sp-data-trust">${day.cost_basis === 'unknown' ? 'UNKNOWN' : 'EST'}</span><span>${escape(day.turns)} invocations · EXACT</span></div></details>`).join('')}</div>
    <p class="sp-activity-caption">${days.length < report.daily.length ? 'Most recent 84 recorded days. ' : ''}Cache is counted once. No activity content is stored.</p>
  </section>`;
}

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
  const key = day => day.dataset.key || day.dataset.date;
  const open = new Set([...element.querySelectorAll('details[open]')].map(key));
  const focused = element.contains(document.activeElement) ? key(document.activeElement.closest('details') || { dataset: {} }) : null;
  const focusedAction = element.contains(document.activeElement) ? document.activeElement.dataset.sessionAction : null;
  const entering = !element.querySelector('.sp-activity-bars');
  element.innerHTML = markup;
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

export function renderQuota(pools, publishedAt, { illustrative = false } = {}) {
  const publication = publishedAt ? new Date(publishedAt).getTime() : NaN;
  const stale = !Number.isFinite(publication) || Date.now() - publication > 120000;
  return `<section class="sp-quota-section" aria-label="Provider limits"><div class="sp-activity-heading"><div><h2>Provider limits</h2><p>${illustrative ? 'Illustrative quota samples. Not connected to your account.' : stale ? 'Snapshot stale or publication time unknown. Values below are last observed.' : 'Latest received snapshot. Providers update limits when they emit usage.'}</p></div><span class="sp-data-trust">${illustrative ? 'SAMPLE' : stale ? 'STALE' : 'RECEIVED'}</span></div>
    ${pools?.length ? `<div class="sp-quota-grid">${pools.map(pool => {
      const known = pool.basis && pool.basis !== 'unknown' && typeof pool.used_pct === 'number' && Number.isFinite(pool.used_pct);
      const used = known ? Math.min(100, Math.max(0, pool.used_pct)) : 0;
      const reset = pool.resets_at ? new Date(pool.resets_at).getTime() : NaN;
      const observation = pool.observed_at ? new Date(pool.observed_at) : null;
      return `<article><header><strong>${escape(pool.provider)} / ${escape(pool.limit_id)}</strong><span>${escape(pool.window?.replaceAll('_', ' '))}</span></header><div class="sp-quota-number">${known ? used.toFixed(1) + '%' : 'Unknown'}<span class="sp-data-trust">${known ? trust(pool.basis) : 'UNKNOWN'} used</span></div><div class="sp-quota-track" aria-hidden="true"><i style="width:${used}%"></i></div><p>${Number.isFinite(pool.remaining_pct) ? pool.remaining_pct.toFixed(1) + '% remaining · ' + trust(pool.basis) : 'Remaining capacity unknown'}</p><p>${illustrative ? "Resets " + escape(pool.reset_label || "unknown") : Number.isFinite(reset) ? reset <= Date.now() ? 'Reset passed · awaiting provider update' : 'Resets ' + escape(new Date(reset).toLocaleString()) : 'Reset time unknown'}</p><small>Observed ${illustrative ? "sample" : observation && Number.isFinite(observation.getTime()) ? escape(observation.toLocaleString()) : 'unknown'}</small></article>`;
    }).join('')}</div>` : '<p>No measured quota pools are available. Remaining capacity is unknown.</p>'}</section>`;
}
