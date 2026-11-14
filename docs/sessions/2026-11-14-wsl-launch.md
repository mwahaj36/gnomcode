# 2026-11-14 WSL launch and resume

## Goals
ADR 0060 PR 3: launch and resume agents inside a WSL distro through the PTY host.

## Outcomes
- `gnomcode_pty::wsl`: pure command builder (`wsl.exe -d <distro> --cd <linux path> -- <agent> ...`), distro-name validation, `command -v` probe, install hint, starting message.
- PTY host wraps launch and resume for `\\wsl.localhost\<distro>\...` workspaces and does not set a UNC working directory.
- `daemon::wsl_launch` preflight for HTTP and Remote launch: probe, install hint (422), `starting` and `wsl_distro` in replies.
- Live session, transcript and limits come from PR 2 ingest (UNC cwd matches the launched folder).

## Problems and fixes
- Login shells in WSL find the Windows `codex` shim under `/mnt/c` through interop. Fix: probe with a non-login `sh -c`, the same environment the launch uses, and treat `/mnt/<drive>/` as not installed.

## Decisions
Launch with the plain non-login environment, as ADR 0060 states.

## Verification
fmt, clippy `-D warnings`, `cargo test --workspace`, `cargo deny check`. Manual: `wsl.exe -d Ubuntu-22.04 --cd /home/wahaj -- sh -c 'pwd; command -v claude'` prints the folder and `/usr/bin/claude` (distro was already running).
