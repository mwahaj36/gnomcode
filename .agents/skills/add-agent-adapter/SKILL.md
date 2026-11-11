---
name: add-agent-adapter
description: >-
  Step-by-step runbook for adding or updating an AI agent log adapter in
  crates/adapters (e.g., Claude Code, Codex, Antigravity, Gemini CLI), including log anonymization, golden fixture testing,
  deduplication keys, and per-model quota pool mapping.
---

# Add or Update an Agent Adapter

Follow this procedure whenever adding support for a new AI coding agent or updating an existing parser in `crates/adapters`.

## Steps

1. **Collect and anonymize a real log sample:**
   - Locate a real session log on disk (never copy it directly into the repo).
   - Run the anonymizer script to strip prompts, code, file paths, usernames, and secrets while preserving JSON structure, token counts, model names, and rate-limit objects:
     ```sh
     node .agents/skills/add-agent-adapter/scripts/anonymize-fixture.mjs <input-file> fixtures/<agent>/<version-or-date>/sample.jsonl
     ```
   - Inspect the output manually to confirm zero personal paths, prompts, or secrets remain.

2. **Document format details:**
   - Record path roots (Windows/macOS/Linux + env var overrides), file kind (`AppendOnlyJsonl | RewrittenJson | Sqlite`), deduplication key, model extraction, and quota `limit_id` fields in [`references/agent-log-formats.md`](./references/agent-log-formats.md) and [`docs/SPEC.md`](../../../docs/SPEC.md) §2 & §3.5.

3. **Implement `SourceAdapter` in `crates/adapters/src/<agent>.rs`:**
   - Keep `parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> Vec<RawEvent>` **100% pure and infallible** (no I/O, no `unwrap()`, no panics on malformed/truncated JSON or unknown fields).
   - Emit a deterministic `dedup_key` on every `UsageRecord` so resumed sessions and rotated files never double-count.
   - Extract the per-model quota pool identifier (`limit_id`) alongside any rate-limit window data (SPEC §3.9).
   - Accept both `\n` and `\r\n` line endings.

4. **Write golden + property tests:**
   - Parse every file in `fixtures/<agent>/` and assert against a snapshot (`insta`).
   - Add tests for: (a) truncated trailing line, (b) corrupted middle line skipped cleanly, (c) duplicate lines producing identical `dedup_key`s, and (d) CRLF vs LF parity.
