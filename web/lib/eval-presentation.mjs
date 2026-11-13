function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function formatTrustBadge(level) {
  switch (String(level || '').toLowerCase()) {
    case 'exact':
      return 'EXACT';
    case 'estimated':
      return 'EST';
    case 'api':
      return 'API';
    default:
      return 'UNKNOWN';
  }
}

function normalizeOutcome(outcome) {
  const norm = String(outcome || '').toLowerCase();
  if (norm === 'passed' || norm === 'pass') return { key: 'passed', label: 'PASS' };
  if (norm === 'failed' || norm === 'fail') return { key: 'failed', label: 'FAIL' };
  return { key: 'unknown', label: 'UNKNOWN' };
}

/**
 * Render pre-computed Rust handoff quality evaluation report data as escaped HTML.
 * Never computes or estimates pass rates or scores in JavaScript.
 * @param {any} report
 * @returns {string}
 */
export function renderHandoffEvalMarkup(report) {
  if (!report || !Array.isArray(report.cases) || report.cases.length === 0) {
    return (
      '<div class="sp-empty sp-eval-empty">' +
      '<p>No handoff quality eval report is available. Run <code>gnomcode evals handoff</code> to create one.</p>' +
      '</div>'
    );
  }

  const summary = report.summary && typeof report.summary === 'object' ? report.summary : {};
  const trustLabel = formatTrustBadge(summary.trust_level);
  const totalCases = typeof summary.total_cases === 'number' && Number.isFinite(summary.total_cases)
    ? String(summary.total_cases)
    : String(report.cases.length);
  const passedCases = typeof summary.passed_cases === 'number' && Number.isFinite(summary.passed_cases)
    ? String(summary.passed_cases)
    : 'N/A';
  const passRate = typeof summary.pass_rate_pct === 'number' && Number.isFinite(summary.pass_rate_pct)
    ? `${summary.pass_rate_pct}%`
    : 'N/A';
  const meanScore = typeof summary.mean_score_pct === 'number' && Number.isFinite(summary.mean_score_pct)
    ? `<span class="sp-eval-meta-item">Mean score ${escapeHtml(summary.mean_score_pct)}%</span>`
    : '';
  const providerRows = Array.isArray(report.by_provider)
    ? report.by_provider.map((row) => {
      const rate = typeof row?.pass_rate_pct === 'number' && Number.isFinite(row.pass_rate_pct) ? `${row.pass_rate_pct}%` : 'N/A';
      return `<li class="sp-eval-provider"><strong>${escapeHtml(row?.target_agent || 'unknown')}</strong><span>${escapeHtml(row?.passed_cases ?? 'N/A')} / ${escapeHtml(row?.cases ?? 'N/A')} passed (${escapeHtml(rate)})</span></li>`;
    }).join('')
    : '';
  const providerList = providerRows ? `<ul class="sp-eval-providers" aria-label="Pass rate by receiving provider">${providerRows}</ul>` : '';
  const updatedText = typeof report.updated_at === 'string' && report.updated_at
    ? `<span class="sp-eval-updated">Updated ${escapeHtml(report.updated_at)}</span>`
    : '';

  const rows = report.cases.map((item) => {
    const title = escapeHtml(item?.title || item?.id || 'Untitled eval case');
    const sourceAgent = escapeHtml(item?.source_agent || 'unknown');
    const targetAgent = escapeHtml(item?.target_agent || 'unknown');
    const repoName = item?.repo_name ? `<span class="sp-eval-meta-item">${escapeHtml(item.repo_name)}</span>` : '';
    const outcome = normalizeOutcome(item?.outcome);
    const caseTrust = formatTrustBadge(item?.trust_level);
    const scoreText = typeof item?.score_pct === 'number' && Number.isFinite(item.score_pct)
      ? `<span class="sp-eval-meta-item">Score: ${escapeHtml(item.score_pct)}%</span>`
      : '';
    const packetText = typeof item?.packet_score_pct === 'number' && Number.isFinite(item.packet_score_pct)
      ? `<span class="sp-eval-meta-item">Packet ${escapeHtml(item.packet_score_pct)}%</span>`
      : '';
    const durationText = typeof item?.duration_ms === 'number' && Number.isFinite(item.duration_ms)
      ? `<span class="sp-eval-meta-item">${escapeHtml(item.duration_ms)} ms</span>`
      : '';
    const summaryLine = item?.summary
      ? `<p class="sp-eval-case-summary">${escapeHtml(item.summary)}</p>`
      : '';

    return (
      `<article class="sp-eval-case">` +
      `<header class="sp-eval-case-head">` +
      `<strong class="sp-eval-case-title">${title}</strong>` +
      `<div class="sp-eval-case-badges">` +
      `<span class="sp-chip sp-eval-outcome sp-eval-outcome--${outcome.key}">${outcome.label}</span>` +
      `<span class="sp-chip sp-eval-trust">${caseTrust}</span>` +
      `</div>` +
      `</header>` +
      `<div class="sp-eval-case-meta">` +
      `<span class="sp-eval-route">${sourceAgent} -&gt; ${targetAgent}</span>` +
      repoName +
      scoreText +
      packetText +
      durationText +
      `</div>` +
      summaryLine +
      `</article>`
    );
  }).join('');

  return (
    `<div class="sp-eval-summary-bar">` +
    `<div class="sp-eval-summary-metrics">` +
    `<strong>${escapeHtml(passedCases)} / ${escapeHtml(totalCases)} passed (${escapeHtml(passRate)})</strong>` +
    `<span class="sp-chip sp-eval-trust">${trustLabel}</span>` +
    meanScore +
    `</div>` +
    updatedText +
    `</div>` +
    providerList +
    `<div class="sp-eval-list">${rows}</div>`
  );
}
