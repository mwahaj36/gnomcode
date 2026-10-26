# 0004 — Brand Redesign and Rename to spawnt

- **Status:** Accepted
- **Date:** 2026-10-27

## Context

FleetHUD used generic SaaS dashboard conventions and heavy space/flight metaphors ("cockpit", "mission control", "flight recorder") that obscured its primary purpose for high-frequency software engineers. The brand name had uppercase variants and lacked a crisp, modern visual identity.

We need a design language that presents dense, high-frequency agent telemetry cleanly without visual noise, matches modern developer environments, and establishes a memorable product identity.

## Decision

1. **Brand Name:** Rename to `spawnt` (always lowercase in all UI, CLI, docs, and installers).
2. **Domain:** `spawnt.xyz` (URLs in `web/lib/brand.ts` remain decoupled until final domain purchase).
3. **Design System:** Adopt the Daytona visual language adapted to the user's custom color palette:
   - Primary Red: `#FF2544` (CTA, active agent states, hover transitions)
   - Secondary Slate: `#BECBD8`
   - Tertiary Deep Indigo: `#252E99` (fill accents only)
   - Canvas Light: `#F6F7F8`
   - Canvas Dark: `#0C1119`
4. **Geometry & Typography:**
   - 1px hairline border grid cells
   - Two strict border radii workspace-wide: `--radius-sm: 4px` (buttons, badges, inputs) and `--radius-md: 6px` (panels, cards, dialogs). No pills (`9999px` forbidden).
   - Uppercase monospace tags with `/` delimiters (`AGENT //`, `SESSION //`).
   - Square-bracketed status badges (`[RUN]`, `[WAIT]`, `[IDLE]`, `[LIMIT]`, `[ERR]`).
   - Square-bracketed trust ratings on every numeric metric (`[EXACT]`, `[EST]`, `[API]`, `[UNKNOWN]`).
5. **Mark & Favicon:**
   - In-app mark: `currentColor` vector mark directly on surface with zero container or background tile.
   - Favicons and app icons: Primary red `#FF2544` mark on dark and light square tiles.
6. **Shell Architecture:**
   - Floating island navbar with hysteresis scroll physics (`scrollY > 45px` popped, `< 15px` flush, `0.32s cubic-bezier(0.16, 1, 0.3, 1)`).
   - Full-bleed agent marquee strip with logo hover transitions to primary red `#FF2544`.
   - Auto-hiding 56px/240px nav rail in console with 4 groups (`MONITOR`, `ANALYZE`, `CONTROL`, `SYSTEM`).
   - 100% pixel parity between browser web console and native tray app window via embedded `console.html`.
7. **Environment Variable:** Support `SPAWNT_HOME` with fallback to `FLEETHUD_HOME`.

## Consequences

- Full visual coherence across landing, docs, pricing, auth, settings, console, and desktop tray.
- Single embedded console HTML guarantees desktop app and web console never drift.
- Backward compatibility preserved: existing `FLEETHUD_HOME` directories and scripts continue working seamlessly.

## Alternatives considered

- Retaining "FleetHUD" with a simple reskin — rejected because space metaphors confused prospective users.
- Custom pill-heavy UI — rejected in favor of Daytona's crisp rectangular geometry.
