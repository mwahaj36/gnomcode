# 2026-11-14 WSL follow-ups

## Goals
Close the ADR 0060 gaps: provider sources, All chats filter, phone picker, paths and badges everywhere, Integrate in WSL, protocol hosts, live check.

## Outcomes
- Provider overview `wsl_sources`, rendered as badges on the card source line. Limits are not split: snapshots merge per provider, pool and window.
- All chats "where it runs" chips (Windows, each distro) and row badges.
- Remote `wsl_distros` RPC and phone "Run in" picker; older desktops leave the form Windows-only.
- `shortPath` names the distro; worktree and changes paths truncate in the middle with a tooltip; tray labels add `WSL: <distro>`.
- `claude_hook` state per running distro and `POST /api/v1/wsl/integrate`; `GNOMCODE_HOOK_EXE` lets the integrate command write the interop path.
- `codex app-server` and `agy acp` hosts wrapped with the same builder.
- RemoteCompanion `loadModels` lint warning resolved with a documented exception.

## Problems and fixes
- Live launch reported Claude missing although `/usr/bin/claude` existed. Root cause: `wsl.exe -- cmd` re-joins arguments into one shell line, so `sh -c 'command -v "$1"'` lost its script. Fix: `--exec` for probe, launch, resume and protocol hosts; arguments pass verbatim.
- Playwright first ran against a binary built before the console bundle; rebuild the daemon after the bundle.

## Decisions
`--exec` instead of `--` (ADR 0060 and SPEC updated). Integrate is a user-initiated action that runs `gnomcode integrate`, which backs up first.

## Verification
Rust gate, web typecheck/unit/bundle check, desktop typecheck/lint/tests, full Playwright. Live on Ubuntu-22.04 (already running): distros listed with `claude_hook: missing`, Claude launched through the API in `\\wsl.localhost\Ubuntu-22.04\home\wahaj`, hosted record with observed model, `claude` process seen in the distro, stop returned 200 and the process exited; Codex returned the install hint; distro state unchanged. Not run live: booting a stopped distro (would need terminating the user's running distro) and the real Integrate write to the distro's Claude settings.
