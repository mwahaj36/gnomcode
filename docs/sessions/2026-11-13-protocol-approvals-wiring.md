# 2026-11-13 - Protocol approvals wiring (Codex app-server, agy acp)

## Goals
- Wire the unused ADR 0045 and ADR 0047 translators into launches behind an opt-in per-agent setting, so approvals reach the existing ADR 0032 API, console and phone. Terminal mode stays the default.

## Outcomes
- `crates/pty/src/protocol_host.rs`: `protocol_approvals` setting (`~/.gnomcode/protocol-approvals.json`, default off), `run_protocol_host` and a driver for both protocols. The host serves the same loopback API as a PTY host (shared `serve_host`, extracted from `run_pty_host_resuming` without behaviour change), so console, Remote and attach need no change.
- `gnomcode pty-host` picks the protocol host only for a new launch of an opted-in agent; resumes and everything else keep the PTY.
- `gnomcode protocol-approvals enable|disable|status codex|antigravity`.
- ADR 0057 and SPEC section 3.10 updated.

## Problems and root-cause fixes
- Both translators read `decision` from `GET /api/v1/approvals/{id}/wait`, but the daemon replies `outcome`. Every approval would have resolved to no decision (cancel). Fixed in both; a loopback mock daemon test covers allow, deny and none.
- Codex cancel interrupts the whole turn, so with nobody watching the console or phone the turn is cancelled rather than left waiting. Documented in the ADR.
- Antigravity's default model had a zero Gemini 4 Argon quota; live runs used `claude-sonnet-5-5-low` through `session/set_model`.

## Live verification (real CLIs, temp workspaces, throwaway prompt, no permission-skipping flags)
Direct protocol probes before wiring: `codex app-server` (codex-cli 0.166.0, approval policy untrusted) raised `item/commandExecution/requestApproval`; `cancel` and `decline` wrote nothing, `accept` created the file. `agy acp` (Antigravity ACP Server 1.5.8, default mode) raised `session/request_permission` with options `allow` (allow_once) and `deny` (reject_once); cancelled and deny wrote nothing, allow created the file. Both matched the modules.

End to end through an isolated daemon (`GNOMCODE_HOME` temp dir, port 41999, launch via `POST /api/v1/sessions/launch`, prompt via host `/input`, approval answered by `GET /api/v1/approvals` + `POST .../decision` as the console does; the user's own Codex config policy, no override):

| Agent | Console decision | Result |
| :--- | :--- | :--- |
| codex | allow | approval listed (`agent=codex tool=Bash`), `[ALLOW]`, `probe.txt` created |
| codex | deny | `[DENY]`, file not written, turn continued |
| codex | none (90 s hold) | `[CANCEL] no decision`, turn interrupted, file not written |
| antigravity | allow | approval listed (`tool=edit`), `[ALLOW]`, `probe.txt` created |
| antigravity | deny | `[DENY]`, file not written |
| antigravity | none (90 s hold) | `[CANCEL] no decision`, file not written |

With the setting off, a Codex launch through the same daemon produced the normal terminal (no protocol banner, escape sequences present).

## Verification
- `CARGO_INCREMENTAL=0 cargo fmt --all`, `clippy --workspace --all-targets -- -D warnings`, `test --workspace`.
