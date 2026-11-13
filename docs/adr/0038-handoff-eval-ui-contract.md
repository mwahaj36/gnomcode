# ADR 0038: Handoff Quality Eval Report UI Contract (F3)

- **Status:** Accepted. UI contract only; the daemon and scorer are specified in [ADR 0055](./0055-handoff-quality-evals.md), which is authoritative where the two differ.
- **Date:** 2026-11-13
- **Owners:** Worker B (UI), Worker A (daemon & core eval harness)

## Context
Roadmap F3 calls for handoff quality evaluations visible to the user. AGENTS.md rule 8 requires 100% of business logic-including evaluation scoring, pass/fail determination, aggregate pass rates, and trust classifications-to remain in Rust (`crates/core` / `crates/daemon`), never in TypeScript/JavaScript.

## Decision
1. Expose a single read-only local HTTP endpoint `GET /api/v1/evals/handoff` from `crates/daemon` returning pre-computed evaluation summaries and per-case results:
   ```json
   {
     "updated_at": "2026-11-13T01:00:00Z",
     "summary": {
       "total_cases": 12,
       "passed_cases": 11,
       "pass_rate_pct": 91.7,
       "trust_level": "exact"
     },
     "cases": [
       {
         "id": "eval-1",
         "title": "Preserve dirty worktree context",
         "source_agent": "claude",
         "target_agent": "codex",
         "repo_name": "spawnt",
         "outcome": "passed",
         "score_pct": 100,
         "trust_level": "exact",
         "duration_ms": 1420,
         "summary": "Target session resumed with modified file list intact."
       }
     ]
   }
   ```
2. Render the report inside the Usage view (`web/local-console/UsageView.tsx`) using the pure, zero-calculation presenter `web/lib/eval-presentation.mjs`. Missing fields render as `N/A` and unknown trust levels render as `UNKNOWN`.
3. The daemon always serves a report (ADR 0055; it runs the embedded set when none is stored), so there is no 404. When the request fails or `cases` is empty, show a calm empty state that points to `gnomcode evals handoff` instead of blocking Usage rendering or fabricating client-side scores. The UI also shows the optional `summary.mean_score_pct`, per-case `packet_score_pct` and the `by_provider` rows from ADR 0055; every figure is estimated and renders as EST.

## Consequences
- Frontend performs zero scoring or aggregation math and preserves strict trust labels (`EXACT`, `EST`, `API`, `UNKNOWN`).
- The Rust eval runner (ADR 0055) supplies the report without UI rework.
