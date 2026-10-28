# 0018 — Router, zero-token handoff, and policy controls

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

When an AI coding agent exhausts its quota, approaches context saturation (>75% window fill), or encounters provider-specific rate limits, users are frequently forced to abandon their workflow or manually copy prompts and modified files to another tool.

A naive re-execution of conversation history on a new model wastes substantial token budget (re-paying for hundreds of thousands of input tokens) and can fail if context limits are exceeded.

## Direction

1. **Zero-Token Handoff (`crates/core`)**:
   - Rather than replaying hundreds of raw log lines or thousands of token turns, Spawnt compiles a structured `HandoffPacket`.
   - The packet includes:
     - The target model and provider.
     - Redacted task objectives from earlier user prompts.
     - A structured list of files modified during the session.
     - A concise summary of recent tool execution states.
   - This delivers a seamless model switch with minimal input overhead.

2. **Policy Controls (`crates/core`, `crates/store`)**:
   - Users control failover behavior through three explicit policies:
     - `AskFirst`: User must approve any suggested model switch or auto-resume.
     - `Automatic`: Transparently failover to the highest-quota fallback model within policy rules.
     - `DenyCrossProvider`: Prevent automatic switching across different providers (e.g. Anthropic to OpenAI or Google) to honor enterprise compliance boundaries.

3. **Daemon Endpoints & CLI (`crates/daemon`, `crates/cli`)**:
   - `POST /api/v1/sessions/{id}/switch`: executes a zero-token handoff to the requested or optimal model.
   - `POST /api/v1/sessions/{id}/resume`: arms and resumes a paused or rate-limited session when quota resets.
   - `GET /api/v1/routing/policy` and `POST /api/v1/routing/policy`: manage routing preferences.
   - CLI commands: `spawnt switch <session-id> --model <model>`, `spawnt resume <session-id>`, and `spawnt policy`.

## Consequences

Users can maintain uninterrupted coding velocity without exceeding subscription limits or leaking data across provider boundaries. All routing operations run locally on loopback.
