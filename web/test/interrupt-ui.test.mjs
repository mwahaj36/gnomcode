import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const presentation = await import('../lib/console-presentation.js');

test('interrupt helpers strictly gate running states and bounded ASCII request IDs', () => {
  assert.equal(typeof presentation.isSessionRunningState, 'function');
  assert.equal(typeof presentation.createInterruptRequestId, 'function');
  assert.equal(typeof presentation.formatInterruptAckMessage, 'function');

  assert.equal(presentation.isSessionRunningState('running'), true);
  assert.equal(presentation.isSessionRunningState('thinking'), true);
  assert.equal(presentation.isSessionRunningState('tool'), true);
  assert.equal(presentation.isSessionRunningState('idle'), false);
  assert.equal(presentation.isSessionRunningState('awaiting_approval'), false);
  assert.equal(presentation.isSessionRunningState('waiting'), false);
  assert.equal(presentation.isSessionRunningState('ended'), false);

  const reqId = presentation.createInterruptRequestId();
  assert.match(reqId, /^[A-Za-z0-9_-]{8,64}$/);

  assert.match(presentation.formatInterruptAckMessage({ sent: true }), /Esc|Interrupt/i);
  assert.match(presentation.formatInterruptAckMessage({ sent: false }), /not|no longer/i);
});

test('interrupt button wires into Remote per Worker A PR #187 contract', () => {
  const tsx = readFileSync(join(root, 'web/components/RemoteCompanion.tsx'), 'utf8');
  const css = readFileSync(join(root, 'web/app/gnomcode-console.css'), 'utf8');

  assert.match(tsx, /sp-interrupt-btn/);
  assert.match(tsx, /type:\s*['"]interrupt['"]/);
  assert.match(tsx, /interrupt_ack/);
  assert.match(tsx, /isSessionRunningState/);
  assert.match(tsx, /Unsupported remote command/);

  const marker = css.indexOf('/* Section G5 interrupt button */');
  assert.notEqual(marker, -1, 'Expected Section G5 interrupt button CSS block');
  const sectionCss = css.slice(marker);
  assert.doesNotMatch(sectionCss, /#[0-9a-fA-F]{3,8}\b/, 'Use --sp-* tokens only');
});
