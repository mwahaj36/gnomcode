# Road to 1.0.0

Authorized by Wahaj on 2026-11-13. Work in order; one focused PR per item; each item updates SPEC, adds a session record and, where marked, an ADR. Tick items here in the same PR that finishes them.

## A. In flight
- [ ] G1 phone approvals (ADR 0032)
- [ ] Provider cards: height, missing-state fallback, logos
- [ ] Web Devices and Connect redesign
- [ ] Project names fix and project activity list
- [ ] Subscription value tiles and activity cleanup

## A2. Added 2026-11-13 (assigned to Gemini via agy, reviewed by Claude)
- [x] Push notifications for Needs you: approval waiting, turn finished, limit low, limit reset
- [x] In-app auto-update from the allowed update manifest
- [x] `gnomcode doctor`: agents, sign-in expiry, limit sources, hooks, disk
- [x] First-run tour refresh for the four places and list-first Workspace

## B. Research gaps (docs/research/2026-11-13-dialpad-cao-comparison.md)
- [x] Batch: G5 phone interrupt, G6 agy native resume, G7 agy limits via `/usage`, G11 env scrub, G12 Unix process-group stop, G13 Codex/Gemini native-install discovery
- [x] G8 Claude usage-limit and API error parsing
- [x] G3 terminal screen model, then G14 swallowed-Enter recovery
- [ ] Protocol-mode approvals for Codex (`codex app-server`), Antigravity (`agy acp`) and Gemini CLI (ACP): Gnomcode-launched sessions driven through the agent's structured protocol so phone and console approvals work for every provider (ADR; verify each protocol live first; never auto-approve)

## C. Phone and Remote features (one ADR each, all providers where the agent exposes the data)
- [ ] Quick actions (canned prompts, never raw argv)
- [ ] Changes view: read-only diff of the session's repo on phone and console
- [ ] File viewer on phone: browse and read project files read-only, within the session workspace only
- [ ] Artifacts alongside changes: files the agent created or edited in the turn, plus images/HTML/markdown outputs, shown next to the diff
- [ ] Phone attachments (upload into the workspace with size/type limits)
- [ ] PWA app lock (WebAuthn/passkey or device PIN; never weakens E2EE)

## D. Audit Phase 2 leftovers
- [x] D3 file watcher and idle back-off for ingest (CPU budget: idle under 1%)
- [x] Antigravity exhausted cross-check against RESOURCE_EXHAUSTED
- [x] Failover demo end to end: Claude exhausted, continue in Codex with handoff

## E. Consolidation (audit Phase 3)
- [ ] Local console built from web/ React; delete console-app.js
- [ ] Split api.rs and store/lib.rs by domain
- [x] Freeze unproven features behind off-by-default toggles (Teams, extra adapters, account sync)

