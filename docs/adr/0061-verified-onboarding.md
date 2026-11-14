# 0061 — Verified provider onboarding from one Rust status endpoint

- **Status:** Accepted
- **Date:** 2026-11-14

## Context

The first-run tour (#183) only described the console. Users still had to discover install, sign-in, `gnomcode integrate claude`, protocol approvals (ADR 0057) and Claude limits (ADR 0030) on their own, and nothing confirmed each step worked. Settings and the doctor reported pieces of the same state in different words.

## Decision

- `GET /api/v1/onboarding/status?provider=&distro=` (authenticated, loopback) returns, per provider, an ordered checklist: `installed`, `signed_in`, `integrated` (Claude) or `protocol_approvals` (Codex, Antigravity, optional), `limits` (Claude, optional) and `first_data`. Each step has `state` (`done | todo | waiting | unknown`), `trust` (`exact | estimated | unknown`), a one-line note and, when not done, the official command the user runs in their own terminal.
- All detection is in `crates/daemon/src/onboarding.rs`. Sign-in is a file-presence check (Claude `.credentials.json`, Codex `auth.json`, Antigravity's folder as `estimated`); token values are never read or returned. The only account field is the Claude account email from `.claude.json`.
- Gnomcode never runs login commands. Agent config changes stay with `gnomcode integrate` (invariant 2).
- With `distro`, checks target a running WSL distro's home over `\wsl.localhost` and `command -v` inside it; a stopped distro is reported `unknown` and is never booted (ADR 0060). Install commands then use the Linux form.
- The CLI `--version` probe is cached for six seconds so two-second polling stays cheap.
- One React checklist component renders this in onboarding, Settings > Providers and the doctor link target, polling every two seconds only while visible.

## Consequences

One source of truth for setup state. Antigravity sign-in is only estimated until a session ingests. A "Run for me" integrate button is deferred; the copyable command is the only path for now.

## Alternatives considered

- Detect in TypeScript: violates invariant 8.
- Extend `GET /api/v1/doctor`: it probes disks and every CLI on each call and has no per-provider or WSL shape.
