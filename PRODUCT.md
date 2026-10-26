# PRODUCT.md — FleetHUD

## Product Truth
FleetHUD is live mission control for AI coding agents (Claude Code, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor) running across Windows, macOS, and Linux. It reads agents' local logs, normalizes them, and provides real-time state, token consumption, cost telemetry, and quota limits through a CLI, a local web dashboard, and a native system tray application.

## Non-Negotiable Product Invariants
1. **Never store content**: Zero prompts, zero source code, zero tool outputs persisted to disk or sent across networks. Content is redacted via `core::redact` prior to any persistence.
2. **Read-only by default**: FleetHUD monitors without altering agent environments unless explicitly instructed (`fleethud integrate`).
3. **Fail open**: Zero latency bottlenecks or blocking on host agent processes (50ms timeout ceiling).
4. **No telemetry**: Zero tracking, analytics, or unsolicited network egress.
5. **Trust levels**: Every number and telemetry metric explicitly indicates provenance (`exact | estimated | api | unknown`).
6. **No emojis**: Clean engineering typography, geometric SVG iconography, and plain-text telemetry badges (`[WAIT]`, `[RUN]`, `EXACT`, `EST`).

## Target Audience & Modes
- **Primary Persona**: Senior engineers and AI researchers running 2+ autonomous coding agents concurrently across personal subscriptions and API credits.
- **Web Surface Mode**: **Persuade** (marketing & downloads), **Operate** (mission control console & live log monitoring).
- **Native Tray Surface Mode**: **Operate** (high-density status glances, bring-to-front window switcher, quick quota checks).
