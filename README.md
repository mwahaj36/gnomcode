# gnomcode

Gnomcode is a local-first orchestrator for AI coding agents: launch Claude Code, Codex, Gemini CLI and other supported official CLIs, switch models with a compact handoff, and use encrypted remote access from a paired phone/browser. Designed for Windows, macOS and Linux, it brings conversations, model switching, usage and quota into one workspace.

## Why Gnomcode?

- Launch agents across projects without juggling separate terminals.
- Continue a task with another supported model using a locally compiled briefing.
- Read conversations and send eligible instructions from a paired device over E2EE.
- Compare usage, estimated API-equivalent cost and quota with explicit trust levels.

## Supported agents

Observation adapters cover Claude Code, OpenAI Codex (including discovered desktop/IDE logs), Gemini CLI and Antigravity. Hosted control uses the same official CLIs. Real authenticated Antigravity/model-switch acceptance remains open.

## Quick start

Use an approved platform build from [Downloads](https://gnomcode.vercel.app/downloads). Public 0.9.0 packages and non-Windows release acceptance remain launch gates. After approved npm publication, the standalone install command is:

```sh
npm install -g gnomcode
gnomcode console
gnomcode doctor
gnomcode launch codex --repo ./my-project
```

Install and authenticate the official CLI separately. Select a folder and agent in Workspace, complete native login/trust prompts in Terminal stream, then send a prompt. Local Gnomcode needs no account; agent inference can still require its provider network.

## Features

- **Launcher and chat:** official CLIs, independent PTY hosting, paged redacted local history and a separate local terminal.
- **Model switching:** a fresh target receives objectives, changed-file paths and recent activity. “Zero-token” means no inference call creates the briefing; the receiving agent still consumes tokens.
- **Remote access:** encrypted paired history, prompts, supported launches and bounded folder selection, with remembered revocable trust. Physical-phone and independent security approval remain open.
- **Cost tracking:** recorded tokens, estimated API-equivalent spend, cache savings and subscription comparisons.
- **Quota monitoring:** observed windows, estimates or unknown values, plus free local routing, guardrails and resume controls. Native behavior needs provider/version acceptance.

All local features are free. Remote and Teams are paid categories; commercial terms are not finalized. Optional account metadata sync is separate from E2EE Remote and needs a prelaunch minimization review.

## Product and status

- [Product brief](PRODUCT.md): what Gnomcode is and questions for discussion.
- [Launch checklist](docs/LAUNCH_CHECKLIST.md): blockers, causes, closure steps and evidence.
- [Engineering sessions](docs/sessions/README.md): individual dated records.

[![npm version](https://img.shields.io/npm/v/gnomcode.svg)](https://www.npmjs.com/package/gnomcode)

---

## Single Source of Truth

- **[`docs/SPEC.md`](./docs/SPEC.md)** — Product specification, system architecture, per-model quota engine, Resume Queue, roadmap, deliverables, and deployment plan.
- **[`AGENTS.md`](./AGENTS.md)** — Engineering rules, Rust standards, non-negotiable invariants, and testing requirements.
- **[`docs/adr/`](./docs/adr/)** — Architecture Decision Records.
- **[`.agents/skills/`](./.agents/skills/)** — Project runbooks (`rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`).

## Development

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```
