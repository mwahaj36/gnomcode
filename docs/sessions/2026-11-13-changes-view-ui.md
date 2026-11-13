# Read-only changes view UI on console and Remote

## Goals

Surface read-only git status (including untracked files) and unified diffs for the active session workspace on both the local console (`GET /api/v1/sessions/{id}/changes`, ADR 0037) and Remote phone companion (`{ type: 'changes', session_id }` E2EE contract).

## Outcomes

- Added pure status and diff formatters (`parseGitStatusLines`, `renderChangesView`) in `web/lib/console-presentation.js` and `web/lib/console-presentation.d.ts`, strictly escaping HTML, surfacing `lossy: true` notices, and omitting any mutation/revert controls.
- Added `#chat-changes-btn` and `#chat-changes-panel` to `crates/daemon/src/console.html` and `crates/daemon/src/console-app.js` using `/api/v1/sessions/{id}/changes` (`413` 1 MiB limit, `504` timeout, `404` no workspace, `400` non-git error handling; merge review switched from `/api/v1/repos/diff?repo=` so the daemon resolves the session workspace) and automatically hiding on session change.
- Added `.sp-remote-tools` (`Changes`) and `.sp-changes-sheet` bottom sheet to `web/components/RemoteCompanion.tsx`, with safe graceful fallback when a desktop returns `Unsupported remote command`.
- Appended `--sp-*` token CSS rules in `web/app/gnomcode-console.css` for `.sp-changes-panel`, `.sp-changes-sheet`, `.sp-diff-code`, and 44px mobile touch targets (hiding `.sp-remote-tools` at `max-height: 450px` so short keyboard viewports stay intact).

## Decisions

- Kept git execution, `GIT_OPTIONAL_LOCKS=0`, `-c core.quotepath=off`, 1 MiB / 10 s limits, and UTF-8 lossy detection exclusively in Rust; JS only formats and escapes the returned strings.
- Did not poll `{ type: 'changes' }` automatically; Remote requests changes only when the user opens or refreshes the sheet.

## Verification

- `web/test/changes-view.test.mjs` (TDD red -> green)
- `node scripts/check-local-console.mjs`
- `cd web && npm ci && npm test && npm run typecheck && npm run build`
- Headless Chromium (`chrome-headless-shell.exe`) visual + layout check at `390x844` and `1440x900` (dark & light).
- Full Rust workspace gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