## F. Differentiators (audit Phase 4)
- [x] "Continue here" for imported sessions (backend, ADR 0053; pairs with UI PR #209)
- [x] Handoff quality evals (backend, ADR 0055; pairs with UI PR #211)
- [x] Worktree per session with diff review and merge (backend, ADR 0051; pairs with UI PR #206)

## G. Release 1.0.0
- [ ] Version 1.0.0 everywhere (cut-release skill), CHANGELOG, release notes
- [ ] Packaged-app smoke test (launch, detect agents, limits, tray, Remote pairing)
- [ ] Windows installer built and published; macOS/Linux builds where CI supports them
- [ ] Website download page and version updated (Vercel)
- [ ] Real-device checks: physical phone Remote + approvals, installed PWA; macOS/Linux runs recorded or listed as known gaps
- [ ] Pricing decision (after release)

## Worker A backend delivery details

- [x] G5 backend: state-gated Esc request over E2EE Remote and authenticated local API, with bounded idempotency receipts (ADR 0035). Full gate passes on baseline PR #189. Phone button belongs to Worker B; native-provider cancellation and physical-phone acceptance remain unverified.

## Worker A native-resume delivery details

- [x] G6 agy native resume: exact-ID command/capability with owner-approved explicit --mode plan, verified interactively on Windows with agy 1.5.7. Mode changes remain with the person in the native CLI. Depends on baseline PR #189 while unmerged; packaged/macOS/Linux acceptance remains unverified.

## Worker A native-usage delivery details

- [x] G7: existing opt-in Antigravity limits now prefer the running server and fall back to bounded, no-inference usage metadata from verified agy 1.5.7; ADR 0029 amended. Other CLI versions stay gated; native macOS/Linux and independent network acceptance remain unverified. Depends on baseline #189 while unmerged.
## Section C UI status (Worker B)

- [x] Quick actions UI: console and Remote canned-prompt strip and management dialog, persisted by the daemon (`/api/v1/quick-actions`, Remote `quick_actions`, ADR 0043).
- [x] Changes view UI: read-only console panel via `GET /api/v1/sessions/{id}/changes` and Remote `changes` (ADR 0037).
- [x] File viewer UI: read-only workspace-confined browser and reader on console and Remote (ADR 0050).
- [x] Artifacts alongside changes UI: sandboxed HTML (`sandbox=""` iframe with a `default-src 'none'` CSP), Markdown and safe images via `GET /api/v1/sessions/{id}/artifacts` and Remote `artifacts` (ADR 0039).
- [x] Phone attachments UI: 15 MiB client preview and extension validation, upload through `POST /api/v1/sessions/{id}/attachments` and Remote `upload_attachment` (ADR 0041).
- [x] PWA app lock (ADR 0034): WebAuthn passkey or PBKDF2-SHA-256 PIN lock with 10-attempt cooldown, decoupled from E2EE; recovery erases remembered desktops so pairing is required again.
- [x] G5 interrupt button UI on console and phone, consuming the merged interrupt contract (ADR 0035).
- [ ] E1 React local console: Home view (`web/local-console/HomeView.tsx` + `/console-react.js` route, desktop release sourcemap/devtools check) - PR open, awaiting Workspace/Usage/Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Home view (`web/local-console/HomeView.tsx` + `/console-react.js` route, desktop release sourcemap/devtools check) - PR open, awaiting Workspace/Usage/Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Workspace view (`web/local-console/WorkspaceView.tsx` + `#local-workspace-root`, mobile heading wrap fix, zero-sourcemap bundle rebuild) - PR open, depends on Home PR #198; awaiting Usage and Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Home view (`web/local-console/HomeView.tsx` + `/console-react.js` route, desktop release sourcemap/devtools check) - PR open, awaiting Workspace/Usage/Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Workspace view (`web/local-console/WorkspaceView.tsx` + `#local-workspace-root`, mobile heading wrap fix, zero-sourcemap bundle rebuild) - PR open, depends on Home PR #198; awaiting Usage and Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Usage view (`web/local-console/UsageView.tsx` + `#local-usage-root`, period selector, export CSV/JSON, shared presenters) - PR open, depends on Workspace PR #199; awaiting Settings PR to retire `console-app.js`
- [ ] E1 React local console: Home view (`web/local-console/HomeView.tsx` + `/console-react.js` route, desktop release sourcemap/devtools check) - PR open, awaiting Workspace/Usage/Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Workspace view (`web/local-console/WorkspaceView.tsx` + `#local-workspace-root`, mobile heading wrap fix, zero-sourcemap bundle rebuild) - PR open, depends on Home PR #198; awaiting Usage and Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Usage view (`web/local-console/UsageView.tsx` + `#local-usage-root`, period selector, export CSV/JSON, shared presenters) - PR open, depends on Workspace PR #199; awaiting Settings PR to retire `console-app.js`
- [x] E1 React local console: Settings view (`web/local-console/SettingsView.tsx` + `#local-settings-root`) - completes React coverage across all 4 console places (Home, Workspace, Usage, Settings); `console-app.js` remains guarded as a minimal shell/navigation bridge (`window.__gnomcodeLocalActions`) pending removal of `include_str!("console-app.js")` in `crates/daemon/src/api.rs` (blocked by Worker B file ownership)
- [ ] E1 React local console: Home view (`web/local-console/HomeView.tsx` + `/console-react.js` route, desktop release sourcemap/devtools check) - PR open, awaiting Workspace/Usage/Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Workspace view (`web/local-console/WorkspaceView.tsx` + `#local-workspace-root`, mobile heading wrap fix, zero-sourcemap bundle rebuild) - PR open, depends on Home PR #198; awaiting Usage and Settings PRs to retire `console-app.js`
- [ ] E1 React local console: Usage view (`web/local-console/UsageView.tsx` + `#local-usage-root`, period selector, export CSV/JSON, shared presenters) - PR open, depends on Workspace PR #199; awaiting Settings PR to retire `console-app.js`
- [x] E1 React local console: Settings view (`web/local-console/SettingsView.tsx` + `#local-settings-root`) - completes React coverage across all 4 console places (Home, Workspace, Usage, Settings); `console-app.js` remains guarded as a minimal shell/navigation bridge (`window.__gnomcodeLocalActions`) pending removal of `include_str!("console-app.js")` in `crates/daemon/src/api.rs` (blocked by Worker B file ownership)
- [x] F1 Worktree per session UI (`web/local-console/WorkspaceView.tsx`, `web/components/RemoteCompanion.tsx`, ADR `0036`) - launch isolation toggle, read-only diff review, and inline-confirmed Merge / Discard / Open PR controls wired to the merged daemon HTTP and E2EE routes (ADR 0051)
