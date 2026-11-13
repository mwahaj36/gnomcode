# 2026-11-13 - Claude child environment scrub (G11)

## Goals
- Prevent nested-session failures when Gnomcode launches Claude Code (PTY host or background metadata commands) from a parent Claude Code environment.
- Remove only the parent-session context markers verified in shipped Claude Code (`CLAUDECODE`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_CHILD_SESSION`), preserving provider selection and restriction environment variables and leaving the parent process environment untouched.

## Outcomes
- Added `CLAUDE_PARENT_ENV_VARS` in `crates/platform/src/executables.rs` and applied `env_remove` for Claude (`"claude" | "claude-code"`) in:
  - `gnomcode_platform::executables::background_agent_command`
  - `gnomcode_pty::run_pty_host` (Windows ConPTY and Unix `portable_pty::CommandBuilder`)
- Updated `docs/SPEC.md` §3.3.

## Decisions
- Scoped removal strictly to the three parent-session markers rather than wiping all `CLAUDE*` variables, so provider selectors (`CLAUDE_CODE_USE_BEDROCK`, etc.) and org restrictions (`CLAUDE_CODE_CHROME_MCP_ORG_DENIED`, etc.) remain intact.

## Verification
- Red-green verified:
  - `gnomcode-platform`: `executables::tests::background_metadata_commands_remove_only_proven_parent_context` failed before fix (`left: []`) and passed after fix.
  - `gnomcode-pty`: `environment_tests::independent_agent_drops_parent_markers_but_preserves_provider_and_restrictions` failed before fix (`clean=false`) and passed after fix (`clean=true;provider=true;restriction=true`).
- Full Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`) run with `CARGO_INCREMENTAL=0`.
