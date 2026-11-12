# 2026-11-12 — Live audit and editor adapter removal

## Goals
- Audit the installed app against real Claude Code, Codex and Antigravity data ([audit](../APP_AUDIT_2026-11-12.md)).
- Narrow scope: remove Cline/Roo, Cursor and Aider adapters; keep Teams as a later product.

## Outcomes
- Removed the three adapters, their discovery roots, fixtures, tests, editor-focus helper, console filter options, web copy and icons.
- Added `AgentId::parse` returning `None` for unknown identifiers. Store lists, reports and the resume queue now skip unknown agents instead of relabeling them as Claude Code (latent bug for any unknown agent string).
- Removed store-open maintenance for legacy Cursor placeholders.
- SPEC §2, §3.5, §3.7, §3.14, PRODUCT, README, AGENTS, launch checklist and adapter skill updated.

## Decisions
- Existing rows for removed agents are retained, never deleted (protect history); they are skipped by typed queries.

## Verification
- `cargo build/test --workspace`, `cargo clippy --workspace --all-targets -D warnings`, `web` `npm test` (30 pass).
