## 2026-10-28 — pricing route and plan accuracy

- Goal: bring pricing into the shared spawnt system and align plan cadence and Teams availability with SPEC.
- Outcome: removed the unsupported monthly/annual toggle and false $49 Teams price, displayed the $19 one-time Pro license and planned $12/seat/month Teams tier, replaced the legacy cards with a responsive hairline comparison, and removed literal `&check;` strings. Updated the route metadata and rollout/design status.
- Decision: keep the existing download and sign-in destinations; Teams has no active contact or purchase flow, so its planned tier is presented as unavailable.
- Verification: Next.js production build, `git diff --check`, and desktop Chromium review in dark and light passed; the accessibility tree exposes plan sections, links, and native disclosure controls. Narrow viewport and full keyboard pass remain pending.

### Visible brand copy correction

- Replaced user-facing Spawnt product-name copy with spawnt across web metadata, pricing, auth, docs, signed-in console, and the daemon console, including Pro license prompts and exported report filenames.
- Kept the existing domain, installer filenames, CLI command, database/schema names, callback key, and `.spawnt` data paths unchanged for compatibility; those are technical identifiers rather than visible product branding.
- Verification: production web build, `git diff --check`, full Rust quality gate (cargo-deny unavailable), and app-source search passed. Desktop theme screenshots confirmed route surfaces follow Light/Dark; narrow and keyboard review remain pending.

