# 2026-11-14 WSL support: ADR and SPEC

## Goals
Record the owner-approved WSL design (ADR 0060) before code lands.

## Outcomes
- `docs/adr/0060-wsl-support.md`, SPEC §3.5, §3.12, §3.13, §3.14 and §9 updated, `docs/PLATFORMS.md` WSL section.

## Problems and fixes
None. Docs only.

## Decisions
Discovery never boots a distro; explicit launch may. Slow poll for 9P roots. Per-account limits counted once.

## Verification
Docs only; `git diff | grep -c 'Â'` is 0. Local machine has Ubuntu-22.04 (WSL 2, running) for later read-only discovery tests.
