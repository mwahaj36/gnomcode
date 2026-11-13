# 2026-11-14 Release gaps

## Goals

Close three release-readiness gaps found after the version, signing and smoke work: stale 1.0.0 notes, a release script that could build without the update key, and no CI coverage of the embedded-key regression.

## Outcomes

- `docs/release-notes/1.0.0.md` coverage now runs through `9a20fe9` and lists user-facing changes from #252 to #262 (surface labels, held-approval alerts, Remote activity timer, handoff goal, hosted open-tool presence, packaging and platform notes). Scope stays Claude Code, Codex and Antigravity.
- `package-release.ps1` throws before `cargo build` unless `GNOMCODE_UPDATE_PUBLIC_KEY` matches 64 hex characters. Dev builds do not use it. Verified: empty value throws.
- New CI job `embedded-key` builds daemon tests with the RFC 8032 test public key, then runs the ignored `embedded_key_requires_signature_despite_runtime_override` with the variable unset and with a different value.

## Problems

- A first attempt at the guard lost its quotes through shell quoting; the parse error was caught by running the script.
- No jq on the local shell, so the CI step parses cargo JSON with python3, which also runs locally.

## Verification

Local run of the CI step logic: both runs pass. Full gate results are in the PR.
