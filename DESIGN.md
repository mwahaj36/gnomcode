# spawnt design guide

Status: the landing, shared marketing shell, pricing and downloads pages, signed-in web console, daemon-served local console, and current Windows icon assets use the spawnt direction and canonical tokens in `web/app/spawnt-tokens.css`. `web/public/spawnt-mark.svg` is the canonical solid triangle-and-square mark. Native tray layout, docs/auth/settings refinements, and installers remain rollout work. The user-supplied [original redesign plan](docs/spawnt_redesign_plan.md) is preserved in the repo; follow the [current rollout plan](docs/spawnt_rollout_plan.md) for implementation order and [SPEC](docs/SPEC.md) for product behavior and security.

## Brand and visual language

- Display `spawnt` in lowercase. Keep internal `fleethud` identifiers until a separate migration phase.
- Keep the Daytona-inspired structure: near-black canvas, thin borders, dense aligned grids, restrained uppercase mono labels, square badges, and the smoothly detaching marketing navbar. Replace the old green and blue accents.
- Use the solid triangle-and-square mark from `web/public/spawnt-mark.svg` in `currentColor`, without a tile behind it. Red identifies primary actions, active state, and attention. Web and embedded console headers use the same mask asset; generated Windows icon assets and installer marks follow its geometry.
- No emojis, decorative gradients, oversized pills, or color-only status indicators.

## Color tokens

The brand colors are primary red `#FF2544`, pale neutral `#BECBD8`, deep blue secondary accent `#252E99`, light canvas `#F6F7F8`, and near-black dark canvas `#070708`. The dark canvas replaces the attached plan's `#0C1119` at the user's request. `#0C1119` remains the light theme's text color and the text color on red buttons.

These values match the current landing CSS. Derived surfaces, borders, and text colors may be tuned after contrast review.

| Role | Dark | Light |
| :--- | :--- | :--- |
| Canvas `--bg` | `#070708` | `#F6F7F8` |
| Surface `--surface` | `#0D0D0F` | `#FFFFFF` |
| Second surface `--surface-2` | `#141416` | `#EDF0F3` |
| Hairline `--border` | `#242428` | `#D9DFE6` |
| Strong hairline `--border-strong` | `#38383E` | `#BECBD8` |
| Main text `--text` | `#F6F7F8` | `#0C1119` |
| Secondary text `--text-2` | `#A9B4C2` | `#4A5565` |
| Muted text `--text-3` | `#8D99AA` | `#657184` |
| Primary fill `--primary` | `#FF2544` | `#FF2544` |
| Small red text `--primary-text` | `#FF6177` | `#D4102E` |
| Secondary accent `--accent` | `#BECBD8` | `#252E99` |
| Text on red `--on-primary` | `#0C1119` | `#0C1119` |

Contrast ratios calculated from these hex values: primary on dark 5.37:1; primary on light 3.50:1; small red text on light 5.00:1; small red text on dark 6.93:1; dark text on primary 5.05:1. Use `--primary-text` for small red text on light backgrounds. Recheck contrast wherever a surface changes.

Status and trust semantics stay separate from the brand palette. Pair every color with a label: `[RUN]`, `[WAIT]`, `[IDLE]`, `[LIMIT]`, `[ERR]`. Number trust badges are `EXACT`, `EST`, `API`, or `UNKNOWN`. A quota warning may use amber, but amber is not a brand accent. Never imply an estimate is exact.

## Type, shape, and layout

- Inter for headings and body; JetBrains Mono for labels, numbers, IDs, and commands. Self-host fonts in shipped surfaces; make no remote font requests. Use tabular numerals.
- Body type is generally 14/20 or 16/24; section headings 28/36 or 40/44; marketing hero up to 56/60. Prefer weights 400 and 600.
- Spacing scale: 4, 8, 12, 16, 24, 32, 48, 64 px. Controls need at least a 44 px touch target.
- Radius: 4 px for controls and square badges, 6 px for panels and the floating navbar, 0 for grid cells and full-bleed regions. Do not use pill-shaped controls or badges. Borders are 1 px. Shadows belong only on floating layers.
- Console content uses a compact hairline grid: KPIs share borders and tables use row separators. Avoid floating dashboard cards.
- Use geometric SVG icons and self-hosted agent marks in `web/public/agents/`. Labels remain readable without icons.

## Motion and interaction

- Default easing: `cubic-bezier(0.16, 1, 0.3, 1)`; controls about 180 ms, panels about 320 ms. No bounce.
- Marketing navbar is flush at the top and progressively detaches into a 6 px radius island as the page scrolls. Its movement follows scroll position. The console has its own app shell.
- Reveal sections and charts on viewport entry. Chart bars rise in sequence and rows may stagger slightly. Hover effects must not shift surrounding layout.
- The supported-agent strip moves continuously and eases to a pause on hover or keyboard focus. Reduced-motion users get a static list.
- Tour tabs, chart bars, and rows are real controls with clear selected states. Motion cannot be the only way to understand a change.
- Respect `prefers-reduced-motion`: remove nonessential travel and looping motion while preserving visible state changes.

## Navigation, theme, and surfaces

- Marketing header: mark and `spawnt`, Product, Agents, Docs, Pricing, `Log in`, and primary Download. `Log in` goes to `/auth`; the landing's console links also pass through `/auth`, which redirects signed-in users to `/console`.
- Theme selection is shared across all web routes and both consoles. The marketing footer keeps its minimal `Theme / system` control; the signed-in console, auth page, and daemon-served local console expose the same cycle. It cycles System, Light, Dark. System follows `prefers-color-scheme` and is the default. Stored overrides apply before first paint.
- The signed-in web console and local daemon console need the same visual hierarchy but keep distinct data and auth paths. Share presentation code as described in the rollout plan.
- Native tray echoes these tokens and status labels, with the platform fallbacks in SPEC. It is a compact control surface.

## Current implementation boundary

The landing, shared site shell, pricing, downloads and documentation pages, and both consoles use canonical spawnt tokens. Documentation uses the same sans-led hierarchy, restrained mono labels, open rows, hairline dividers, and square controls as the marketing surfaces; it should not resemble a raw Markdown export. The two consoles retain independent rendering/data adapters: Next/React for signed-in snapshots, standalone embedded HTML for local daemon data. The daemon console groups its existing local report, session, quota, guardrail, account, and settings capabilities into matching destinations. Project rollups remain explicitly unavailable locally. Auth/settings refinements, tray scripts, and installers are still migration targets.

## Landing console tour

- Keep overview KPI labels separate from values and trust markers, with shared alignment across all three metrics.
- Use consistent grid tracks for every row in a tour table so agent, state, value, and trust columns remain aligned. On narrow screens, hide the value column uniformly rather than allowing content-driven track widths.

## Command snippets

- Every executable command shown in the app has an adjacent square `Copy` button with an accessible name, visible success/failure feedback, and keyboard focus styling.
- Copy the exact command shown. Keep long commands horizontally scrollable or safely wrapped at narrow widths; the copy control remains visible.
