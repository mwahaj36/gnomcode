import assert from 'node:assert/strict';
import test from 'node:test';
import { middleTruncate, renderProviderCards, shortPath, wslBadge, wslDistro, wslFolder } from '../lib/console-presentation.js';

const BS = String.fromCharCode(92);
const unc = [BS + BS + 'wsl.localhost', 'Ubuntu-22.04', 'home', 'a', 'proj'].join(BS);

test('wsl distro comes from share paths only', () => {
  assert.equal(wslDistro(unc), 'Ubuntu-22.04');
  assert.equal(wslDistro('//wsl$/Debian/x'), 'Debian');
  assert.equal(wslDistro('C:' + BS + 'x'), '');
  assert.equal(wslDistro('/home/wsl.localhost/x'), '');
});

test('linux folder maps to the share path', () => {
  assert.equal(wslFolder('Ubuntu-22.04', '/home/a/proj/'), unc);
  assert.equal(wslFolder('Ubuntu-22.04', 'relative'), '');
});

test('middle truncation keeps both ends', () => {
  const out = middleTruncate('a'.repeat(30) + 'b'.repeat(30), 21);
  assert.equal(out.length, 21);
  assert.ok(out.startsWith('aaaa') && out.endsWith('bbbb') && out.includes('...'));
  assert.equal(middleTruncate('short', 21), 'short');
});

test('rows get a distro badge only for WSL sessions', () => {
  assert.match(wslBadge({ repo_path: unc }), /WSL: Ubuntu-22\.04/);
  assert.equal(wslBadge({ repo_path: 'C:' + BS + 'p' }), '');
});

test('short paths name the distro instead of share segments', () => {
  assert.equal(shortPath(unc), 'WSL: Ubuntu-22.04 · ~');
  assert.equal(shortPath('C:\\Users\\W\\Documents\\GitHub\\x'), '~/Documents/GitHub');
});

test('provider cards list WSL sources on the source line', () => {
  const html = renderProviderCards([{ provider: 'claude-code', display_name: 'Claude Code', windows: [{ window: 'session', label: 'Session', used_pct: 40, remaining_pct: 60, basis: 'exact', state: 'ok' }], headline_window: 0, source_label: 'Status line', wsl_sources: ['WSL: Ubuntu-22.04'] }]);
  assert.match(html, /sp-wsl-badge[^>]*>WSL: Ubuntu-22\.04/);
});
