# G5 Remote interrupt backend

## Goal and evidence

Add backend-only turn interruption, preserving the hosted process and native permission prompts. At base `fee3c8e`, `crates/daemon/src/remote.rs:626-866` rejects interrupt as unsupported. `crates/pty/src/lib.rs:797-817` implements process stop, while `:1056-1078` already provides authenticated input acknowledged after the PTY write.

## Outcome and decisions

New `crates/daemon/src/interrupt.rs` implements request validation, stable conversation resolution, Running-state and pending-approval gating, serialization with other conversation controls, and bounded in-memory duplicate protection. Existing daemon files gain only module, route and dispatch/activity lines under the explicit owner authorization. No UI, new dependency, configuration mutation or process termination was added. SPEC and ADR 0035 document the JSON protocol, receipt lifetime and acknowledgment limits.

Remote request: `{"type":"interrupt","session_id":"<conversation>","request_id":"<8-64 ASCII letters/digits/-/_>"}`. Local `POST /api/v1/sessions/{id}/interrupt` body: `{"request_id":"..."}`. Success/no-op: `{"type":"interrupt_ack","session_id":"...","request_id":"...","sent":true|false}`. Errors retain the existing error envelope; clients must never automatically replay an old request after reconnect, daemon restart or receipt expiry.

## Problems and root causes

The initial ownership split prohibited existing Remote dispatcher changes. The owner explicitly authorized append-only dispatch/module wiring and assigned both shared daemon files exclusively to Worker A. The original handoff PR #185 records the resolution.

The first two regression tests failed with `Unsupported remote command` before implementation. After adding the minimal dispatch and handler, both passed. Additional coverage verifies local authentication/Host/Origin, bounded request IDs and retention of an ambiguous write failure so repeated delivery cannot affect a later composer.

## Verification

- Red: `cargo test -p gnomcode-daemon interrupt::tests` failed on unsupported dispatch before implementation (exact test result).
- Green: the same command passed all four focused tests (exact test result).
- `CARGO_INCREMENTAL=0 cargo fmt --all --check`: pass.
- `CARGO_INCREMENTAL=0 cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `CARGO_INCREMENTAL=0 cargo test --workspace`: fail at `api::tests::native_link_requires_account_wide_consent_before_auth_or_persistence` (daemon: 119 passed, one failed; exact test result). The endpoint returns 401 instead of expected 428. The identical test also fails on the reporting branch with unchanged main implementation (zero passed, one failed; exact test result). No test was disabled. The gate stopped there; later workspace suites were not reached.
- `cargo deny check`: fail, `RUSTSEC-2026-0344` for existing `ruzstd 0.8.3`; Cargo.lock unchanged. The tool reports the fix in 0.9.1 or later. Bans, licenses and sources pass.
- Final focused rerun: four passed, zero failed (exact test result). The baseline reproduction shared the build directory and left an executable without G5 tests; an initial filtered rerun selected zero tests. Invalidated the daemon package artifacts, rebuilt from this feature worktree and confirmed the four named tests actually executed.
- Core dependency purity, staged whitespace, UTF-8 without BOM and LF checks: pass; staged diff has zero U+00C2 mojibake matches (exact check result). All existing shared files have additions only.
- Cross-platform review: reuses the existing portable authenticated input path; no signals, filesystem layout, platform conditional code or process termination added. New host-discovery integration test uses a subprocess with a temporary GNOMCODE_HOME and temporary workspace; no real agent history/config is used.
- Verified live: local loopback integration with a controlled input endpoint and real database/binding/request code, in automated tests only.
- Not verified: native cancellation in Claude/Codex/Antigravity/Gemini, actual E2EE relay/phone operation and macOS/Linux execution. The API acknowledges input delivery only.

## Dependencies and questions

No Worker B code dependency. Worker B can implement the phone button against the documented JSON. Is Esc accepted as cancellation by each currently supported CLI version? This remains a live acceptance question; no universal cancellation claim is made.

## Status at stop

G5 implementation is reviewable but remains a draft because the full gate is red. Root cause of the consent-test failure: `remote_device_action_handler` reads cloud auth at `crates/daemon/src/api.rs:1772-1775` before checking `remote_access_confirmed` at `:1779-1780` (line numbers in this branch). An isolated unsigned-in home therefore returns 401 before reaching the required 428 consent rejection. Existing handler edits are outside the append-only route/module allowance. Updating vulnerable ruzstd also needs the unowned root Cargo.toml/Cargo.lock. Owner must assign those fixes or explicitly extend file ownership; no lint, test or advisory suppression is proposed.

Resume by fixing/merging the baseline consent and dependency blockers, rebasing this feature branch, rerunning the complete gate and native acceptance, then marking G5 ready before continuing to G6. No later item was started beyond read-only `agy --version`/`--help` inspection (installed version 1.5.7 advertises --conversation; interactive resume remains unverified).
