# 2026-11-13 - F3 Handoff quality eval report UI

## Goals
- Surface pre-computed handoff quality evaluation results (`GET /api/v1/evals/handoff`) in the local console Usage view (`UsageView.tsx`) while keeping all scoring and trust determination strictly in Rust (`AGENTS.md` §2.8).

## Outcomes
- Added `web/lib/eval-presentation.mjs` (+ `.d.mts`) with `renderHandoffEvalMarkup` to format pre-computed pass/fail outcomes, aggregate pass rates, trust chips (`EXACT`, `EST`, `API`, `UNKNOWN`), HTML-escaped summaries, and a calm empty state when eval data is absent.
- Integrated `<section className="sp-section sp-eval-report">` in `web/local-console/UsageView.tsx`, fetching `GET /api/v1/evals/handoff` alongside Usage refresh cycles and failing cleanly to an empty state on `404` or offline responses.
- Appended token-only responsive styling (`.sp-eval-*`) in `web/app/gnomcode-console.css`.
- Documented the API contract in `docs/adr/0038-handoff-eval-ui-contract.md`.

## Dependencies & Assumptions
- Merge review: the backend is merged (ADR 0055). The UI reads pre-computed JSON fields (`summary`, `cases`, `trust_level`, `pass_rate_pct`, `outcome`) without performing any evaluation or rate math in JavaScript.

## Verification
- `web/test/handoff-eval-ui.test.mjs` (RED -> GREEN)
- `node scripts/build-local-console.mjs` & `--check`
- `web/` (`npm test`, `npm run typecheck`, `npm run lint`, `npm run build`)
- Chromium headless CDP (`1440x900` & `390x844`, dark & light on `#usage` with mocked `/api/v1/evals/handoff` data)
- Rust workspace gate (`CARGO_INCREMENTAL=0`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
