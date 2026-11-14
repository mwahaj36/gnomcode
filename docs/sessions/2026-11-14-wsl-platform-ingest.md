# 2026-11-14 WSL discovery and ingest roots

## Goals
ADR 0060 PR 2: discover running WSL distros without booting any, ingest their agent roots, poll 9P.

## Outcomes
- `gnomcode_core::wsl`: distro tag, Linux path to UNC and back.
- `gnomcode_platform::wsl`: `Lxss` registry parsing, running-list parsing (UTF-16), stopped-distro skip, home root building, two-spelling dedupe, 60 s cached discovery merged into `discover_agent_roots`.
- Ingest maps Linux project folders from WSL logs to `\wsl.localhost\<distro>\...`; project keys fold `wsl$` into `wsl.localhost`.
- Watcher gives WSL roots no OS watch; the back-off poll covers them.

## Problems and fixes
- Hermetic tests would have read real WSL logs. Root cause: discovery ignores agent overrides. Fix: pinning all three agent homes (or `GNOMCODE_DISABLE_WSL`) turns WSL discovery off.

## Decisions
Running list via `wsl.exe -l --running -q` only after the registry shows a distro; it never starts one.

## Verification
fmt, clippy `-D warnings`, `cargo test --workspace`, `cargo deny check` pass. Live ignored test on this machine: registered and running `Ubuntu-22.04`, four existing roots found, distro state unchanged.
