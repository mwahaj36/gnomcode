# 0060 — WSL support on Windows

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

Many Windows users run Claude Code, Codex or Antigravity (`agy`) inside a WSL distro. Their logs live under `\wsl.localhost\<distro>\home\<user>\` and are invisible to a Windows-only Gnomcode. Reading them must not cost the user anything they did not ask for: booting a distro takes seconds and memory, and the 9P file share does not reliably deliver change notifications.

## Decision

Approved by the owner on 2026-11-14.

1. **Discovery never boots a stopped distro.** Distros are enumerated from `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss` (no `wsl.exe` call, which could start the VM). Only running distros are read. A distro that starts later is picked up by a cheap periodic check. Any failure (no WSL, registry missing, share unreachable) fails open: Windows roots keep working.
2. **Explicit launch may start a distro.** Launching a new session in a distro the user picked may start it; the UI shows `Starting <distro>...` until it answers.
3. **Tagging and dedupe.** Sessions and sources carry their distro (`WSL: Ubuntu`). Project keys reuse the `wsl.localhost` normalization in `crates/core/src/project_path.rs`. A file reachable through two paths (`wsl.localhost` and `wsl$`) is read once. Account limits are per account: a Claude account used in Windows and in WSL is counted once.
4. **Watching.** WSL roots (`\wsl.localhost\<distro>\home\<user>\.claude`, `.codex`, `.gemini/antigravity*`) fall back to the slow poll when 9P delivers no change events. Windows roots keep events. Idle CPU stays within the D3 budget. Files are never held open.
5. **Launch and resume** run `wsl.exe -d <distro> --cd <linux path> -- <agent>` with `claude`, `codex` or `agy`. Skip-permission flags are never added (invariant 7). The daemon checks `command -v <agent>` first and gives an install hint when missing.
6. **Hook.** The Claude hook inside WSL needs `gnomcode integrate claude` run in the distro. The console detects a missing hook and offers `Integrate in WSL` (the Windows binary via interop, or instructions). Sessions work without it (fail open).

## Consequences

- WSL sessions appear next to Windows ones with a distro badge, and can be resumed.
- A stopped distro's history stays hidden until it runs; this is deliberate.
- Polling 9P adds latency (slow-poll interval) for WSL roots only.
- All registry, UNC and `wsl.exe` code lives in `crates/platform` under `cfg(windows)`; command building stays pure and tested.

## Alternatives considered

- `wsl.exe -l -v` as the only source: lists stopped distros too. Discovery reads the registry first and calls `wsl.exe -l --running -q` (which never starts a distro) only when a distro is registered, at most once a minute.
- Running a Gnomcode daemon inside each distro: doubles installs and accounts; rejected.
- Watching 9P with events only: misses changes silently; rejected.
