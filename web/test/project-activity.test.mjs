import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { projectsFromSessions, renderProjectRows, renderProjects, shortPath } from '../lib/console-presentation.js';

const day = n => new Date(Date.UTC(2026, 10, 13 - n)).toISOString();
const many = count => Array.from({ length: count }, (_, i) => ({ id: `p${i}`, name: `project-${i}`, sessions: i + 1, last_active: day(i), agents: ['codex'], repo_root: `C:\\Users\\Wahaj\\Documents\\GitHub\\project-${i}` }));

test('projects come from synced sessions with token totals, newest first, without the unassigned bucket', () => {
  const projects = projectsFromSessions([
    { project_name: 'alpha', agent: 'codex', total_tokens: 1500, last_event_at: day(3) },
    { project_name: 'alpha', agent: 'claude-code', total_tokens: 2500, last_event_at: day(1) },
    { project_name: 'unknown', agent: 'codex', total_tokens: 999, last_event_at: day(0) },
    { project_name: '', agent: 'codex', last_event_at: day(0) },
    { project_name: 'beta', agent: 'antigravity', total_tokens: 500, last_event_at: day(2) },
  ]);
  assert.deepEqual(projects.map(project => [project.name, project.sessions, project.total_tokens, project.last_active, project.agents.join('+')]), [
    ['alpha', 2, 4000, day(1), 'codex+claude-code'],
    ['beta', 1, 500, day(2), 'antigravity'],
  ]);
  const rows = renderProjectRows(projects);
  assert.ok(rows.indexOf('alpha') < rows.indexOf('beta'));
  assert.doesNotMatch(rows, /data-project-id="unknown"/i);
  assert.match(rows, /Codex, Claude Code/);
  assert.match(rows, /<b>4K<\/b><span> tokens</);
  assert.match(rows, /<b>2<\/b><span> sessions</);
  assert.match(rows, /<b>1<\/b><span> session</);
});

test('folders are shortened for one line and keep the full path as the tooltip', () => {
  assert.equal(shortPath('C:\\Users\\Wahaj\\Documents\\GitHub\\fleethud'), '~/Documents/GitHub');
  assert.equal(shortPath('/home/dev/code/deep/er/app'), '.../code/deep/er');
  assert.equal(shortPath('D:\\Games'), 'D:');
  assert.equal(shortPath('app'), '');
  const html = renderProjectRows([{ id: 'a', name: 'fleethud', sessions: 3, last_active: day(1), agents: ['codex'], repo_root: 'C:\\Users\\Wahaj\\Documents\\GitHub\\fleethud' }]);
  assert.match(html, /title="C:\\Users\\Wahaj\\Documents\\GitHub\\fleethud"/);
  assert.match(html, /<span>~\/Documents\/GitHub<\/span>/);
});

test('filter is always shown when projects exist and rows paginate cleanly beyond ten', () => {
  const ten = renderProjects(many(10));
  assert.equal((ten.match(/class="sp-project-row"/g) || []).length, 10);
  assert.match(ten, /class="sp-project-search"/);
  assert.doesNotMatch(ten, /sp-project-pager/);
  const twelve = renderProjects(many(12));
  assert.equal((twelve.match(/class="sp-project-row"/g) || []).length, 10);
  assert.match(twelve, /class="sp-project-search"/);
  assert.match(twelve, /Page 1 of 2/);
  assert.match(twelve, /Showing 1–10 of 12/);
  assert.doesNotMatch(twelve, /project-11/);
  const page2 = renderProjectRows(many(12), { page: 2 });
  assert.equal((page2.match(/class="sp-project-row"/g) || []).length, 2);
  assert.match(page2, /Page 2 of 2/);
  assert.match(page2, /project-11/);
  const filtered = renderProjectRows(many(12), { query: 'PROJECT-1' });
  assert.equal((filtered.match(/class="sp-project-row"/g) || []).length, 3, 'project-1, -10 and -11, matched case-insensitively');
  assert.doesNotMatch(filtered, /sp-project-pager/);
  assert.match(renderProjectRows(many(12), { query: 'zzz' }), /No project matches "zzz"/);
});

test('names and paths are escaped and an empty list says so in one line', () => {
  const html = renderProjectRows([{ id: 'x"><img>', name: '<img src=x onerror=1>', sessions: 1, last_active: day(0), agents: ['<i>x'], repo_root: 'C:\\a"><script>\\b' }]);
  assert.doesNotMatch(html, /<img|<script|<i>/);
  assert.match(renderProjectRows([]), /No project activity recorded yet/);
  assert.match(renderProjects(undefined), /remain local/);
});

test('web and local consoles share the list and the CSS stays in its own section', async () => {
  const client = await readFile(new URL('../app/console/Client.tsx', import.meta.url), 'utf8');
  const local = await readFile(new URL('../local-console/UsageView.tsx', import.meta.url), 'utf8');
  const css = await readFile(new URL('../app/gnomcode-console.css', import.meta.url), 'utf8');
  assert.match(client, /mountProjectList\(element\.current, projectsFromSessions\(sessions\)/);
  assert.match(local, /mountProjectList\(/);
  assert.match(css, /Project activity \(fix\/project-activity\)/);
  assert.match(css, /\.sp-projects \.sp-project-row \{[^}]*min-height: 44px/);
});
