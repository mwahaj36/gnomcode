---
name: rust-quality-gate
description: >-
  Run the full Gnomcode Rust quality gate (rustfmt, clippy with -D warnings,
  unit/golden tests, cargo-deny, and architectural dependency-direction checks)
  before committing or opening a PR. Activate whenever finishing a Rust code
  change or validating workspace health.
---

# Rust Quality Gate

Use this skill before committing any Rust change to ensure the workspace passes every CI gate and obeys `AGENTS.md`.

## Steps

1. **Run the automated gate script:**
   - On Windows (PowerShell): [`scripts/check.ps1`](./scripts/check.ps1)
   - On macOS / Linux (POSIX sh): [`scripts/check.sh`](./scripts/check.sh)

2. **Verify architectural dependency rules (`AGENTS.md` §3):**
   - `gnomcode-core` has **zero** internal crate dependencies and **zero** async/I/O crates (`tokio`, `rusqlite`, `notify`, `axum`).
   - `unsafe` blocks exist **only** in `crates/platform` and every block has a preceding `// SAFETY:` comment.
   - No `.unwrap()` or `.expect()` in non-test code.

3. **Verify privacy invariants (`AGENTS.md` §2):**
   - Confirm no prompt text, code, file contents, or raw tool outputs are stored in SQLite structs or emitted in `tracing` macros.
   - Confirm any activity summary string flows through `gnomcode_core::redact` before persistence.

4. **Verify documentation sync:**
   - If scope, data schema, CLI flags, roadmap status, or deliverables changed, update [`docs/SPEC.md`](../../../docs/SPEC.md) in the same commit.
