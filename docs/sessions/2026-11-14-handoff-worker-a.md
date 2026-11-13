# Handoff fold: Worker A (PR #185)

Date: 2026-11-14. Source: running log PR #185 (`docs/handoff/worker-A.md`, plus `2026-11-13-worker-a-ownership-blocker.md`), closed unmerged. Statuses checked against main `f255fa5` on 2026-11-14.

## Decisions

- The first stop (G5 blocked: Remote dispatcher in `remote.rs` and `lib.rs` module registration outside Worker A ownership) was resolved by allowing minimal wiring; G5 backend landed as #187.
- Approvals use structured protocols and never auto-approve: Codex `app-server` (ADR 0045), Antigravity `agy acp` (ADR 0047). Gemini dropped by ADR 0058.
- E3: unproven features frozen behind off-by-default toggles (ADR 0049); later superseded for Teams and extra adapters by deletion (ADR 0058).

## Item status

| Item | PR | State |
| :--- | :--- | :--- |
| Green baseline | #189 | merged |
| G5 interrupt backend | #187 | merged |
| G6 agy resume | #193 | merged |
| G7 usage fallback | #197 | merged |
| G11 env scrub | #200 | merged |
| G12 Unix process-group stop | #202 | merged |
| G13 Codex/Gemini discovery | #203 | merged |
| C files, changes, artifacts, attachments, quick actions | #205 #207 #208 #210 #212 | merged |
| G8 Claude error parsing | #213 | merged |
| G3 vt100 + G14 guarded Enter | #214 | merged |
| Codex app-server approvals | #215 | merged |
| Antigravity ACP approvals | #216 | merged |
| D3 notify watcher + back-off | #217 | merged, see open item 1 |
| Antigravity RESOURCE_EXHAUSTED | #218 | merged |
| Failover e2e | #219 | merged |
| E3 freeze toggles | #220 | merged |
| 7c Gemini ACP approvals | none | closed: dropped, Gemini removed from 1.0 (ADR 0058) |
| 10 E2 split api.rs and store/lib.rs | none listed | done on main (`api.rs` 648 lines, `api/` domain modules, `store/lib.rs` 122 lines) |
| Live checks | | Claude 429 fixture anonymized and parsed; `agy acp` 1.5.7 and `codex app-server` 0.166.0 `initialize` verified live |

## Still open

1. D3 watcher not wired into the daemon: `IngestWatcher` (`crates/ingest/src/watcher.rs`) is not referenced by `DaemonOrchestrator::spawn_reconciler` (`crates/daemon/src/lib.rs`). The idle CPU budget (under 1%) is therefore not delivered at runtime despite the checked roadmap line. 1.0-relevant: follow-up added to `docs/ROADMAP_1.0.md`.
2. Not verified live: multi-turn failover with a truly exhausted Claude OAuth token; HTTP/Remote feature-toggle sync with Supabase (`account_sync_enabled = true`); Codex/Antigravity protocol approvals end to end through a phone.
3. Roadmap bookkeeping: section C boxes and duplicate F1/F2 lines are stale (see Worker B fold).
