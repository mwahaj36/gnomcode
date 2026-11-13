function escape(value) {
  return String(value ?? '').replace(/[&<>"']/g, (char) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  })[char]);
}

const STATUS_LABELS = {
  '??': 'Untracked',
  A: 'Added',
  M: 'Modified',
  D: 'Deleted',
  R: 'Renamed',
  C: 'Copied',
  U: 'Conflict',
};

function parseGitStatus(statusText = '') {
  if (typeof statusText !== 'string' || !statusText) return [];
  const rows = [];
  for (const rawLine of statusText.split(/\r?\n/)) {
    if (!rawLine || rawLine.length < 3) continue;
    const xy = rawLine.slice(0, 2);
    const trimmed = xy.trim() || 'M';
    const key = xy === '??' ? '??' : trimmed[0];
    const filePath = rawLine.slice(3).trim();
    if (!filePath) continue;
    rows.push({ code: key, label: STATUS_LABELS[key] || 'Changed', path: filePath });
  }
  return rows;
}

function classifyDiffLine(line) {
  if (
    line.startsWith('diff --git ') ||
    line.startsWith('index ') ||
    line.startsWith('+++ ') ||
    line.startsWith('--- ')
  ) {
    return 'sp-diff-line sp-diff-line--meta';
  }
  if (line.startsWith('@@')) return 'sp-diff-line sp-diff-line--hunk';
  if (line.startsWith('+')) return 'sp-diff-line sp-diff-line--add';
  if (line.startsWith('-')) return 'sp-diff-line sp-diff-line--del';
  return 'sp-diff-line';
}

export function formatWorktreeBadge(session) {
  if (!session) return '';
  if (typeof session.worktree_branch === 'string' && session.worktree_branch.trim()) {
    return `Worktree · ${session.worktree_branch.trim()}`;
  }
  if (session.worktree || session.worktree_path) {
    return 'Worktree isolated';
  }
  return '';
}

export function renderWorktreeDiffMarkup(data) {
  if (!data) {
    return '<p class="sp-provider-empty">Select Review diff to inspect worktree changes.</p>';
  }
  if (data.loading) {
    return '<p class="sp-provider-empty" role="status">Loading worktree changes...</p>';
  }
  if (data.error) {
    return `<p class="sp-field-note sp-tone-red" role="alert">${escape(data.error)}</p>`;
  }
  const files = parseGitStatus(data.status || '');
  const diffText = typeof data.diff === 'string' ? data.diff : '';
  const badges = [
    '<span class="sp-chip sp-chip--neutral">Read-only diff</span>',
    data.lossy
      ? '<span class="sp-chip sp-chip--low" title="Non-UTF-8 bytes were replaced by Rust">Lossy UTF-8 decoding</span>'
      : '',
    data.branch ? `<span class="sp-chip sp-chip--neutral">${escape(data.branch)}</span>` : '',
    data.repo ? `<span class="sp-changes-repo" title="${escape(data.repo)}">${escape(data.repo)}</span>` : '',
  ]
    .filter(Boolean)
    .join('');
  if (!files.length && !diffText.trim()) {
    return `<div class="sp-changes-meta">${badges}</div><p class="sp-provider-empty">No uncommitted changes in this worktree.</p>`;
  }
  const fileMarkup = files.length
    ? `<ul class="sp-changes-files" aria-label="Changed files">${files
        .map(
          (file) =>
            `<li class="sp-changes-file"><span class="sp-chip sp-chip--neutral">${escape(file.label)}</span><code>${escape(file.path)}</code></li>`,
        )
        .join('')}</ul>`
    : '';
  const diffLines = diffText.trim()
    ? diffText
        .split(/\r?\n/)
        .map((line) => `<span class="${classifyDiffLine(line)}">${escape(line) || ' '}</span>`)
        .join('')
    : '';
  const diffMarkup = diffLines
    ? `<pre class="sp-diff-code" tabindex="0" aria-label="Worktree git diff">${diffLines}</pre>`
    : '<p class="sp-provider-empty">Untracked files only (no tracked diff hunks yet).</p>';
  return `<div class="sp-changes-meta">${badges}</div>${fileMarkup}${diffMarkup}`;
}
