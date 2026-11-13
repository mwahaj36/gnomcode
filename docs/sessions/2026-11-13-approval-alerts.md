# Held approval alerts - 2026-11-13

Goal: Successful phone/console holds now queue the existing structured ApprovalWaiting notification asynchronously for Claude Code, Codex and Antigravity. Hook hints and periodic observations share the same notification engine, dedupe and rate limits. Push payloads never include approval tool names or summaries; notification preferences remain authoritative. Failed or unavailable delivery cannot delay the hold response or block agent use.

Outcome and verification: Regression first failed because held approvals queued no structured payload. Router test now covers all three providers, repeated hints, redacted payload shape and disabled preferences. Full workspace fmt/clippy/tests passed. Cross-platform review: existing notification/queue transport reused; no new OS calls, paths, permissions or network destinations. Native desktop/physical phone delivery not exercised.

Decision: reuse the existing Rust contract; no new architecture or dependency.
