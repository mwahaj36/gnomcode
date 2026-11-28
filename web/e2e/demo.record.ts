import { mkdir } from 'node:fs/promises';
import path from 'node:path';
import { seedDemo } from './demo-data';
import { startDaemon, type Daemon } from './daemon';
import { test as base } from './fixtures';

// Captures the README demo frames from the real console seeded with invented demo sessions.
// Run: npx playwright test -c playwright.demo.config.ts, then scripts/make-demo-gif.ps1.
const out = path.resolve(__dirname, '..', '..', 'target', 'demo-frames');

const test = base.extend<object, { daemon: Daemon }>({
  daemon: [
    async ({}, use) => {
      const daemon = await startDaemon({ seedWith: seedDemo });
      await use(daemon);
      await daemon.stop();
    },
    { scope: 'worker' },
  ],
});

type Card = { provider: string; windows: Array<Record<string, unknown>>; [k: string]: unknown };

// Claude and Antigravity limits come from vendor APIs that a demo cannot call, so their cards
// borrow the Codex card's shape (built by the real daemon from the demo logs) with demo numbers.
function demoLimits(cards: Card[]): Card[] {
  const codex = cards.find((c) => c.provider === 'codex');
  if (!codex?.windows?.length) return cards;
  const used: Record<string, [number, number]> = { 'claude-code': [72, 44], antigravity: [15, 27] };
  return cards.map((c) => {
    const pct = used[c.provider];
    if (!pct) return c;
    const windows = codex.windows.map((w, i) => {
      const used = pct[i] ?? pct[0];
      return { ...w, provider: c.provider, used_pct: used, remaining_pct: 100 - used };
    });
    return { ...codex, provider: c.provider, display_name: c.display_name, plan: c.provider === 'claude-code' ? 'max' : 'pro', windows, enable_hint: null, enable_command: null, source_label: 'Provider API', cost: c.cost ?? codex.cost };
  });
}

test('record README demo frames', async ({ page, openConsole }) => {
  await mkdir(out, { recursive: true });
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.route('**/api/v1/providers/overview*', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    const cards = Array.isArray(body) ? body : body.providers;
    const patched = demoLimits(cards);
    await route.fulfill({ response, json: Array.isArray(body) ? patched : { ...body, providers: patched } });
  });
  let n = 0;
  const shot = async () => {
    await page.waitForTimeout(900);
    await page.screenshot({ path: path.join(out, `frame-${String(n++).padStart(2, '0')}.png`) });
  };
  await openConsole('#overview');
  await shot();
  await openConsole('#usage');
  await shot();
  await openConsole('#sessions');
  await page.locator('.ide-session', { hasText: 'Add retry with exponential' }).first().click();
  await shot();
  await page.locator('.ide-session', { hasText: 'settings screen flicker' }).first().click();
  await shot();
});
