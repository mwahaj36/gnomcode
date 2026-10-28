# 0002 — Closed source; free tray; Pro unlocked by signed license file

- **Status:** Accepted
- **Date:** 2026-10-26

## Context

CodexBar (free, macOS) already offers quota and cost tracking in the menubar, so a paid tray would compete with a free, polished tool. Our differentiators are live cross-agent state, per-model quota pools, and cross-platform support (Windows, macOS, Linux), and all three need wide adoption to matter. At the same time, the founder wants Spawnt to be a proprietary, closed-source product rather than open source.

## Decision

- **Closed source:** the source repository (`mwahaj36/spawnt`) is private. Only compiled binaries are distributed under a proprietary EULA via a public releases-only repo (`mwahaj36/spawnt-releases`), npm per-platform packages, and OS package managers.
- **Free tier (freeware, forever):** CLI, daemon, local web dashboard, native tray on all 3 OSes, live session state, per-model quota tracking, notifications, and bring-to-front window switcher.
- **Pro (\$19 one-time):** guardrails (hook-based budget/loop auto-stop), Resume Queue (auto-continue at quota reset), custom alert rules, advanced analytics/export, and personal multi-machine sync.
- **Teams (later):** aggregate team dashboard and budget alerts.
- **Single binary + offline license:** one compiled binary ships to everyone; Pro features are unlocked by an offline-verifiable Ed25519-signed license file issued by a Cloudflare Worker upon Lemon Squeezy / Paddle checkout.

## Consequences

- **Privacy trust must be earned explicitly:** because users cannot audit the source code, we enforce strict zero-network defaults, ship `spawnt doctor --network` to list every outbound endpoint, and publish a clear privacy policy.
- **No free OSS code signing or free public-repo CI:** we budget for an Apple Developer membership (\$99/yr) and an individual Windows OV code-signing certificate (~\$100–300/yr) starting in M3, and manage private-repo GitHub Actions minutes carefully (SPEC §7.3).
- **Third-party license compliance:** we bundle `THIRD_PARTY_NOTICES.html` (generated via `cargo-about`) in every release archive and forbid copyleft crates (`GPL`/`LGPL`/`AGPL`) in `deny.toml`.

## Alternatives considered

- **Open source (MIT):** eligible for free OSS signing (SignPath) and unlimited public CI, but exposes all adapter, guardrail, and UI work to immediate cloning and removes binary-level Pro gating.
- **Separate Free and Pro binaries:** doubles build/signing matrix and forces Pro buyers to reinstall after purchase.
- **Paid tray:** competes head-on with a free incumbent on macOS and throttles top-of-funnel adoption.
