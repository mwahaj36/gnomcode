import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const testDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = join(testDir, '..', '..');

test('F3 handoff eval UI renders Rust-computed scores, trust labels, empty fallback, and escapes text', async () => {
  const { renderHandoffEvalMarkup } = await import('../lib/eval-presentation.mjs');

  const emptyMarkup = renderHandoffEvalMarkup(null);
  assert.match(emptyMarkup, /sp-eval-empty/);
  assert.match(emptyMarkup, /gnomcode evals handoff/);

  const populatedMarkup = renderHandoffEvalMarkup({
    updated_at: '2026-11-13T01:00:00Z',
    summary: {
      total_cases: 4,
      passed_cases: 3,
      pass_rate_pct: 75,
      trust_level: 'estimated',
    },
    cases: [
      {
        id: 'eval-1',
        title: 'Preserve dirty tree <script>alert(1)</script>',
        source_agent: 'claude',
        target_agent: 'codex',
        repo_name: 'spawnt',
        outcome: 'passed',
        score_pct: 100,
        trust_level: 'exact',
        duration_ms: 1420,
        summary: 'Kept branch state cleanly',
      },
      {
        id: 'eval-2',
        title: 'Resume tool context',
        source_agent: 'codex',
        target_agent: 'gemini',
        outcome: 'failed',
        trust_level: 'unknown',
        summary: 'Dropped pending file ref',
      },
    ],
  });

  assert.match(populatedMarkup, /EST/);
  assert.match(renderHandoffEvalMarkup({ cases: [{ id: 'a', outcome: 'passed' }], by_provider: [{ target_agent: 'codex', cases: 2, passed_cases: 1, pass_rate_pct: 50 }], summary: { mean_score_pct: 80.5 } }), /Mean score 80.5%[\s\S]*codex<\/strong><span>1 \/ 2 passed \(50%\)/);
  assert.match(populatedMarkup, /3 \/ 4 passed/);
  assert.match(populatedMarkup, /75%/);
  assert.match(populatedMarkup, /PASS/);
  assert.match(populatedMarkup, /FAIL/);
  assert.match(populatedMarkup, /claude -&gt; codex/);
  assert.doesNotMatch(populatedMarkup, /<script>alert\(1\)<\/script>/);
  assert.match(populatedMarkup, /&lt;script&gt;alert\(1\)&lt;\/script&gt;/);

  const usageViewSrc = readFileSync(join(repoRoot, 'web', 'local-console', 'UsageView.tsx'), 'utf8');
  assert.match(usageViewSrc, /\/api\/v1\/evals\/handoff/);
  assert.match(usageViewSrc, /eval-presentation\.mjs/);
  assert.match(usageViewSrc, /Handoff quality evals/i);
});
