# Worker B Handoff Log

- **Worker:** B (UI, web/, apps/desktop/, local console, release prep)
- **Base commit:** `fee3c8e` (`origin/main`)
- **ADR numbers assigned to Worker B:** even numbers starting at `0034`

## Status overview

| Order | Area | Item | Status | PR | Depends on |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | Section C | Quick actions (canned prompts, never raw argv) | PR open | [#188](https://github.com/mwahaj36/spawnt/pull/188) | Optional Worker A backend persistence |
| 2 | Section C | Changes view (read-only diff on console and phone) | PR open | [#190](https://github.com/mwahaj36/spawnt/pull/190) | Optional Worker A Remote `changes` handler |
| 3 | Section C | File viewer on phone and console (read-only, workspace only) | PR open | [#191](https://github.com/mwahaj36/spawnt/pull/191) | Optional Worker A `C file listing and reading` |
| 4 | Section C | Artifacts alongside changes | PR open | [#192](https://github.com/mwahaj36/spawnt/pull/192) | Optional Worker A `C last-turn artifacts` |
| 5 | Section C | Phone attachments UI | in progress | - | Optional Worker A `C attachments` |
| 6 | Section C | PWA app lock (WebAuthn/passkey or device PIN; ADR 0034) | not started | - | - |
| 7 | B / G5 | G5 interrupt button on phone and console chat | not started | - | Worker A G5 PR |
| 8 | Section E1 | Local console built from web/ React (Home) | not started | - | - |
| 9 | Section E1 | Local console built from web/ React (Workspace) | not started | - | - |
| 10 | Section E1 | Local console built from web/ React (Usage) | not started | - | - |
| 11 | Section E1 | Local console built from web/ React (Settings + retire console-app.js) | not started | - | - |
| 12 | Section F | F1 worktree per session UI | not started | - | Worker A / backend |
| 13 | Section F | F2 "Continue here" for imported sessions UI | not started | - | Worker A / backend |
| 14 | Section F | F3 handoff quality eval report page | not started | - | Worker A eval data |
| 15 | Section G | Release 1.0.0 prep (cut-release, CHANGELOG, Windows installer) | not started | - | All Worker A PRs |

## Entries

### 1. Section C — Quick actions (canned prompts, never raw argv)
- **PR:** https://github.com/mwahaj36/spawnt/pull/188 (`feat/quick-actions-ui`, commit `2de5af6`)
- **What changed:**
  - `web/lib/console-presentation.js`, `web/lib/console-presentation.d.ts`: exported `QUICK_ACTIONS_STORAGE_KEY`, `DEFAULT_QUICK_ACTIONS`, `normalizeQuickAction`, `mergeQuickActions`, `loadStoredQuickActions`, `saveStoredQuickActions`, `renderQuickActionsStrip`.
  - `crates/daemon/src/console.html`, `crates/daemon/src/console-app.js`: added `#chat-quick-actions` bar, `#quick-actions-dialog` manager, `sendPromptText` / `sendQuickAction` dispatching via `POST /api/v1/sessions/{id}/prompt` (preserving unsent composer drafts).
  - `web/components/RemoteCompanion.tsx`: added `.sp-quick-actions` toolbar, `submitQuickAction` dispatching via `{ type: 'prompt', session_id, text }`, and `Quick prompts` bottom sheet.
  - `web/app/gnomcode-console.css`: appended `--sp-*` token pill strip and dialog styles, 44px touch targets at `<=760px`, hidden at `max-height: 450px` for short keyboards.
  - `web/test/quick-actions.test.mjs`, `docs/SPEC.md` (append §11), `docs/ROADMAP_1.0.md` (append status), `docs/sessions/2026-11-13-quick-actions-ui.md`, `docs/sessions/README.md`.
- **Evidence:** Gap at `crates/daemon/src/console.html:96-97` and `web/components/RemoteCompanion.tsx:586-591` (composer only, no canned prompts); Remote `prompt` validation at `crates/daemon/src/remote.rs:796-822`. Tests: `web/test/quick-actions.test.mjs` (5/5 pass). Gate: `CARGO_INCREMENTAL=0 cargo fmt --all -- --check` pass, `cargo clippy --workspace --all-targets -- -D warnings` pass, `cargo test --workspace` pass, `node scripts/check-local-console.mjs` pass, `web/` `npm ci` + `npm test` (104/104) + `npm run typecheck` + `npm run build` pass.
- **Verification:** Browser UI verified live against an ephemeral mock daemon in `chrome-headless-shell.exe` at `390x844` and `1440x900` (dark & light) and `390x350` (`scrollWidth === innerWidth`, min pill height 44px on mobile, 32px on desktop, hidden under `450px` height). Not verified live against a real phone/relay (`check-remote-companion.mjs`) or Worker A backend persistence.
- **Deviations:** Stored locally (`gnomcode-quick-actions-v1` in `localStorage`) instead of calling unimplemented `quick_prompts` Remote RPCs (which would trigger `Unsupported remote command`).
- **Open risks/questions:** Waiting on Worker A's `C quick actions` PR + ADR if cross-device quick-prompt storage is needed.
- **Dependencies:** Independent UI PR; optional sync depends on Worker A `C quick actions` PR.

### 2. Section C — Changes view (read-only diff on console and phone)
- **PR:** https://github.com/mwahaj36/spawnt/pull/190 (`feat/changes-view-ui`, commit `5afec50`)
- **What changed:**
  - `web/lib/console-presentation.js`, `web/lib/console-presentation.d.ts`: exported `parseGitStatusLines`, `renderChangesView`.
  - `crates/daemon/src/console.html`, `crates/daemon/src/console-app.js`: added `#chat-changes-btn` and `#chat-changes-panel` calling `GET /api/v1/repos/diff?repo=<path>` (1 MiB / 10 s cap, `lossy` banner, auto-close on session switch).
  - `web/components/RemoteCompanion.tsx`: added `.sp-remote-tools` (`Changes` button) and `.sp-changes-sheet` bottom sheet requesting `{ type: 'changes', session_id }` with calm `Unsupported remote command` fallback.
  - `web/app/gnomcode-console.css`: appended `--sp-*` token styles for `.sp-changes-panel`, `.sp-changes-files`, `.sp-diff-code`, `.sp-changes-sheet`, 44px mobile controls, hidden tools bar on `max-height: 450px`.
  - `web/test/changes-view.test.mjs`, `docs/SPEC.md` (append §11), `docs/ROADMAP_1.0.md` (append status), `docs/sessions/2026-11-13-changes-view-ui.md`, `docs/sessions/README.md`.
- **Evidence:** Gap at `crates/daemon/src/api.rs:1485-1513` vs `console.html:87` and `RemoteCompanion.tsx`. Tests: `web/test/changes-view.test.mjs` (3/3 pass). Gate: `CARGO_INCREMENTAL=0 cargo fmt --all -- --check` pass, `cargo clippy --workspace --all-targets -- -D warnings` pass, `cargo test --workspace` pass, `node scripts/check-local-console.mjs` pass, `web/` `npm ci` + `npm test` (102/102) + `npm run typecheck` + `npm run build` pass.
- **Verification:** Browser UI verified live in `chrome-headless-shell.exe` against mocked `/api/v1/repos/diff` at `390x844` and `1440x900` (dark & light): zero horizontal overflow, clean status pills, unified diff lines, 44px mobile buttons. Not verified live over Remote E2EE relay (`{ type: 'changes' }` unimplemented in daemon).
- **Deviations:** Remote requests `changes` only on demand and handles `Unsupported remote command` inside the sheet.
- **Open risks/questions:** Waiting on Worker A's `C session changes/diff` PR + ADR for Remote E2EE parity.
- **Dependencies:** Local works against existing `GET /api/v1/repos/diff`; Remote depends on Worker A `C session changes/diff` PR.

### 3. Section C — File viewer on phone and console (read-only, workspace only)
- **PR:** https://github.com/mwahaj36/spawnt/pull/191 (`feat/file-viewer-ui`, commit `6906389`)
- **What changed:**
  - `web/lib/console-presentation.js`, `web/lib/console-presentation.d.ts`: exported `normalizeWorkspaceRelPath`, `renderFileEntries`, and `renderFileViewer` (`RepoFileEntry`, `RepoFileListing`, `RepoFileData`).
  - `crates/daemon/src/console.html`, `crates/daemon/src/console-app.js`: added `#chat-files-btn`, `#chat-files-panel`, `#chat-file-path-form`, `#chat-file-path-input`, `#chat-files-list`, `#chat-file-body`, calling `GET /api/v1/repos/file?repo=&path=` and optional `GET /api/v1/repos/files`, with fallback to changed files from `/api/v1/repos/diff` + manual relative path lookup.
  - `web/components/RemoteCompanion.tsx`: added `Files` button in `.sp-remote-tools` and `.sp-files-sheet` using E2EE `{ type: 'repo_files', session_id, path }` and `{ type: 'repo_file', session_id, path }` with calm `Unsupported remote command` handling.
  - `web/app/gnomcode-console.css`: appended `.sp-files-panel`, `.sp-file-code`, `.sp-file-entry`, `.sp-file-form` using `var(--sp-*)` tokens, 44px mobile touch targets, and 16px mobile path inputs.
  - `web/test/file-viewer.test.mjs`, `docs/SPEC.md` (append §11), `docs/ROADMAP_1.0.md` (append status), `docs/sessions/2026-11-13-file-viewer-ui.md`, `docs/sessions/README.md`.
- **Evidence:** Gap at `crates/daemon/src/api.rs:1419-1482` vs `console.html`, `console-app.js`, `RemoteCompanion.tsx`, and `remote.rs:625-920`. Tests: `web/test/file-viewer.test.mjs` (3/3 pass), `web/` `npm test` (102/102 pass). Gate: `CARGO_INCREMENTAL=0 cargo fmt --all -- --check` pass, `cargo clippy --workspace --all-targets -- -D warnings` pass, `cargo test --workspace` pass, `node scripts/check-local-console.mjs` pass, `web/` `npm run typecheck` + `npm run build` pass, encoding/LF/mojibake check pass.
- **Verification:** Visually verified live in `chrome-headless-shell.exe` against mocked daemon endpoints at `390x844` and `1440x900` (dark & light): `overflowX=false`, `filesVisible=true`, `entriesCount=3`, `fileLines=5`. Not verified live over Remote E2EE relay (`repo_files` / `repo_file` unimplemented in daemon).
- **Deviations:** Requests `repo_files` / `repo_file` on user open only and intercepts `Unsupported remote command` in-sheet; no Rust edits.
- **Open risks/questions:** Waiting on Worker A's `C file listing and reading` PR + ADR for daemon-side directory listing and Remote E2EE parity.
- **Dependencies:** Local reading works against existing `GET /api/v1/repos/file`; directory listing and Remote E2EE depend on Worker A `C file listing and reading` PR.

### 4. Section C — Artifacts alongside changes (HTML sandboxed, Markdown, images)
- **PR:** https://github.com/mwahaj36/spawnt/pull/192 (`feat/artifacts-view-ui`, commit `4b371d2`)
- **What changed:**
  - `web/lib/console-presentation.js`, `web/lib/console-presentation.d.ts`: exported `classifyArtifactKind`, `isSafeArtifactImageSrc`, `renderArtifactsList`, and `renderArtifactPreview` (HTML inside strict `<iframe sandbox="" referrerpolicy="no-referrer" srcdoc="...">`, Markdown via sanitized `MarkdownIt`, images validated via data-URI/local-API allowlist).
  - `crates/daemon/src/console.html`, `crates/daemon/src/console-app.js`: added `#chat-artifacts-btn` and `#chat-artifacts-panel` reading `GET /api/v1/sessions/{id}/artifacts` + workspace change count from `GET /api/v1/repos/diff` (with fallback to changed markdown/HTML files via `/api/v1/repos/file`).
  - `web/components/RemoteCompanion.tsx`: added `Artifacts` button in `.sp-remote-tools` and `.sp-artifacts-sheet` requesting `{ type: 'artifacts', session_id }` on demand with calm `Unsupported remote command` handling and React `<iframe sandbox="" referrerPolicy="no-referrer" srcDoc={...} />`.
  - `web/app/gnomcode-console.css`: appended `var(--sp-*)` token styles, responsive grid/stack at 390px and 1440px, 44px mobile targets, hidden secondary tools on `max-height: 450px`.
  - `web/test/artifacts-view.test.mjs`, `docs/SPEC.md` (append §11), `docs/ROADMAP_1.0.md` (append status), `docs/sessions/2026-11-13-artifacts-view-ui.md`, `docs/sessions/README.md`.
- **Evidence:** Gap in `crates/daemon/src/console.html:87`, `remote.rs:625-920`, and `RemoteCompanion.tsx:586` (no artifact inspection panel/sheet). Tests: `web/test/artifacts-view.test.mjs` (3/3 pass), `web/` `npm test` (102/102 pass). Gate: `CARGO_INCREMENTAL=0` `fmt`/`clippy`/`test` pass, `check-local-console.mjs` pass, `web/` `npm ci` + `npm test` + `npm run typecheck` + `npm run build` pass, encoding/LF/mojibake check pass.
- **Verification:** Visually verified live in `chrome-headless-shell.exe` against mocked endpoints at `390x844` and `1440x900` (dark & light): `overflowX=false`, `artifactsVisible=true`, `itemCount=3`, `hasSandboxedIframe=true`, `changesText="2 workspace changes"`. Not verified live over Remote E2EE relay (`artifacts` unimplemented in daemon).
- **Deviations:** Requests `artifacts` on demand only and intercepts `Unsupported remote command` in-sheet; no Rust edits.
- **Open risks/questions:** Waiting on Worker A `C last-turn artifacts` PR + ADR for daemon-side artifact tracking and Remote E2EE parity.
- **Dependencies:** Full turn-scoped artifact tracking and Remote E2EE depend on Worker A `C last-turn artifacts` PR.

### 5. Section C — Phone and console attachments UI
- **PR:** https://github.com/mwahaj36/spawnt/pull/194 (`feat/phone-attachments-ui`, commit `3b7d09d`)
- **What changed:**
  - `web/lib/console-presentation.js`, `web/lib/console-presentation.d.ts`: exported `MAX_ATTACHMENT_BYTES`, `validateAttachmentCandidate`, `renderAttachmentPills`, `formatPromptWithAttachments`.
  - `web/components/RemoteCompanion.tsx`: added hidden file input + `.sp-attach-trigger`, staged `.sp-attachments-bar` above composer textarea, E2EE `upload_attachment` / `attachment_ack`, prompt ref attachment, and calm `Unsupported remote command` handling.
  - `crates/daemon/src/console.html`, `crates/daemon/src/console-app.js`: added `#chat-attach-input`, `#chat-attach-btn`, `#chat-attachments-bar`, `#chat-attach-status` calling `POST /api/v1/sessions/{id}/attachments`.
  - `web/app/gnomcode-console.css`: appended `var(--sp-*)` token styles with 44px mobile touch targets.
  - `web/test/attachments-ui.test.mjs`, `docs/SPEC.md` (append §11), `docs/ROADMAP_1.0.md` (append status), `docs/sessions/2026-11-13-phone-attachments-ui.md`, `docs/sessions/README.md`.
- **Evidence:** Gap in `remote.rs:625-920`, `console.html:96-99`, and `RemoteCompanion.tsx:586-591`. Tests: `web/test/attachments-ui.test.mjs` (3/3 pass), `web/` `npm test` (102/102 pass). Gate: `CARGO_INCREMENTAL=0` `fmt`/`clippy`/`test` pass, `check-local-console.mjs` pass, `web/` `npm ci` + `npm test` + `npm run typecheck` + `npm run build` pass, encoding/LF/mojibake pass.
- **Verification:** Visually verified live in `chrome-headless-shell.exe` against mocked endpoints at `390x844` and `1440x900` (dark & light): `overflowX=false`, `attachBtnVisible=true`, `attachBtnMinHeight=44` (mobile) / `40` (desktop), `pillCount=2`. Not verified live over Remote E2EE relay (`upload_attachment` unimplemented in daemon).
- **Deviations:** Requests `upload_attachment` on user file pick only and handles `Unsupported remote command` in-composer; no Rust edits.
- **Open risks/questions:** Waiting on Worker A `C attachments` PR + ADR for authoritative backend validation and E2EE dispatch.
- **Dependencies:** Backend persistence and Remote E2EE dispatch depend on Worker A `C attachments` PR.
