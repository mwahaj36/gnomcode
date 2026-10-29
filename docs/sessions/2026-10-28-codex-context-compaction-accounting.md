## 2026-10-28 — Codex context compaction accounting

- Provider-wide follow-up: audited all seven adapters; the other adapters already use request snapshots. Added shared-store regression coverage for Claude, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor, and Aider proving a newer 20k context replaces a previous 190k snapshot. Context remains last-observed until the provider emits new usage; compaction content is never stored.

- Goal: keep Codex's displayed context-window usage aligned with the active context after compaction.
- Outcome: Codex usage records now take `context_tokens` from `last_token_usage.total_tokens`, while cumulative `total_token_usage` remains reserved for session-wide accounting. Existing persisted Codex rows are corrected at query time from their latest turn input/output counts, without a database rewrite. Added an anonymized synthetic rollout fixture demonstrating context dropping from 190k to 20k after `context_compacted` even while cumulative tokens increase.
- Verification: adapter and store regression tests each reproduced the cumulative-token bug before their fixes and passed after. Full Rust quality gate passed with the store compatibility correction; `cargo-deny` was skipped because it is not installed.
