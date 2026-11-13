# Build-pinned update signing

Date: 2026-11-13. Owner: codex-release.

## Goals and outcomes

Embed an owner-supplied public key at Rust compilation. Require signatures when embedded and keep app use fail-open. Add offline signing and key-handling/rotation instructions. No private key generated, read or committed. ADR 0059 records the change; per-artifact signature format remains compatible with ADR 0033.

## Root cause and verification

Runtime-only configuration allowed an intended signed build to accept unsigned bytes after environment removal. A public-key-only RFC 8032 regression was compiled with the test public key, then run without it at runtime: failed on unsigned acceptance before the fix. It passes after the fix with runtime key absent, blank and replaced, rejecting unsigned, bad-signature and tampered content and accepting the RFC signature. No private key is necessary.

Node signer regression failed before the script existed, then passed: absent/conflicting key inputs do not change the manifest or print a sentinel. Valid owner-key signing remains unverified because no key was supplied and key generation is prohibited.

## Decisions and risks

Malformed embedded keys fail closed for installation. Development builds without a key keep hash-only compatibility. Cached files use the same verifier as fresh downloads. Electron still requires daemon ready state and rechecks staged path/hash before explicit launch. No agent or app is stopped on verification failure. Signature covers bytes, not manifest metadata; approved origins and SemVer checks still apply. Existing unpinned clients do not gain mandatory verification retroactively. Review key custody and rotation before release.

Cross-platform review: no OS branches, unsafe, process, IPC or agent-log changes. Existing update paths use platform gnomcode_home and Path joins. Signer uses portable Node path APIs; macOS/Linux execution not observed.

## Full gate

CARGO_INCREMENTAL=0 with dev/test debug symbols disabled: cargo fmt --all, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace passed (499 tests, one build-key regression intentionally ignored in the default build). The ignored regression was separately run and passed with a compiled RFC public key and three runtime environments. Node signer test passes. git diff --check passes; U+00C2 count 0. Cargo.lock unchanged; no new dependencies. macOS/Linux execution, owner-key signing and installed update acceptance are unverified.
