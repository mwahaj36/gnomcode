# Gnomcode product brief

This describes Gnomcode's implemented surfaces and their boundaries, not a guarantee of production readiness. Gnomcode is free and open source under the Apache License 2.0. Engineering invariants live in [AGENTS.md](../AGENTS.md); detailed design notes, ADRs and release gates are kept in the maintainers' local `localdocs/` tree, which is not part of the public repository. Public documentation is in tracked `docs/`.

## Product and audience

Gnomcode is a local-first workspace around existing AI coding agent CLIs. It aims to help developers launch and follow sessions across projects, preserve useful conversation continuity when changing agents, see usage and limits, and interact with an online desktop from a paired device. The agent and its provider perform the coding and inference; Gnomcode does not supply the user's vendor account, entitlement or permissions.

The intended user is a developer already using one or more supported coding agents who wants a common place to manage sessions and understand usage. Demand is a hypothesis; there is no independent customer research establishing it yet.

## What exists and what it does

| Capability | User value and boundary | Evidence |
| --- | --- | --- |
| Local launch and conversations | Select a workspace and supported official CLI, converse and use terminal access for native prompts. A separate PTY owner is implemented. Process survival and reattachment across all supported OSes still need acceptance. | `crates/pty/src/lib.rs`, `crates/daemon/src/console.html` |
| Discovery and history | Agent adapters ingest supported local records; session list, metadata search and paged Gnomcode-owned redacted chat help users return to work. Search does not cover message content. Redaction is not guaranteed to catch every secret; agent-owned history remains separate. | `crates/adapters`, `crates/ingest`, `web/lib/console-presentation.js`, `crates/store` |
| Model/provider handoff | A bounded briefing can carry selected context into a target; provider-internal state is not transferred and the receiving model consumes tokens. Real authenticated switching and model availability require acceptance. | `crates/daemon/src/lib.rs`, `crates/daemon/src/api/handoff.rs` |
| Usage and quota | Recorded usage and pricing can inform estimated API-equivalent costs and quota views. Completeness varies by agent and source; estimates are not invoices and unknown values must remain unknown. | `crates/core`, `crates/adapters`, `web/lib/console-presentation.js` |
| Local rules and resume | Guardrail rules, alerts, routing and a resume queue are implemented. Hard-stop enforcement is Claude-specific and fail-open; it is not a guaranteed cross-provider billing cap. Native compatibility and timing need real-agent validation. | `crates/daemon/src/pro.rs`, `crates/core/src/rules.rs` |
| Paired Remote | A paired browser can use an encrypted channel for supported conversation and control operations while the desktop is online. Native linking requires confirmation that every signed-in account device can access the host. Account-wide discovery, automatic encrypted connections, reconnection and host-wide disable are implemented; real-phone acceptance and independent security review remain open. It is not an independently running cloud agent. | `crates/daemon/src/remote.rs`, `web/components/RemoteCompanion.tsx` |
| Optional account sync | Account-backed metadata snapshots support remote account views. This is separate from encrypted Remote and includes selected session metadata, project names, reports and quota; consent and historical cloud copies need review. The session list is limited to 200, with the total reported separately. | `crates/daemon/src/sync.rs` |
| Desktop and CLI | Electron shell, tray and CLI provide local entry points. Windows, macOS and Linux are design targets, not equally verified release claims. | `apps/desktop`, `crates/cli`, `crates/tray` |

## Agent support is not uniform

1.0 supports exactly Claude Code, Codex and Antigravity. Cline/Roo, Cursor and Aider adapters were removed 2026-11-12 and the Gemini CLI adapter 2026-11-13. The launcher accepts Claude, Codex and Antigravity CLI names; observation does not establish hosted control, complete history, model switching or quota coverage for each. Installed CLI versions, authentication and available provider entitlements govern what actually works. Test real versions before making support claims.

## Why someone might choose it

The proposed advantage is a combination: keep familiar official CLI workflows, see local cross-agent activity and usage in one place, carry bounded context between providers, and interact with the online machine remotely. Local-first operation and explicit uncertainty labels can appeal to developers who value control and want to understand data provenance. These are hypotheses, not evidence of uniqueness: other products provide parallel workspaces, code review and remote agents. Gnomcode currently has weaker managed isolation and integrated review workflows than selected competitors.

## Data, trust and limits

Gnomcode-owned state is local by default, including redacted conversation records. Redaction reduces exposure; it is not a confidentiality guarantee. Deleting Gnomcode history does not imply deleting agent-owned logs, backups or forensic remnants. Local API security and Remote encryption are implemented controls, not independent certification. Account metadata sync has a separate trust model from Remote. Agent CLIs contact their own providers; optional update, pricing, vendor, account and relay traffic depends on configuration. No telemetry is intended by the product invariants.

## Licensing and release state

Gnomcode 1.0 is free and open source under the Apache License 2.0, with no paid tier; Remote is free. There is no Teams product. Any future paid hosted service would need its own terms published before launch. Before going public, close the remaining gates: strip AI attribution from commit history, decide code signing, land or document macOS/Linux packaging, and run a live multi-agent handoff. These gates are tracked in the maintainers' local launch checklist.
