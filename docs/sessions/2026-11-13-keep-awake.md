# 2026-11-13 - Keep-awake sleep inhibitor (research gap G4)

## Goals
- Keep the laptop awake while agent work or scheduled delivery is pending, as SPEC 3.14.3 requires for scheduled messages and failover.

## Outcomes
- New `gnomcode_platform::keepawake::SleepInhibitor`: ref-counted, with a private `Backend` trait. Windows holds `ES_CONTINUOUS | ES_SYSTEM_REQUIRED` on a dedicated thread, macOS spawns `caffeinate -i -w <pid>`, Linux spawns `systemd-inhibit --what=idle:sleep` around a self-terminating shell loop.
- The daemon automation loop (`automation::sync_keep_awake`, once per second) holds one reference while a hosted session is busy, a scheduled message is pending or a resume is armed.
- SPEC 3.14.3 updated.

## Problems encountered
- Gap verified: no `SetThreadExecutionState`, `caffeinate`, `systemd-inhibit` or `powerSaveBlocker` anywhere in the repo.
- `SetThreadExecutionState` is per thread, so a tokio worker cannot hold it; a dedicated thread does.

## Decisions
- No new dependency: the `windows` crate is not in the workspace, so the existing `#[link(name = "kernel32")]` pattern is used (unsafe confined to `crates/platform` with SAFETY comments).
- No settings toggle in the UI: `UserSettings` exists in core but is not persisted or served anywhere. `GNOMCODE_KEEP_AWAKE=0` disables the inhibitor instead.
- No ADR: this adds no architecture boundary beyond the platform crate's existing role.
- The paired-phone live-channel trigger from the research report is not implemented (not asked for in this change).

## Verification
- Unit tests with a fake backend cover first-acquire start, last-release stop, extra release and fail-open on backend failure.
- The real OS backends were not exercised against an actual sleep; only compiled on the host platform.
