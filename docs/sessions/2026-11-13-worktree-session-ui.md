# 2026-11-13 - F1 Worktree per session UI (launch, diff review, merge/discard/Open PR)

## Goals
- Deliver the frontend UI for roadmap Section F item 1 (`Worktree per session with diff review and merge`) across the React local console (`WorkspaceView.tsx`) and encrypted Remote (`RemoteCompanion.tsx`) without putting Git logic in TypeScript.
- Document the daemon HTTP + E2EE protocol contract in ADR `0036` against the merged backend (ADR 0051).

## Outcomes
- Added `docs/adr/0036-worktree-session-ui-protocol.md` specifying `worktree` / `worktree_branch` launch fields, session metadata fields, reuse of `/api/v1/sessions/{id}/changes`, and confirmation-gated `worktree/merge`, `worktree/discard`, and `worktree/open-pr` actions.
- Added pure presentation helper `web/lib/worktree-presentation.mjs` (+ `.d.mts`) with HTML escaping, worktree badge formatting, and read-only status/unified diff rendering.
- Added "Isolate in git worktree" toggle + optional branch field in both `WorkspaceView.tsx` and `RemoteCompanion.tsx` launchers.
- Added `.sp-worktree-bar` showing the worktree branch/path, `Review diff`, and two-step inline-confirmed `Merge`, `Open PR`, and `Discard` controls showing the daemon's refusal messages.

## Dependencies & Assumptions
- Merge review: the backend is merged (ADR 0051). The UI calls the documented endpoints (`/api/v1/sessions/{id}/changes` and `/api/v1/sessions/{id}/worktree/{merge,discard,open-pr}`, plus E2EE WS equivalents) and shows the daemon's message verbatim; the earlier pending-backend wording was removed.

## Verification
- `node --test web/test/worktree-session-ui.test.mjs` (RED -> GREEN)
- `node scripts/build-local-console.mjs --check`
- `web/` (`npm test`, `npm run typecheck`, `npm run lint`, `npm run build`)
- Chromium headless CDP (`1440x900` & `390x844`, dark & light on `/#sessions` with worktree bar & expanded diff panel)
- Rust workspace gate (`CARGO_INCREMENTAL=0`: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`)
