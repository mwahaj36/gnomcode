# Product

<!-- impeccable:product-schema 1 -->

Design-tool context for the impeccable skill. The authoritative product brief is [docs/PRODUCT.md](docs/PRODUCT.md); where they differ, that file wins. Facts marked *(inferred)* were inferred from the repository and the owner's redesign brief of 2026-11-25 while the owner was unavailable to answer the interview; confirm or correct them.

## Platform

web

## Users

Developers who already run one or more official AI coding agent CLIs (Claude Code, Codex, Antigravity) and want one local place to launch and follow sessions across projects, see usage and limits, switch models with continuity, and reach their desktop from a phone. They live in terminals, editors and agent chats for long stretches, often in dim rooms on large displays, and glance at the console between tasks. *(inferred)*

## Product Purpose

Gnomcode is a free, open source (Apache-2.0), local-first runtime and orchestrator around the official agent CLIs. The Rust daemon owns product logic; the console shows sessions, conversations, usage and limits with an explicit trust level on every number. Success is a developer who keeps their agents' work, cost and limits in view without leaving their flow.

## Positioning

One local workspace across the official CLIs, with redacted continuity for model handoffs, numeric provenance (exact, estimated, api, unknown) and paired encrypted Remote, with no telemetry and no paid tier.

## Operating Context

Used alongside an IDE and terminal on Windows, macOS and Linux, inside an Electron shell or a browser on loopback; Remote and the account console are used from phones. Sessions run for hours; the console is checked in short glances and occasionally driven for long conversations. *(inferred)*

## Capabilities and Constraints

See [docs/PRODUCT.md](docs/PRODUCT.md) and [AGENTS.md](AGENTS.md). Binding for design: business logic stays in Rust; every number declares its trust level; no emojis anywhere; no telemetry or remote font requests; loopback only; never escalate agent permissions; support is not uniform across agents and must not be overclaimed.

## Brand Commitments

Lowercase wordmark `gnomcode` and the solid triangle-and-square mark (`web/public/gnomcode-mark.svg`) are kept. Owner chose on 2026-11-25: replace the visual world, use Inter (self-hosted, already bundled), apply to the whole app including marketing.

## Evidence on Hand

Anonymized fixtures in `fixtures/`, real console screens rendered by the e2e harness, provider logos in `web/public`. No customer research, testimonials, user counts or press exist; do not fabricate them.

## Product Principles

1. Show what is true, label what is estimated, never invent a number.
2. The work comes first: conversations, sessions and limits before chrome.
3. Local and private by default; say plainly what leaves the machine.
4. Familiar patterns over novelty for anything people operate daily.
5. Fail open: Gnomcode never stands between a developer and their agent.

## Accessibility & Inclusion

WCAG 2.2 AA across console, Remote and marketing: keyboard operable, visible focus, 4.5:1 text contrast, 24px minimum targets (44px on touch), reduced motion and forced colors respected. *(inferred from owner brief)*
