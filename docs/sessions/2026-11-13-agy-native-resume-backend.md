# G6 Antigravity native resume backend

## Owner mode-policy resolution

The owner selected explicit plan-mode resume after the finding below: always build `agy --conversation <native-id> --mode plan` for both antigravity and agy aliases. Any later mode change stays with the person in the native CLI. This is intentionally restrictive when the previous mode is unknown; no mode is inferred, broader permissions are never restored automatically and no fallback to Auto is allowed. The actual four-argument command was already verified interactively. G6's draft hold is resolved subject to the updated failing test and fresh Rust gate below.

Final verification: renamed the command regression to `resume_uses_the_exact_native_session_without_permission_bypass`, changed its literal expected argv first and observed failure because --mode plan was absent. Added only the restrictive mode arguments for the two Antigravity aliases; the focused test then passed. Fresh full Rust quality gate PASS with CARGO_INCREMENTAL=0 and isolated test homes: fmt, workspace clippy -D warnings, workspace tests, cargo-deny and core purity. The earlier web ci/test/typecheck/build pass covers the unchanged capability code; the final policy change only alters native argv. No new web change or regression concern justified repeating that build. G6 is now reviewable, with its previous draft hold resolved and roadmap backend completion checked.

## Goal and evidence

Support exact native resume for Gnomcode-launched Antigravity CLI conversations after verifying the actual interactive flag. At baseline `0d439e6`, `crates/pty/src/lib.rs:120-122` rejects Antigravity in resume_command, `crates/daemon/src/lib.rs:337-342` rejects its saved native target and `crates/daemon/src/remote.rs:1184-1195` withholds can_resume for an observed saved Antigravity session.

## Live verification before implementation

Installed `agy --version` reports 1.5.7. Its actual help advertises --conversation, --new-project, --mode and --log-file. In a disposable workspace, launched the native CLI with --new-project and explicit native plan mode. Sent one harmless self-authored marker request with no tools requested; the CLI returned the marker. On exit it printed its native resume command. Reopened that exact ID using `agy --conversation <native-id> --mode plan`; the interactive terminal restored the saved user turn and assistant marker before any new prompt was sent. Then exited cleanly. No credentials were entered, permission bypass flags passed, agent configuration edited or native history removed.

This verifies separated --conversation and ID arguments in interactive mode on Windows for 1.5.7, not merely a help entry. Native logs remain outside the repository in the disposable workspace; no account email, native session ID or raw personal log is committed. A first idle exploratory launch displayed the native default Auto mode; it was exited without a prompt, and the marker/resume check used the explicit restrictive plan flag. Fresh product launches stay unchanged; native Antigravity resumes use the explicit restrictive mode chosen by the owner.

## Scope and decisions

Extend the existing exact-ID command builder for antigravity and agy aliases with explicit restrictive --mode plan. Reuse its strict ID checks, workspace validation and shell-free launch path without adding a prompt/model/bypass argument. Extend only the existing daemon native-target allowlist and Remote saved-session resume capability. Imported sessions remain read-only and no model-switch capability is added.

The owner explicitly authorized the minimal existing lib.rs allowlist and remote.rs capability edits after their append-only restriction was identified, plus minimal vendor_limits.rs wiring for the next G7 item. The latter allowance is not used by this G6 PR. New daemon regression coverage lives in its own new test module.

## Verification

- Fresh red: `resume_uses_the_exact_native_session_without_permission_flags` rejected Antigravity with UnsupportedAgent, and `saved_antigravity_native_conversation_advertises_resume` returned false instead of true before implementation.
- Green: both executed focused tests pass. Capability coverage also confirms imported Antigravity history remains read-only. Tests use synthetic IDs and temporary workspaces/in-memory databases, never the separate live native history.
- Full Rust quality gate PASS with CARGO_INCREMENTAL=0 and isolated GNOMCODE_HOME/CLAUDE_CONFIG_DIR/CODEX_HOME: formatting, workspace clippy with -D warnings, workspace tests, cargo-deny and core purity.
- Web compatibility gate PASS because can_resume is presented to the UI: npm ci, npm test, npm run typecheck, npm run build. NEXT_TELEMETRY_DISABLED=1; no tracked web file changed. Desktop not touched, so desktop commands not run.
- Initial gate success did not resolve the native mode-policy finding below. The owner subsequently chose explicit plan-mode resume; fresh Rust verification of that final policy is required before marking ready.

## Dependencies and limits

Depends on baseline PR #189, branch fix/restore-green-baseline, for the green baseline. Independent of G5 #187 and Worker B UI changes. No new endpoint, JSON shape, setting, dependency or ADR; this extends the existing native-resume contract. Packaged Gnomcode end-to-end resume and macOS/Linux native runs remain unverified until separately recorded.

## Additional native mode finding before publication

After the first full Rust and web gates passed, repeated the exact minimal `agy --conversation <native-id>` command without the plan override. History was restored, but the previously Plan conversation reopened in native Auto mode. No follow-up prompt was sent; exited cleanly. Therefore bare native resume does not establish preservation of the earlier permission mode. Paused G6 publication and asked the owner to choose explicit restrictive plan-mode resume or keep G6 gated until prior mode can be restored reliably. Do not treat the initial green gate as acceptance of this newly observed native behavior.

## Initial draft hold (owner subsequently resolved mode policy)

Preserve this implementation as a draft only; do not merge or enable it until the owner's mode-policy choice is applied and tested. The pending question offers verified `--mode plan` on resume, with any later mode change performed by the person in the native CLI, or keeping G6 gated pending reliable restoration. No answer is assumed from the preselected option. Native sessions have exited; no history was deleted. Resume on feat/agy-native-resume, based on baseline PR #189, by updating the expected command test first for the chosen policy, observing the failure, implementing the smallest policy change and rerunning the complete gate. Later items have not started beyond source inspection; G7 minimal fallback wiring is already authorized.
