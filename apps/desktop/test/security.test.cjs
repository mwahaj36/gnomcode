const test = require('node:test');
const assert = require('node:assert/strict');
const { daemonUrl, isDaemonNavigation } = require('../dist/security.js');

test('accepts only valid daemon metadata and generates a loopback URL', () => {
  assert.equal(daemonUrl({ port: 41738, token: 'a'.repeat(64) }), `http://127.0.0.1:41738/?token=${'a'.repeat(64)}`);
  for (const value of [null, {}, {port: 0, token: 'a'.repeat(64)}, {port: 65536, token: 'a'.repeat(64)}, {port: 41738, token: 'bad'}, {port: '41738', token: 'a'.repeat(64)}]) {
    assert.equal(daemonUrl(value), null);
  }
});

test('navigation cannot escape the selected loopback daemon', () => {
  assert.equal(isDaemonNavigation('http://127.0.0.1:41738/#sessions', 41738), true);
  for (const url of ['https://127.0.0.1:41738/', 'http://localhost:41738/', 'http://127.0.0.1:1234/', 'https://example.com/', 'javascript:alert(1)', 'http://user@127.0.0.1:41738/']) {
    assert.equal(isDaemonNavigation(url, 41738), false);
  }
});

test('daemon inherits its environment', () => {
  const { daemonEnvironment } = require('../dist/security.js');
  assert.deepEqual(daemonEnvironment({ GNOMCODE_HOME: '/test/new', OTHER: 'kept' }), { GNOMCODE_HOME: '/test/new', OTHER: 'kept' });
});

test('tray status shows daemon-provided ongoing counts and handles malformed data', () => {
  const { trayTooltip } = require('../dist/security.js');
  assert.equal(trayTooltip({ ongoing_count: 0 }), 'No ongoing sessions (estimated)');
  assert.equal(trayTooltip({ ongoing_count: 1 }), '1 ongoing session (estimated)');
  assert.equal(trayTooltip({ ongoing_count: 3 }), '3 ongoing sessions (estimated)');
  // awaiting_approval_count surfaced in tooltip
  assert.equal(trayTooltip({ ongoing_count: 2, awaiting_approval_count: 1 }), '2 ongoing sessions · 1 awaiting approval (estimated)');
  assert.equal(trayTooltip({ ongoing_count: 1, awaiting_approval_count: 0 }), '1 ongoing session (estimated)');
  for (const value of [null, {}, {ongoing_count: -1}, {ongoing_count: 1.5}, {ongoing_count: '3'}]) {
    assert.equal(trayTooltip(value), 'Gnomcode: daemon unavailable');
  }
});

test('tray count selects a numbered icon only for ongoing sessions', () => {
  const { trayBadgeName } = require('../dist/security.js');
  assert.equal(trayBadgeName({ ongoing_count: 0 }), 'gnomcode-tray.ico');
  assert.equal(trayBadgeName({ ongoing_count: 3 }), 'gnomcode-tray-3.png');
  assert.equal(trayBadgeName({ ongoing_count: 27 }), 'gnomcode-tray-9plus.png');
  assert.equal(trayBadgeName({ ongoing_count: '3' }), 'gnomcode-tray.ico');
});

test('renderer requests are limited to local daemon resources and local generated blobs', () => {
  const { isAllowedResource } = require('../dist/security.js');
  assert.equal(isAllowedResource('http://127.0.0.1:41738/gnomcode-tokens.css', 41738), true);
  assert.equal(isAllowedResource('blob:http://127.0.0.1:41738/123', 41738), true);
  assert.equal(isAllowedResource('data:image/png;base64,AA==', 41738), true);
  for (const url of ['https://example.com/a', 'http://localhost:41738/', 'blob:https://example.com/id', 'file:///etc/passwd', 'javascript:alert(1)']) {
    assert.equal(isAllowedResource(url, 41738), false);
  }
});

test('default HTTP port is treated as port 80 for local navigation', () => {
  assert.equal(isDaemonNavigation('http://127.0.0.1:80/health', 80), true);
});

test('terminal resources allow only the daemon PTY WebSocket while navigation stays HTTP-only', () => {
  const { isAllowedResource } = require('../dist/security.js');
  const terminal = 'ws://127.0.0.1:41738/api/v1/sessions/codex-host/pty?token=test';
  assert.equal(isAllowedResource(terminal, 41738), true);
  assert.equal(isDaemonNavigation(terminal, 41738), false);
  for (const url of [
    terminal.replace('41738', '1234'), terminal.replace('127.0.0.1', 'localhost'),
    terminal.replace('127.0.0.1', 'example.com'), terminal.replace('ws:', 'wss:'),
    terminal.replace('/pty?', '/stop?'), terminal.replace('/api/v1/sessions/codex-host/pty', '/other'),
    terminal.replace('127.0.0.1', 'user@127.0.0.1'), `${terminal}#fragment`,
  ]) assert.equal(isAllowedResource(url, 41738), false, url);
});

test('external links open only approved product pages without embedded credentials or query data', () => {
  const { isAllowedExternalUrl } = require('../dist/security.js');
  assert.equal(isAllowedExternalUrl('https://gnomcode.vercel.app/docs'), true);
  assert.equal(isAllowedExternalUrl('https://gnomcode.vercel.app/pricing'), true);
  for (const value of ['https://evil.example/', 'http://gnomcode.vercel.app/', 'https://user@gnomcode.vercel.app/', 'https://gnomcode.vercel.app/docs?token=secret', 'https://gnomcode.vercel.app:444/']) {
    assert.equal(isAllowedExternalUrl(value), false);
  }
});

test('external links do not carry arbitrary fragments', () => {
  const { isAllowedExternalUrl } = require('../dist/security.js');
  assert.equal(isAllowedExternalUrl('https://gnomcode.vercel.app/docs#privacy'), true);
  assert.equal(isAllowedExternalUrl('https://gnomcode.vercel.app/docs#secret'), false);
});

test('desktop chrome uses themed native window controls on each platform', () => {
 const {windowChrome}=require('../dist/security.js');
 for (const platform of ['win32','linux']) {
  const dark=windowChrome(platform,true),light=windowChrome(platform,false);
  assert.equal(dark.titleBarStyle,'hidden');assert.equal(dark.titleBarOverlay.height,32);
  assert.notEqual(dark.titleBarOverlay.color,light.titleBarOverlay.color);
  assert.notEqual(dark.titleBarOverlay.symbolColor,dark.titleBarOverlay.color);
 }
 assert.equal(windowChrome('darwin',true).titleBarStyle,'hiddenInset');
});

test('native notifications require explicit bounded arguments', () => {
  const { desktopNotification } = require('../dist/security.js');
  assert.deepEqual(desktopNotification(['app','--gnomcode-notification','Response ready']),{title:'Gnomcode',body:'Response ready'});
  assert.equal(desktopNotification(['app','--gnomcode-notification']),null);
  assert.equal(desktopNotification(['app','--gnomcode-notification','x'.repeat(513)]),null);
  assert.equal(desktopNotification(['app','ordinary launch']),null);
});

test('workspace activation accepts only the local conversation route',()=>{
 const {workspaceLaunchHash}=require('../dist/security.js');
 assert.equal(workspaceLaunchHash(['app','--gnomcode-open-hash','#sessions?folder=C%3A%5Cproject']),'#sessions?folder=C%3A%5Cproject');
 for(const value of ['javascript:alert(1)','#auth','#sessions?token=secret','https://example.com'])assert.equal(workspaceLaunchHash(['app','--gnomcode-open-hash',value]),null);
});
