# Spawnt

Updated 2026-10-29. This product brief is for discussion. [SPEC](docs/SPEC.md) owns technical requirements and status; [LAUNCH_CHECKLIST](docs/LAUNCH_CHECKLIST.md) owns remaining release work.

## What the product is

Spawnt is a local-first runtime and orchestrator for AI coding agents. Developers get one place to launch official agent CLIs, work in their conversations, switch models with a compact handoff, and control supported sessions from a paired phone or browser. Windows, macOS and Linux are design targets; Windows currently has the locally verified desktop build.

The agent does the coding. Spawnt coordinates the workflow around it: workspace selection, session state, conversation continuity, usage and quota visibility, and remote interaction. It does not replace vendor accounts, subscriptions, authentication or permissions.

## Who it is for

Developers running multiple agents across several projects. They already know Claude Code, Codex or Gemini CLI and want less terminal juggling, continuity when a model reaches its limit, and a way to direct work away from the desk. API users also need cross-provider spend visibility. Teams are a later commercial direction.

## Three main workflows

1. **Launch and work.** Select a folder and installed supported CLI. Spawnt opens a conversation backed by the official process. Use Terminal stream for login, trust and permission prompts. A separate same-workspace terminal handles local shell work. Discovered sessions remain readable without granting control of their processes.
2. **Switch and continue.** Pick a supported target agent/model. Rust compiles redacted objectives, changed-file paths and recent activity into a handoff, launches a fresh target session and submits it. Original history remains available. This transfers a compact briefing, not provider-internal state. “Zero-token” means no model call generates the briefing; receiving agents still consume input tokens and can incur charges.
3. **Step away and stay in control.** Start Remote in desktop Profile. Sign into the same Spawnt account on the phone, enter the desktop's eight-digit discovery code, then confirm the separate security code on both devices. The encrypted channel carries conversations, prompts, session discovery and bounded folder selection. Remembered trust reconnects after reload/backgrounding and expires after seven inactive days. Remote cannot execute arbitrary shell commands or bypass native permissions.

## Supporting capabilities

- A searchable session list, paged local chat history, safe Markdown and per-session drafts held in browser memory.
- Token usage, estimated API-equivalent spend, cache savings and plan-value comparisons across agents/projects.
- Per-provider/model quota windows and resets when recorded by a reliable source. Missing information stays unknown.
- Free local guardrails, alerts, Resume Queue, routing policy and exports. Native agent acceptance varies by operation.
- A separate PTY owner keeps agent processes independent of the desktop UI; full crash-survival and reattachment acceptance remains a platform release gate.

## What agent support means

| Agent | Observation | Hosted control |
| :--- | :--- | :--- |
| Claude Code | Local adapter/history | Official CLI |
| OpenAI Codex | CLI/desktop/IDE logs and observed quota | Official CLI, no desktop takeover |
| Gemini CLI | Local adapter/history | Official CLI |
| Antigravity | Local transcript adapter | Official `agy` CLI; live authenticated acceptance outstanding |
| Cline / Roo | Editor log adapter | No hosted editor takeover |
| Cursor | Local log adapter | No hosted editor takeover |
| Aider | Opt-in local adapter | No hosted launcher |

An adapter does not imply equal chat, cost, quota or Remote capabilities. Model discovery uses fresh visible Codex CLI cache entries. Other CLIs retain native model selectors where no reliable local catalog exists.

## Data and trust

Rust owns parsing, pricing, quota math, routing and security. SQLite defaults to `~/.spawnt/state.db` or `SPAWNT_HOME/state.db`. It contains metadata, source checkpoints and redacted conversation records for continuity. Agent-owned histories are preserved. Redaction reduces exposure but cannot guarantee removal of every user-entered secret.

Local HTTP requires authentication and binds to loopback. Remote uses outbound TLS and end-to-end encryption between paired devices. Optional account metadata sync is a separate service without E2EE. Its current live snapshot includes project/session labels, profile/report data and automation metadata; it must not be described as a hashed-project-only feed. Field-level minimization is a launch blocker. Chat events are excluded from that snapshot.

Spawnt does not read official agent subscription tokens. Its own optional account credentials are separate. No product analytics or telemetry are permitted. Official agent CLIs contact their providers under their own settings.

Numbers declare `exact`, `estimated`, `api` or `unknown`. Exact token counts do not make calculated dollar amounts exact invoices. Model catalog entries do not prove current entitlement or quota.

## Business model

Closed-source software. All local features are free, without a Spawnt account or local license. Remote and Teams are the paid categories; prices and commercial terms are not finalized. Existing signed-license/live-account checks gate the Remote prototype. Old paid-local tiers and historical 1.x notes do not define the current offer. Development version: 0.9.0.

## Current state

The repo contains the Rust engine, CLI, Electron shell, Next.js site, chat/terminal workspace, routing/handoff and encrypted Remote implementation. Prior sessions record automated browser/relay checks and Windows packaging. This is a prelaunch product.

Physical-phone acceptance, real authenticated model/provider switching, macOS/Linux desktop acceptance, independent Remote security review, metadata privacy review, legal/commercial approval and distribution remain open. Teams is a roadmap direction, not a complete service. [LAUNCH_CHECKLIST](docs/LAUNCH_CHECKLIST.md) explains how to close each gap.

## Positioning

**The runtime for your AI coding agents.** Launch agents, switch models mid-task and continue from your phone, with one view of usage and limits.

Lead with launch, continuity and encrypted remote control. Reporting supports those workflows. Marketing should be optimistic and outcome-focused; documentation records limitations and launch evidence. Avoid promising perfect context transfer, free provider inference, universal agent control or security certification.

## Questions for product discussion

- Should first run prioritize launching a new agent or importing existing sessions?
- What evidence is required before charging for Remote?
- Which account snapshot fields should be removed or require separate consent?
- What is the smallest useful Teams product beyond personal snapshots?
- Which platforms and channels will be enabled on launch day?
