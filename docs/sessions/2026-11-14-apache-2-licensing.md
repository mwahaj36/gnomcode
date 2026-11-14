# Apache-2.0 open source licensing (ADR 0062)

Date: 2026-11-14.

## Goals

Make Gnomcode 1.0 free and open source under Apache-2.0 and remove closed-source protections and paid gates. The repository stays private for now.

## Outcomes

- Root `LICENSE` (Apache-2.0 text) and `NOTICE`; `CONTRIBUTING.md` (DCO), `TRADEMARKS.md`, `CODE_OF_CONDUCT.md`; SECURITY.md no longer lists a non-existent mailbox.
- ADR 0062 supersedes ADR 0002. AGENTS.md invariant 8 and SPEC 6.7/6.8 rewritten; business logic in Rust stays as an architecture rule.
- EULA files removed. Installer, desktop staging, npm packages, archives and release checks now ship and byte-compare the root `LICENSE`. npm `license` is `Apache-2.0`.
- `obfstr!` and the crate dependency removed from all crates (mechanical unwrap of 129 call sites, then `cargo fmt`).
- Remote license gate and the hard-coded owner email check removed from `crates/daemon/src/pro.rs` and `account_devices.rs`; the unused entitlement test was deleted.
- Web copy (pricing, terms, FAQ, landing, Remote docs) now says free and open source.

## Decisions

- Apache-2.0 chosen by the owner. Remove obfstr now (owner choice).
- License activation endpoints and `core::license` remain; removal is a follow-up (touches the console Settings UI, which other agents are editing).
- Winget `LicenseUrl` points at the GitHub repo and will need the final org and repo name.

## Verification

See the pull request for the gate output.
