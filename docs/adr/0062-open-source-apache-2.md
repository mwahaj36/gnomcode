# 0062 - Open source under Apache-2.0, no paid tier in 1.0

- **Status:** Accepted. Supersedes [0002](./0002-closed-source-free-tray.md).
- **Date:** 2026-11-14

## Context

ADR 0002 chose closed source with a paid Pro tier. Comparable tools (ccusage, CodexBar, Happy, Vibe Kanban) are free and open source, so local features are table stakes. Gnomcode's differentiator is cross-vendor handoff, supported by trust labels, no telemetry and Windows and Linux support. With no users yet, adoption matters more than clone protection.

## Decision

- License the repository under the Apache License 2.0. The root `LICENSE` is the unmodified license text; `NOTICE` carries the copyright line.
- No paid tier in 1.0. Remote is free. No code path checks a license or entitlement.
- Business logic stays in Rust as an architecture rule only. Remove `obfstr!`, packer bans and license checks as protections.
- Contributions use DCO sign-off. The name and logo are covered by `TRADEMARKS.md`, not by the Apache license.
- GitHub hosts source and releases. Installers are never hosted on Vercel. Revenue options (hosted relay, team features, sponsors) are deferred and need their own ADR.
- The repository stays private until the owner decides to publish.

## Consequences

- Forks become legal. The trademark policy limits confusing use of the name.
- Dependencies must stay Apache-2.0 compatible (the `cargo deny` allow-list already is; copyleft stays denied).
- SignPath Foundation free signing becomes available once the repository is public.
- The signed license-file machinery in `core::license` and the license API are dead weight; removal is a follow-up.
