import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

// Invented, readable demo sessions for the README recording. No real user data.
const day = 86_400_000;
const iso = (ms: number) => new Date(ms).toISOString();

const claudeChats: Array<{ project: string; ago: number; turns: Array<[string, string]> }> = [
  {
    project: 'billing-service',
    ago: 0.05,
    turns: [
      ['Add retry with exponential backoff to the invoice webhook handler.', 'Added `retry_with_backoff` around the webhook POST: 5 attempts, 200 ms base delay, jitter, and a dead-letter log on final failure. Tests cover the 429 and timeout paths.'],
      ['Run the tests.', 'All 48 tests pass, including the 3 new retry cases.'],
    ],
  },
  {
    project: 'mobile-app',
    ago: 1.2,
    turns: [['Why does the settings screen flicker on Android?', 'The theme provider re-mounts on every navigation because its key changes. Memoizing the key fixes the flicker; I updated `ThemeRoot.tsx`.']],
  },
  {
    project: 'docs-site',
    ago: 3.5,
    turns: [['Fix the broken links in the getting started guide.', 'Fixed 6 links that pointed at the old `/v1/` paths and added a link check to CI.']],
  },
];

const codexChats: Array<{ project: string; ago: number; user: string; reply: string }> = [
  { project: 'billing-service', ago: 0.02, user: 'Continue the webhook retry work from the handoff brief.', reply: 'Picked up from the brief: added metrics for retry attempts and a dashboard query. Ready for review.' },
  { project: 'data-pipeline', ago: 2.3, user: 'Speed up the nightly export job.', reply: 'Batched the inserts and added an index on `exported_at`; the job dropped from 14 min to 3 min.' },
];

function claudeSession(cwd: string, start: number, turns: Array<[string, string]>, sessionId: string): string {
  const lines: string[] = [];
  let t = start;
  let parent: string | null = null;
  turns.forEach(([user, reply], i) => {
    const u = `${sessionId}-u${i}`;
    const a = `${sessionId}-a${i}`;
    lines.push(JSON.stringify({ parentUuid: parent, isSidechain: false, type: 'user', message: { role: 'user', content: [{ type: 'text', text: user }] }, uuid: u, timestamp: iso(t), entrypoint: 'cli', cwd, sessionId, version: '2.1.325', gitBranch: 'main' }));
    t += 40_000;
    lines.push(JSON.stringify({ parentUuid: u, isSidechain: false, type: 'assistant', message: { model: 'claude-opus-5', id: `msg-${a}`, type: 'message', role: 'assistant', content: [{ type: 'text', text: reply }], stop_reason: 'end_turn', usage: { input_tokens: 1200, cache_creation_input_tokens: 9000, cache_read_input_tokens: 42000 + i * 8000, output_tokens: 900 + i * 300 } }, requestId: `req-${a}`, uuid: a, timestamp: iso(t), entrypoint: 'cli', cwd, sessionId, version: '2.1.325', gitBranch: 'main' }));
    parent = a;
    t += 60_000;
  });
  return lines.join('\n') + '\n';
}

function codexSession(cwd: string, start: number, user: string, reply: string, id: string, now: number): string {
  const usage = { input_tokens: 58000, cached_input_tokens: 41000, cache_write_input_tokens: 0, output_tokens: 2100, reasoning_output_tokens: 600, total_tokens: 60100 };
  const s = Math.floor(now / 1000);
  return [
    { timestamp: iso(start), type: 'session_meta', payload: { id, timestamp: iso(start), cwd, originator: 'codex_cli_rs', cli_version: '0.165.0', model_provider: 'openai' } },
    { timestamp: iso(start + 1000), type: 'turn_context', payload: { cwd, model: 'gpt-6.1' } },
    { timestamp: iso(start + 2000), type: 'response_item', payload: { type: 'message', role: 'user', content: [{ type: 'input_text', text: user }] } },
    { timestamp: iso(start + 50_000), type: 'response_item', payload: { type: 'message', role: 'assistant', content: [{ type: 'output_text', text: reply }] } },
    { timestamp: iso(start + 51_000), type: 'event_msg', payload: { type: 'token_count', info: { total_token_usage: usage, last_token_usage: usage, model_context_window: 400000 }, rate_limits: { limit_id: 'codex', primary: { used_percent: 38, window_minutes: 300, resets_at: s + 2 * 3600 + 900 }, secondary: { used_percent: 61, window_minutes: 10080, resets_at: s + 3 * 86400 }, plan_type: 'plus' } } },
  ].map((l) => JSON.stringify(l)).join('\n') + '\n';
}

/** Writes the demo agent logs into an isolated home used by `startDaemon`. */
export async function seedDemo(home: string): Promise<void> {
  const now = Date.now();
  for (const [i, c] of claudeChats.entries()) {
    const dir = path.join(home, 'claude', 'projects', `-work-${c.project}`);
    await mkdir(dir, { recursive: true });
    const id = `d${i}000000-0000-4000-8000-00000000000${i}`;
    await writeFile(path.join(dir, `${id}.jsonl`), claudeSession(`/work/${c.project}`, now - c.ago * day, c.turns, id));
  }
  for (const [i, c] of codexChats.entries()) {
    const start = now - c.ago * day;
    const d = new Date(start);
    const dir = path.join(home, 'codex', 'sessions', String(d.getUTCFullYear()), String(d.getUTCMonth() + 1).padStart(2, '0'), String(d.getUTCDate()).padStart(2, '0'));
    await mkdir(dir, { recursive: true });
    const id = `e${i}000000-0000-4000-8000-00000000000${i}`;
    await writeFile(path.join(dir, `rollout-${d.toISOString().slice(0, 19).replace(/:/g, '-')}-${id}.jsonl`), codexSession(`/work/${c.project}`, start, c.user, c.reply, id, now));
  }
}
