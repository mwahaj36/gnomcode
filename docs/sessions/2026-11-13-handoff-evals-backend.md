# 2026-11-13 - Handoff quality evals backend (F3)

## Goals
- Build the Rust side of roadmap F3 that Worker B's UI (PR #211) renders: a repeatable synthetic eval set, a scorer, `GET /api/v1/evals/handoff` and `gnomcode evals handoff`, with CI as a report.

## Outcomes
- `gnomcode_core::handoff_eval` (pure scorer and report builder), `gnomcode_daemon::handoff_eval` (fixture runner, stored latest report, endpoint), `gnomcode evals handoff` (report, `--json`, `--dir`, `--no-save`), nine synthetic cases under `fixtures/evals/handoff`, and a non-gating `handoff-evals` CI job.
- First baseline: 8 of 9 cases pass, mean 94.2%, all estimated. The failing case is `long-session`, where the original goal and file fall outside the newest 50 events that a handoff carries. The zero-token packet summary scores lower than the delivered context on every case because it holds no assistant statements, so open tasks never appear in it.
- ADR 0055, SPEC 3.14.1 paragraph and ADR row, roadmap F3 ticked (backend).

## Problems encountered
- The bash tool mangled backslashes and long heredocs while writing fixtures and Rust; files were written with the editor tool instead.
- `continuation_prompt` was private and the 50-event limit was a literal in the delivery path. Both are now shared (`pub(crate)` and `HANDOFF_CONTEXT_EVENTS`) so the eval measures the real handoff.

## Decisions
- Score what the target agent receives, not a separate summary format, and report the packet summary only as information.
- Label every figure `estimated`: coverage of synthetic phrases is a proxy.
- Serve a freshly run report when none is stored, so the Usage view shows data without a CLI step; no 404.
- New ADR 0055 instead of editing ADR 0038, which is not on `main` yet.

## Verification
- `cargo test -p gnomcode-core handoff_eval` (9 tests, written red first against a stub scorer), `cargo test -p gnomcode-daemon handoff_eval` (8), `cargo test -p gnomcode evals` (1).
- `gnomcode evals handoff` run against a temp `GNOMCODE_HOME`. Full gate: see PR.
