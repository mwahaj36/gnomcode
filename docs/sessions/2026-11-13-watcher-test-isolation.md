# 2026-11-13 - Watcher test isolation

## Goals
- Stop `watcher::tests::ingest_watcher_wakes_on_file_write_and_skips_idle_rescan` from running `ingest_all` against the real home directory (AGENTS.md section 6).

## Outcome
- The test re-executes its own test binary as a child with `HOME`, `USERPROFILE`, `GNOMCODE_HOME`, `SPAWNT_HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME` and `GEMINI_HOME` pointed at a temp dir. The child runs the original body.
- The parent asserts the child exited successfully and reported `1 passed`, so a mistyped test name cannot pass silently.

## Problems and decisions
- `ingest_all` calls `discover_agent_roots` and `gnomcode_home`, which read the environment. In-process `set_var` is `unsafe` in edition 2024 and `unsafe` is denied outside `crates/platform`, so a child process is the only safe override.

## Verification
- `cargo test -p gnomcode-ingest watcher` passes; full gate (fmt, clippy -D warnings, test --workspace: 499 passed) passes.
