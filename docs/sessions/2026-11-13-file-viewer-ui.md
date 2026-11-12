# Read-only workspace file viewer UI on console and Remote

## Goals

Allow reading workspace-contained files safely and read-only from both the local console (`GET /api/v1/repos/file` + optional `GET /api/v1/repos/files`) and Remote phone companion (`repo_files` / `repo_file` E2EE contract).

## Outcomes

- Added `normalizeWorkspaceRelPath`, `renderFileEntries`, and `renderFileViewer` to `web/lib/console-presentation.js` and `web/lib/console-presentation.d.ts`, rejecting traversal (`..`) and absolute/drive paths in UI feedback while keeping all authoritative boundary and 512 KiB size checks in Rust.
- Wired `#chat-files-btn` and `#chat-files-panel` in `crates/daemon/src/console.html` and `crates/daemon/src/console-app.js`, reading through `/api/v1/repos/files` and `/api/v1/repos/file` (ADR 0050). Merge review removed the git-diff listing fallback now that the backend is merged, derived the parent-folder link in the UI from the returned path, and mapped HTTP 422 (binary or non-UTF-8) to a clear message.
- Wired `.sp-remote-tools` (`Files`) and `.sp-files-sheet` in `web/components/RemoteCompanion.tsx`, gracefully handling `Unsupported remote command`.
- Added `--sp-*` token CSS rules in `web/app/gnomcode-console.css` for `.sp-files-panel`, `.sp-file-code`, `.sp-file-entry`, 44px mobile targets, and 16px mobile path inputs.

## Verification

- `web/test/file-viewer.test.mjs` (TDD red -> green)
- `node scripts/check-local-console.mjs`
- `cd web && npm ci && npm test && npm run typecheck && npm run build`
- Headless Chromium (`chrome-headless-shell.exe`) visual + layout check at `390x844` and `1440x900` (dark & light).
- Full Rust workspace gate (`CARGO_INCREMENTAL=0`): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
