# 2026-11-14 Wire ingest watcher into the daemon (D3)

## Goal

Close the 1.0.1 release blocker: `IngestWatcher` existed (#217) but `DaemonOrchestrator::spawn_reconciler` still ran a full reconcile every two seconds, so the D3 idle CPU budget did not hold at runtime.

## Outcome

- `spawn_reconciler` now builds `IngestWatcher::new(2 s, 60 s)` and drives `run_reconcile_loop`: a 2 s tick that runs a pass only when the watcher saw a change or the idle back-off (2 s doubling to 60 s) elapsed.
- Fail open: if the OS watcher cannot be created or no path could be watched, the daemon logs a warning and polls every tick (previous behavior). Watcher error events mark the state dirty.
- Watched paths: agent roots (recursive), `GNOMCODE_HOME` (non-recursive, for `claude-limits.json`) and `GNOMCODE_HOME/sessions` (non-recursive, PTY records for conversation binding).
- `ingest_all`, checkpoints and the reconcile pass are unchanged, so truncation, rotation and no-double-counting behavior is unchanged.

## Problems and root-cause fixes

- The old `IngestWatcher::new` watched `GNOMCODE_HOME` recursively. The daemon's own `state.db` WAL writes live there, which would wake ingestion after every pass (feedback loop). Fix: non-recursive home watch and events whose paths are all `*.db*` files are ignored.

## Verification

- New tests: `reconcile_loop_runs_on_file_event_and_idles_otherwise` (one initial pass, no idle passes, file write triggers a pass, db writes do not) and `reconcile_loop_falls_back_to_polling_without_watcher`.
- Gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace`, `cargo deny check`.
- Idle CPU, release `gnomcode daemon`, isolated temp home with one Codex fixture, 15 s warm-up then 60 s sample, Windows 11: before 172 ms CPU (0.29% of one core), after 78 ms (0.13% of one core). Both are under the 1% budget on this tiny corpus; the saving grows with corpus size because idle passes no longer rescan every root every 2 s.
