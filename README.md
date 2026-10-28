# spawnt

> **See, stop, and control your agents.**
> Track live agent state, per-model quota pools, and measured usage and estimated API-equivalent cost across Claude Code, Codex, Google Antigravity, Gemini CLI, Cline/Roo, and Cursor on Windows, macOS, and Linux. Local-first, with optional account pairing for metadata snapshots.

[![npm version](https://img.shields.io/npm/v/fleethud.svg)](https://www.npmjs.com/package/fleethud)

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
