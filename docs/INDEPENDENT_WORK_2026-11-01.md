# Independent launch work — 2026-11-01

This records a source review, automated checks and limited measurements from the `fix/independent-launch-work` branch. It does not establish production readiness. The [launch checklist](LAUNCH_CHECKLIST.md) remains the list of open gates.

## Retention and deletion

`purge_chat_events_older_than` existed but had no production caller or scheduling/configuration. It compared RFC 3339 timestamps as text against SQLite's space-separated timestamp; a regression test reproduced a missed expiry. The query now compares instants using one cutoff for selection and deletion.

A fixture replay reproduced deleted chat returning. [ADR 0023](adr/0023-deleted-chat-replay-markers.md) and schema v10 now record SHA-256 digests of deleted event IDs locally, atomically with explicit deletion or retention pruning, and both direct insert and ingestion reject those IDs. Tests cover replay, linked conversation deletion, restart, new IDs and v9 migration. The marker stores an unsalted identifier digest, not the raw ID or message content, is retained until the local database is reset, and is not part of the account snapshot. It cannot prevent reimport after identifiers change, from backups predating deletion, or after database reset; pre-v10 deletions cannot be reconstructed. Older Spawnt binaries ignore markers, so restart all Spawnt processes on the updated version. Session titles and other metadata can remain after chat deletion; agent-owned histories and backups are untouched. SQLite WAL and backups can retain old content; row deletion is not forensic erasure. No scheduled retention was added. Confirm a retention policy, backup handling and real source behavior before release.

## Network and account data inventory

`spawnt doctor --network` previously listed only loopback and Supabase and described all outbound traffic as opt-in. It now describes connection categories without printing tokens or claiming to test connectivity. In an isolated temporary home, `strace` observed no `connect`, `sendto`, or `recvfrom` calls while running this diagnostic. This is a single Linux execution, not a whole-product offline audit.

| Path | Trigger and data boundary | Evidence |
| --- | --- | --- |
| Daemon and PTY | Authenticated loopback HTTP/WebSocket; local ports vary. | `crates/cli/src/main.rs`, `crates/pty/src/lib.rs` |
| Account auth and sync | After account connection, HTTPS authentication/refresh and periodic owner-scoped account snapshot publication. URL/key are configurable; do not print credentials. The snapshot is not end-to-end encrypted. | `crates/daemon/src/sync.rs`, `crates/daemon/src/api.rs` |
| Remote | User starts or restores paired Remote; TLS WebSocket relay carries end-to-end encrypted frames, with connection metadata visible to infrastructure. URL is configurable. | `crates/daemon/src/remote.rs`, `crates/daemon/src/remote_trust.rs` |
| Remote entitlement | A signed local license can avoid the account check; otherwise Remote access checks the configured account service over HTTPS. | `crates/daemon/src/pro.rs` |
| Pricing | Explicit local API refresh retrieves the LiteLLM public price catalog from `raw.githubusercontent.com`. | `crates/daemon/src/api.rs` |
| Browser and agent CLIs | Browser account/website calls and provider traffic from installed CLIs follow those programs' settings. | `web/lib/supabase.ts`, `web/components`, agent CLI |

No implemented update-manifest GET was found in the reviewed Rust daemon/CLI or desktop shell. It is permitted/planned by SPEC; it must not be described as an observed current connection until verified. The privacy page now says that update-check implementation remains a release check. Recheck all network paths and configured destinations on the shipping build.

The current snapshot contains generation time, total/active/ongoing counts, up to 200 explicitly selected session records (ID, agent, surface, project label, model, state/presence, timestamps, token/context/cost metrics and trust), usage timelines with daily/provider/model totals, and quota pool fields (provider, pool, window, used/remaining percentage, reset, plan, basis and observation time). A temporary Remote discovery offer can add a room, public key, relay URL, expiry, label, pairing code and protocol version. The account request also sends the user ID and update time. New snapshots exclude chat, titles, local profile and relay ledger; the separate account settings service stores user-selected display/plan/pricing settings and account identifiers. Exact key-set tests now cover snapshot, session and quota fields; nested report evolution still needs contract review. Source migration defines owner RLS, but its presence is not proof of production configuration or two-account isolation. Historical snapshots and backups require a separate decision and verification.

## Adapters and automation

