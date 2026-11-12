# Restore green baseline

## Goal

Fix the consent-ordering regression and RUSTSEC-2026-0344 together in a separate baseline PR based on origin/main, before rebasing G5. Owner explicitly permits existing consent-handler and root Cargo.toml/Cargo.lock/deny.toml edits for this PR only. No check may be disabled.

## Root causes and decisions

At main `fee3c8ee8529213c60d541e7e160594979d6ad44`, `crates/daemon/src/api.rs:1768-1776` loads cloud authentication before checking `remote_access_confirmed` in the enable-account-access arm. The isolated consent regression test returns 401 instead of 428. SPEC section 3.15 and ADR 0025 require explicit native consent before cloud authentication or persistence, while normal local bearer/Host/Origin authentication still applies first.

History identifies the regression in `1f96ab227b57a1e60ac3b57808c9dbd132eb0340` (PR #132): the old early consent guard was removed and reintroduced inside a match below cloud-auth loading. The original test was added by `238a25bd` alongside the consent contract. Fix the handler, preserving the test and contract. Do not weaken expected status to 401.

Root Cargo.toml selects ruzstd 0.8 and Cargo.lock resolves 0.8.3. `crates/ingest/src/lib.rs:134-141` decodes externally produced Codex archives before applying the output reader cap; an internal decompression allocation bomb is therefore relevant. `cargo info ruzstd@0.9.1` verifies that a patched release exists and its declared Rust 1.87 minimum matches this workspace. Upgrade the dependency floor and lockfile to 0.9.1; no deny.toml ignore is appropriate.

## Changes and scope

Restore the early consent guard after local request authentication and before cloud client/auth setup. Keep other device operations unchanged. Upgrade only ruzstd and any required resolution changes. Append SPEC clarification and this session record. No UI, protocol extension, architecture decision or new setting is introduced.

## Verification

- Fresh red: unchanged `native_link_requires_account_wide_consent_before_auth_or_persistence` failed with 401 versus expected 428 before the handler fix.
- Green: the same unchanged test passed after restoring the early guard.
- Full `.agents/skills/rust-quality-gate/scripts/check.ps1`, with CARGO_INCREMENTAL=0 and isolated GNOMCODE_HOME/CLAUDE_CONFIG_DIR/CODEX_HOME: PASS. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` and core dependency purity all passed.
- Existing `archived_codex_rollouts_are_discovered_and_decompressed` passed with ruzstd 0.9.1, covering archive discovery, round-trip decoding and malformed input.
- Cargo.lock changes only ruzstd's version and checksum. No advisory ignore, disabled lint/test, dependency API adjustment or deny.toml change.
- Privacy/cross-platform review: no new I/O, network destination, persistence, process or OS-specific behavior; local request authentication still precedes the consent check, while the check now precedes cloud auth. Only existing portable Rust paths changed.
- Web/desktop gates not run: neither UI nor presenters changed. Native provider/physical-phone/macOS/Linux acceptance not verified by this Rust gate.

## Coordination and remaining evidence

G5 PR #187 will depend on this baseline PR and be rebased onto its branch without merging. Worker B should base new PRs on this branch while the baseline is unmerged, then use freshly fetched origin/main after it merges. All PRs remain unmerged by Worker A. Native/provider/phone/platform acceptance is not established by these baseline fixes.
