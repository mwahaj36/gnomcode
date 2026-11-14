'use strict';

const assert = require('node:assert/strict');
const { mkdtempSync, readFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const path = require('node:path');
const test = require('node:test');

test('parses Windows Registry Run and StartupApproved output cleanly', () => {
  const {
    isWindowsStartupApprovedDisabled,
    parseWindowsRegQueryValue,
    windowsAutostartCommand,
  } = require('../dist/autostart');

  const sampleRun = [
    '',
    'HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Run',
    '    Gnomcode    REG_SZ    "C:\\Users\\Ada\\AppData\\Local\\Gnomcode\\desktop\\gnomcode-desktop.exe" --background',
    '',
  ].join('\r\n');
  assert.equal(
    parseWindowsRegQueryValue(sampleRun),
    '"C:\\Users\\Ada\\AppData\\Local\\Gnomcode\\desktop\\gnomcode-desktop.exe" --background',
  );
  assert.equal(
    windowsAutostartCommand('C:\\App\\gnomcode-desktop.exe'),
    '"C:\\App\\gnomcode-desktop.exe" --background',
  );

  const disabledApproved = '    Gnomcode    REG_BINARY    0300000012345678\r\n';
  const enabledApproved = '    Gnomcode    REG_BINARY    0200000000000000\r\n';
  assert.equal(isWindowsStartupApprovedDisabled(disabledApproved), true);
  assert.equal(isWindowsStartupApprovedDisabled(enabledApproved), false);
});

test('manages Linux desktop autostart files cleanly inside XDG_CONFIG_HOME', () => {
  const { getLoginItemEnabled, setLoginItemEnabled } = require('../dist/autostart');
  const root = mkdtempSync(path.join(tmpdir(), 'gnomcode-autostart-'));
  try {
    const env = { XDG_CONFIG_HOME: root };
    const noopAdapter = {
      getLoginItemSettings: () => ({ openAtLogin: false }),
      setLoginItemSettings: () => {},
    };
    assert.equal(getLoginItemEnabled(noopAdapter, 'linux', env, root), false);
    assert.equal(
      setLoginItemEnabled(true, noopAdapter, '/opt/gnomcode/gnomcode-desktop', 'linux', env, root),
      true,
    );
    const entry = readFileSync(path.join(root, 'autostart', 'gnomcode.desktop'), 'utf8');
    assert.match(entry, /Exec="\/opt\/gnomcode\/gnomcode-desktop" --background/);
    assert.equal(
      setLoginItemEnabled(false, noopAdapter, '/opt/gnomcode/gnomcode-desktop', 'linux', env, root),
      false,
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
