# 0001 — Rust for core, CLI, daemon and hooks

- **Status:** Accepted
- **Date:** 2026-10-26

## Context

FleetHUD needs:
- a hook handler that runs on every Claude Code tool call,
- an always-on background daemon,
- painless `npx` installs on Windows, macOS and Linux,
- a tray app built with Tauri (Rust).

The original plan left the choice between Node.js/TypeScript and Go/Rust open.

## Decision

All runtime code is Rust, organized as a Cargo workspace (`crates/*`). The web dashboard is the only TypeScript, and it is a static bundle embedded into the binary.

## Consequences

- Hook startup is about 5 ms, versus 60–120 ms for Node. The per-tool-call overhead is invisible.
- Ships as a single static binary per target, with no native-module ABI breakage under `npx`. npm ships per-platform packages instead.
- The Tauri tray links the same crates. There's no second implementation.
- Low idle memory and CPU for the daemon.
- Slower iteration than TypeScript. We mitigate this with strict layering (pure `core`/`adapters`) so most logic is fast to unit-test.
- Contributors need a Rust toolchain.

## Alternatives considered

- **Node/TypeScript:** fastest to write, but hook latency, native SQLite modules and a ~50 MB runtime are dealbreakers.
- **Go:** good binaries, but no code sharing with Tauri.
