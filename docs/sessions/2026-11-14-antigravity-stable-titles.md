# 2026-11-14 - Antigravity stable session titles

## Goal
Antigravity sessions (IDE, agy CLI, Antigravity 2.0 app) changed title on every input and showed the last prompt. Claude and Codex titles stay stable.

## Root cause
The Antigravity adapter derives a title from the first `USER_INPUT` it sees in each parsed chunk. The watcher parses only the appended tail, so each new prompt started a fresh adapter with no title and produced the latest prompt as its title. The session upsert used `coalesce(excluded.title, session.title)`, so the new value always won. Claude and Codex never derive a title from prompts, so they were unaffected.

## Changes
- `crates/store/src/ingest.rs`: Antigravity titles are write-once; a stored non-placeholder title is never replaced by ingest.
- `crates/store/src/projects.rs`: Antigravity's own `conversation_summaries.title` now wins over a prompt-derived title.
- `crates/store/src/schema.rs`: migration 16 restores each non-subagent Antigravity title from its first stored user prompt.
- `crates/adapters/src/antigravity.rs`: `prompt_title` skips system and meta text and system-sourced steps.
- Fixtures `multiturn.jsonl` for cli-1.5.8, ide-1.107.0 and app-2.31.2 (synthetic, generic text, so the anonymizer's prompt redaction does not apply) and tests in `crates/ingest/src/antigravity_title_tests.rs`.

## Decisions
- No rename feature exists, so nothing user-set can be lost; ingest still never overwrites an existing title.
- Migration relies on stored chat events; if retention removed the first prompt, the earliest remaining one is used.

## Verification
fmt, clippy, workspace tests and cargo deny; new tests fail without the store change.
