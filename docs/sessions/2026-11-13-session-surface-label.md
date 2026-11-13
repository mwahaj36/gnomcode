# 2026-11-13 - Surface label on session rows and the Workspace list

## Goals
- Show which product a session came from ("Antigravity IDE", "Antigravity CLI", "Antigravity 2.0", "Claude desktop app") as secondary text on Home session rows and in the Workspace conversation list, using the label Rust already computes (`core::surface_label`, #235). No label logic in TypeScript.

## Result
- `core::surface_label` also names the Claude and Codex desktop apps ("Claude desktop app", "Codex desktop app"); CLI sessions keep the plain agent name.
- `GET /api/v1/conversations` rows (the Workspace list) now carry `surface_label` taken from the observed session's surface; `/api/v1/sessions` (Home) already did.
- The presenters show `surface_label` in place of the agent name when it is present (`p · Antigravity IDE · model` on Home, `Claude desktop app · Idle` in the Workspace list) and fall back to the agent. Values are escaped like every other field.

## Tests
- `web/test/console-presentation.test.mjs` (label, fallback, escaping), `crates/adapters/src/antigravity_tests.rs` (label table), `web/e2e/surface.spec.ts` (a second daemon seeded with the Claude desktop fixture: Home rows, Workspace list and the API field). 79 browser tests in total.

## Verification
- `CARGO_INCREMENTAL=0` fmt, clippy `-D warnings`, `cargo test --workspace`; web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e`.
