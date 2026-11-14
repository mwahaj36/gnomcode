# Gnomcode

**One place to launch your AI coding agents, watch their usage and limits, and hand a task from one to another when it hits its quota.**

Gnomcode reads the logs of Claude Code, Codex and Antigravity to show live sessions, cost and quota, launches and resumes them through their official CLIs, and compiles a briefing so you can continue a task on a different agent. Everything runs locally. No account, no telemetry.

<!-- DEMO GIF SLOT: drop a Claude Code -> Codex handoff recording at .github/assets/handoff-demo.gif -->
<p align="center">
  <img src=".github/assets/handoff-demo.gif" alt="Handing a task from Claude Code to Codex" width="760">
</p>

[![npm version](https://img.shields.io/npm/v/gnomcode.svg)](https://www.npmjs.com/package/gnomcode)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

## Install

```sh
npm install -g gnomcode
```

Then open the console:

```sh
gnomcode console
```

Prebuilt installers for Windows are on the [Releases page](https://github.com/mwahaj36/gnomcode-releases/releases). macOS and Linux: install with npm above, or build from source (see [Development](#development)).

## First run (no account, under a minute)

1. Install and sign in to at least one agent CLI yourself (`claude`, `codex` or `agy`) - Gnomcode never handles their credentials.
2. `gnomcode console` opens the local dashboard in your browser. It binds to `127.0.0.1` only.
3. Pick a folder and an agent, finish the agent's own login/trust prompt in the terminal pane, and send a prompt.
4. `gnomcode doctor` lists every endpoint the daemon can reach, so you can confirm what is and isn't local.

No sign-up. The only outbound call Gnomcode makes on its own is an opt-out update check.

## Supported agents

| Agent | Observe sessions, usage, quota | Launch and resume | Approve prompts remotely | Handoff target |
| :--- | :--- | :--- | :--- | :--- |
| Claude Code | Yes | Yes | Yes | Yes |
| Codex | Yes | Yes | Opt-in protocol mode | Yes |
| Antigravity | Yes | Yes | Opt-in protocol mode | Yes |

What actually works depends on the installed CLI version, your authentication and your provider entitlements. Observation is the most complete; hosted control and model-switch acceptance vary by agent and are still being hardened. Test your own versions before relying on a claim.

## Platforms

| Platform | Status |
| :--- | :--- |
| Windows x64 | Installer and npm |
| macOS, Linux | npm or build from source; native packaging not yet release-tested |

## What it does

- **Launch and chat** - run the official CLIs under a managed PTY, with paged, redacted local history and a separate terminal pane.
- **Cross-vendor handoff** - a new agent receives the objective, changed-file paths and recent activity as a compiled briefing. No inference call builds it; the receiving agent still spends its own tokens.
- **Usage and quota** - recorded tokens, estimated API-equivalent cost and quota windows. Every number carries a trust label (`exact`, `estimated`, `api`, `unknown`); an estimate is never shown as exact.
- **Remote** - pair a phone or browser over an end-to-end-encrypted relay to read sessions, send prompts and answer approvals. Trust is remembered and revocable.
- **Guardrails and resume** - budget and loop rules, alerts, routing and a resume queue, all local. Hard-stop enforcement is Claude-specific and fails open.

## Privacy

- Loopback only: servers bind to `127.0.0.1` and require a bearer token plus Host/Origin checks.
- No telemetry: the only self-initiated network call is the opt-out update-manifest check. `gnomcode doctor --network` lists them all.
- Local history is redacted before storage; agent-owned logs are left untouched. Redaction reduces risk but is not a guarantee - do not type secrets into chat.

## Free and open source

Gnomcode is licensed under the [Apache License 2.0](LICENSE). Everything in 1.0 is free, Remote included. The name and logo are covered by [TRADEMARKS.md](TRADEMARKS.md).

Contributions are welcome - see [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) (DCO sign-off required), [SECURITY.md](SECURITY.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Engineering rules live in [AGENTS.md](AGENTS.md); the product brief is [docs/PRODUCT.md](docs/PRODUCT.md) and a feature comparison with similar tools is in [docs/comparison.md](docs/comparison.md).

## Development

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

The web console and desktop shell are in `web/` and `apps/desktop/` (`npm ci` then the package scripts). The Rust daemon owns all product logic.
