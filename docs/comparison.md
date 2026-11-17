# How Gnomcode compares

A feature-level comparison with similar open-source tools for watching and running AI coding agents. This is about capabilities, not price - the tools below are all free and open source, and so is Gnomcode.

## Similar tools

- **ccusage** - reads Claude Code logs and reports token usage and cost. CLI and a menubar companion.
- **CodexBar** - macOS menubar app showing Codex (and some Claude) usage and quota.
- **Happy** - mobile and web client that pairs with a coding agent for remote prompting.
- **Vibe Kanban** - a board for running and tracking multiple agent tasks.

Capabilities change often; check each project directly before relying on a row.

## Feature matrix

| Feature | Gnomcode | ccusage | CodexBar | Happy | Vibe Kanban |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Usage and cost from agent logs | Yes | Yes | Yes | No | Partial |
| Live quota and reset windows | Yes | Partial | Yes | No | No |
| Trust label on every number | Yes | No | No | No | No |
| Multiple agents in one view | Claude Code, Codex, Antigravity | Claude only | Codex (+some Claude) | Varies | Varies |
| Launch and resume agents | Yes | No | No | Partial | Yes |
| Cross-vendor handoff (quota -> another agent) | Yes | No | No | No | No |
| Remote access from phone/browser (E2EE) | Yes | No | No | Yes | Partial |
| Guardrails, alerts, resume queue | Yes | No | No | No | Partial |
| Windows and Linux | Yes | Yes | macOS-focused | Varies | Yes |
| Local-first, no account, no telemetry | Yes | Yes | Yes | Varies | Varies |

## Where Gnomcode is different

The distinguishing capability is **cross-vendor handoff**: when one agent runs out of quota or context, Gnomcode compiles a bounded briefing (objective, changed files, recent activity) and starts the task on another agent - Claude Code, Codex or Antigravity, any direction. The usage, quota and remote features overlap with the tools above; the handoff does not.

Supporting differences: a trust label (`exact`, `estimated`, `api`, `unknown`) on every reported number so an estimate is never shown as exact; all three supported agents in one place instead of one tool per vendor; and first-class Windows and Linux rather than macOS-first.

## Where it is weaker

Gnomcode has less managed task isolation and no integrated code-review workflow compared with task-runner tools like Vibe Kanban. Its remote and handoff paths are newer and have had less real-world use than the single-purpose incumbents. Native macOS and Linux packages are built and smoke-tested in CI, though physical desktop UI verification on both OSes remains limited.
