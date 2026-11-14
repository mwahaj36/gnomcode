# 2026-11-14 Watch agent roots created after daemon start

## Goal

Follow-up to #276: agent roots that did not exist when the daemon started were only picked up by the 60 s idle fallback scan.

## Outcome

- `IngestWatcher` keeps its root list and derives the watch set: an existing root is watched directly (plus its parent, non-recursive); a missing root is watched through its nearest existing ancestor, non-recursive.
- The watcher callback classifies events against the roots. Create, remove or rename events on a root path or a missing intermediate folder mark a layout change. Changes inside an active root mark data. Everything else (home-dir churn, a `Modify` that a parent reports on the root folder itself, Gnomcode's own `*.db*` files) is ignored.
- On the next `take_due` (one 2 s tick), a layout change re-derives the watch set, drops ancestor watches nobody needs, re-creates every watch (fresh OS handle after delete and recreate) and forces a pass. The idle fallback scan also rechecks the roots.
- Fail open: watcher errors or a poisoned lock count as a layout change and a dirty pass. No OS watcher still means plain polling.

## Problems and root-cause fixes

- Delete and recreate was missed on Windows: a recursive watch does not report its own folder's deletion. Fix: also watch each root's parent non-recursively.
- That parent watch reports a `Modify` on the root folder whenever anything inside it changes, which woke ingestion on our own db writes. Fix: an event whose path is exactly a root is data only through the root's own watch, and is a layout change only for structural kinds.

## Verification

- New test `reconcile_loop_picks_up_root_created_after_start`: home-dir writes and an unrelated folder trigger no pass. Creating `.codex/sessions` after start triggers a pass within ticks. Writes inside it trigger passes. Deleting and recreating it triggers a pass again.
- Existing watcher and reconcile loop tests still pass (5 repeated runs).
- Gate: `cargo fmt --check`, clippy `-D warnings`, `cargo test --workspace`, `cargo deny check`.