The repository contains versioned fixtures for Claude Code, Codex, Gemini and Antigravity, plus Cursor settings and Codex title/context cases. A new table-driven regression verifies that all seven adapters count truncated input and do not store chat from it. Existing parser tests exercise sample transcripts; Cline/Roo and Aider coverage includes synthetic inputs rather than a complete current real-log version matrix. Several adapters still fall back to `Utc::now()` when source timestamps are missing, including Cline, Aider, Gemini, Antigravity, Codex and Claude. Replaying such input can fabricate changing activity times and violates the intended deterministic-parser contract. The domain currently requires timestamps, so deciding how to represent missing times without silently discarding historical messages needs a separate design and fixtures. Do not infer equal chat, quota or control support from an adapter. Collect new anonymized current-version fixtures before claiming coverage.

Resume eligibility now rejects unknown quota, invalid percentages, future observations and readings older than the entry's eligibility time. This is a conservative local guard, not proof a provider has restored capacity. The queue still picks one matching provider snapshot; multiple windows, native busy/approval state and version behavior need acceptance. There is a crash/concurrency window between spawning a resume process and persisting the run count; a durable claim and recovery design is still needed before promising no duplicate resume. Native permissions must never be bypassed.

## Website checks

On 2026-11-01, the deployed sitemap listed 19 URLs; all returned HTTP 200 with matching canonical links. The pages exposed 22 distinct same-origin links and 15 external host/path URLs; all returned HTTP 200, and three JSON-LD blocks parsed as JSON. External checks stripped query strings and fragments. `robots.txt` returned 200 and contained the documented account/API exclusions. This did not validate query-dependent external links, semantic Schema.org correctness, search indexing, every download artifact or accessibility. Recheck after the final deployment and approve legal copy separately.

## Synthetic performance baseline

On Linux x86-64, kernel 6.18.57, an AMD EPYC 9V45 host exposed three logical CPUs to this environment. `scripts/measure-local-runtime.py` runs a debug build in a temporary home, using a fake `codex` command that sleeps; it measures `/proc` RSS and CPU ticks over four seconds per scenario and stops its sessions. This is a small observed sample, not a benchmark of real inference, Electron, active output, release builds, other platforms or sustained load.

| Scenario | Observed RSS | Observed CPU over sample |
| --- | --- | --- |
| Daemon idle | 20,620 KiB | 0.00% of one logical CPU |
| Daemon with three idle synthetic sessions | 20,972 KiB | 0.25% of one logical CPU |
| Three PTY hosts | 12,812, 12,872 and 12,720 KiB | 0.25%, 0.25%, 0.25% of one logical CPU |
| Three fake agent processes | 684, 684 and 684 KiB | 0.00% each |

Reproduce on Linux with `cargo build -p spawnt && python3 scripts/measure-local-runtime.py`. Establish budgets and measure release builds, real output, Electron and all launch OSes before publishing performance claims.

## Small product proposals for validation

These are options for discovery, not approved scope or uniqueness claims. Sources are vendor documentation; no competitor hands-on evaluation was performed.

1. **Read-only review first.** Show a bounded per-workspace diff, status and untracked-file notice without automatically applying or discarding changes. Test binary/large files, path containment and agent/session identity. [Conductor](https://www.conductor.build/docs/concepts/workflow) and [Warp](https://docs.warp.dev/code/code-review) document integrated review workflows. Validate whether users need review in Spawnt before adding PR or merge actions.
2. **Optional isolated workspaces.** Explore explicitly created Git worktrees and branches with visible location, setup and cleanup choices; never automatically remove user work or modify the original tree. [Conductor](https://www.conductor.build/docs/concepts/git-worktrees) documents managed worktrees. Validate demand, cross-platform behavior and retention before specifying the lifecycle.
3. **Checkpoint/restore.** Explore only after review and isolation: preview tracked/untracked impact, protect secrets, and require an explicit restore action. [Cline](https://docs.cline.bot/core-workflows/task-management) documents task checkpoints. Restore semantics and recovery need a separate design.
4. **Coordinated or cloud agents.** [Warp](https://docs.warp.dev/platform/orchestration) and [GitHub](https://docs.github.com/en/copilot/concepts/agent-capabilities/about-agents/about-github-copilot-agents) document coordination and cloud delegation. These change execution and privacy expectations; do not add them for parity without user research and an approved architecture.
