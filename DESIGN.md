# DESIGN.md: spawnt design system

Source of truth for the look of the marketing site, the console (web and native) and the installers. Tokens live in `web/app/tokens.css` and are mirrored in the console. Contrast is verified by `node scripts/check-contrast.mjs`. Full rollout plan: the redesign plan in the PR description and `docs/SESSION_LOG.md`.

## 1. Philosophy

Flat, dense, engineered. Daytona-style language: hairline grid cells, uppercase mono labels, square bracketed badges, a floating island navbar. Simple and elegant copy. No "cockpit" or "mission control" wording. The product name is always lowercase: `spawnt`.

## 2. Color

Brand (fixed):

| Token | Hex | Use |
| :--- | :--- | :--- |
| `primary` | `#FF2544` | Brand, primary buttons, active marker, attention, icons |
| `secondary` | `#BECBD8` | Neutral chrome, chart gridlines, links on dark |
| `tertiary` | `#252E99` | Secondary accent, selected rows, chart series 2, links on light |
| `light` | `#F6F7F8` | Light canvas |
| `dark` | `#0C1119` | Dark canvas |

Derived tokens (surface, surface-2, border, border-strong, text, text-2, text-3, primary-text, link, warn) are defined per theme in `tokens.css`. Follow `prefers-color-scheme`; a manual `data-theme` attribute overrides it.

Contrast rules (measured, see script output):

- `primary` as a fill with `dark` text: 5.0:1. Buttons use `--on-primary` (dark) in both themes. White on primary is only 3.75:1, so never use it for small text.
- `primary` as text: use `--primary-text` (`#D4102E` on light, `#FF4D66` on dark). Raw `#FF2544` is graphics, icons and large text only on light (3.5:1).
- `tertiary` on the dark background is 1.7:1. Never use it as text or an icon on dark. On dark it is a fill with white text only.
- Status is never color alone: always a text badge (`[RUN]`, `[WAIT]`, `[IDLE]`, `[LIMIT]`, `[ERR]`). Trust level badges on every number: `EXACT`, `EST`, `API`, `UNKNOWN`.
- `--warn` (amber) is allowed only for quota warnings.

## 3. Shape, borders, depth

- Two radii only: `--radius-sm` 4px (buttons, inputs, badges, chips, tabs, tooltips) and `--radius-md` 6px (cards, panels, tables, drawers, popovers, dialogs, navbar island). Full-bleed regions are 0. No pills; the status dot is the only circle. Nested elements use the smaller radius.
- Borders are always 1px. Panels are cells of a border-collapsed grid with no gaps.
- Shadows only on popovers, dialogs and the scrolled navbar island.
- Optional dot-matrix texture on the page canvas only, at `--dot-alpha` (6% or less), disabled for reduced transparency and print.
- No gradients, no glassmorphism except the navbar island blur.

## 4. Type and spacing

- Inter for UI, JetBrains Mono for numbers, IDs, commands and labels. Both self-hosted; no remote font requests.
- Scale (px / line-height): 12/16, 13/20, 14/20 (body), 16/24, 20/28, 28/36, 40/44, 56/60 (marketing hero only). Weights 400 and 600.
- Labels: uppercase, 11-12px, 600, 0.05em tracking, `--text-3`. Section headers use the `/` separator (`USAGE / THIS WEEK`). Tabular numerals everywhere.
- Spacing scale: 4, 8, 12, 16, 24, 32, 48, 64.

## 5. Motion

`--spring` (`cubic-bezier(0.16, 1, 0.3, 1)`), zero bounce. 0.18s for controls, 0.32s for panels and the navbar. Reduced motion sets durations to 0.

### Floating island navbar

Flush at the top, pops into a floating island on scroll. Toggle `.scrolled` above 45px and remove below 15px (hysteresis, `requestAnimationFrame`, passive listener, also evaluated on mount). Flush: padding 0, full width, radius 0, bottom hairline only. Scrolled: shell padding `12px 1.25rem 0`, island `max-width: 1240px`, radius 6px, full border in `--border-strong`, shadow. Keep a 1px transparent border on all sides to avoid reflow.

## 6. Logo

Source: `brand/spawnt-mark.svg` (solid triangle plus square, `currentColor`).

- In-app (navbar, sidebar, top bar, splash, installer UI): mark in `--text` (white on dark, black on light), directly on the app background, no container.
- Icons and favicons: `#FF2544` mark on a `dark` tile and on a `light` tile (`brand/spawnt-icon-red-on-dark.svg`, `brand/spawnt-icon-red-on-light.svg`). `web/app/icon.svg` switches tile with the browser theme.
- Wordmark: `spawnt`, lowercase, Inter 600, tracking -1%. Lockup is mark, 12px gap, wordmark. Minimum mark size 16px. Do not recolor beyond the values above, stretch, or add effects.

## 7. Copy

Sentence case, plain words, active voice, no emojis, no exclamation marks, verbs on buttons. Tagline: "See what your coding agents are doing." Constants live in `web/lib/brand.ts`.

## 8. Implementation status

- Done: tokens, brand marks, adaptive favicon, brand constants, contrast check.
- Next: console shell and pages, marketing pages, native app, installers, internal rename.
