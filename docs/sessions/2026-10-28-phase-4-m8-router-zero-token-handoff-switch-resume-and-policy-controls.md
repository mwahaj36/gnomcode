## 2026-10-28 — Phase 4: M8 router, zero-token handoff, switch/resume, and policy controls

- Began ADR 0018: model router, handoff compilation, resume queue, and policy controls. Live switching and routing still required follow-up.
- Domain types (`crates/core`): Added `RoutingPolicy` (`ask_first`, `automatic`, `deny_cross_provider`), `HandoffPacket` compiling prompt summaries, touched files, and turn counts for model transitions without raw log token replay. Added unit tests for serialization and parsing.
- Storage layer (`crates/store`): Added `get_routing_policy` and `save_routing_policy` persisting to SQLite `context_relay_config` with unit tests.
- Daemon API (`crates/daemon`):
  - `POST /api/v1/sessions/{id}/switch`: initially recorded a `System` chat event without switching. Follow-up work replaced this with a target-session launch and prompt delivery.
  - `POST /api/v1/sessions/{id}/resume`: arms session in `ResumeQueue` for automatic execution on quota reset.
  - `GET /api/v1/routing/policy` & `POST /api/v1/routing/policy`: inspects and updates the active failover policy.
  - Unit tests in `crates/daemon/src/api.rs`.
- CLI commands (`crates/cli`):
  - `spawnt switch <session-id> --model <model>`: initially only compiled a packet. Follow-up work added target-session launch and prompt delivery.
  - `spawnt resume <session-id> [--prompt <prompt>]`: arms session in Resume Queue.
  - `spawnt policy [--set <policy>]`: reads or updates active routing policy.
  - Integration tests in `crates/cli/tests/launch_lifecycle.rs` for `policy`, `switch`, and `resume`.
- Full quality gate verified across all crates and frontend packages.

