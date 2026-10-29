# spawnt redesign: Implementation Plan

Audience: an implementing model that needs explicit instructions. Follow phases in order. One branch and one PR per phase (AGENTS.md section 8). Add a dated record under `docs/sessions/` and update its index every PR. Update `docs/SPEC.md` and `DESIGN.md` in the same change as the code they describe.

## 0. Findings from the repo (read first)

| Fact | Consequence |
| :--- | :--- |
| The web console is **one file**, `crates/daemon/src/console.html` (65 KB), embedded in the daemon and served at `/` and `/console` on `127.0.0.1`. | This is the key to "native and web look identical": **there is only one console UI**. The native app must render this same page in a webview. Never build a second console in PowerShell or Tauri UI code. |
| `scripts/tray.ps1` (402 lines) is the current native tray. `apps/tray` (Tauri, SPEC M3) does not exist in the repo. | Build the Tauri tray as a thin shell. Retire `tray.ps1` and the `.vbs` launchers after parity. |
| Marketing site is Next.js 15 + Tailwind 3 in `web/`. The Tailwind config and `globals.css` still use GitHub-dark colors, so the Daytona look lives in per-page styles and `daytona-preview.html`. | Centralise all tokens first (Phase 1). Delete `daytona-preview.html` (root and `web/public/`) at the end of Phase 4. |
| `DESIGN.md` describes the Daytona look: green accent, dot matrix, zero pills, mono uppercase nav. | Rewrite it for spawnt in Phase 1. **Keep** the good parts: 1px hairlines, rectangular geometry, no emojis, plain-text status badges. |
| Existing data API: `/api/v1/sessions`, `report`, `quota`, `stats`, `live`, `alerts`, `export`, `resume-queue`, `pro/relay`, `guardrails/check`, `pricing/models`. | Pages below map to these. Anything missing (per-repo, per-day rollups) must be a **new Rust endpoint**. Business logic and aggregation stay in Rust (invariant 8). The UI only formats. |
| `Desktop\fleethub.svg` is an L-bracket polyline plus a square, no fill. `Desktop\Untitled-13.svg` is unrelated Softec event artwork. | Use `fleethub.svg` as the spawnt mark (assumption, see section 12). It has no color, so recolor via `currentColor`. |

## 1. Brand and naming

- Product name: **spawnt**. Always write it all lowercase: `spawnt`, everywhere (wordmark, nav, titles, copy, installers, CLI), including at the start of a sentence. Never `Spawnt` or `SPAWNT` in display text (env vars like `SPAWNT_HOME` are the only uppercase exception).
- Domain: **do not change any URL yet.** Put the domain in one constant (`web/lib/brand.ts`: `SITE_URL`, `DOWNLOAD_BASE`) and keep the current values. Swapping to `spawnt.xyz` later becomes a one-line change.
- Remove from all copy: "cockpit", "mission control", "fleet", "HUD". Use plain verbs: *see, track, limit, resume*.
- Tagline (use everywhere, one version): **"See what your coding agents are doing."**
- Sub-line: "Live status, cost and limits for your agents. Local-first. Nothing leaves your machine."

### Rename scope: split into two PRs

1. **User-facing rename (Phase 1 to 6):** UI strings, window titles, tray labels, installer names, npm display name, docs, site copy, README.
2. **Internal rename (Phase 7, separate PR, last):** crate names, binary `spawnt` to `spawnt`, env vars, data dir. This is breaking and risky. Requirements:
   - Keep `spawnt` as a thin alias binary for at least one release.
   - Accept both `SPAWNT_HOME` and `SPAWNT_HOME` (new wins), likewise for the other env overrides.
   - Migrate the data dir on first run: if the old dir exists and the new one does not, move it. Add a store/platform test for this. Never delete the old data.
   - Rewrite hook and integration entries that `integrate` wrote earlier (keep the backup-first and `--remove` guarantees).
   - Add an ADR (`docs/adr/0004-rename-to-spawnt.md`) and a row in SPEC section 9.
   - Update the `cut-release` skill scripts that hardcode `spawnt` paths.

## 2. Design tokens (Phase 1)

Palette from the attached image:

| Token | Hex | Role |
| :--- | :--- | :--- |
| `primary` | `#FF2544` | Brand, primary buttons, active nav marker, "needs attention" |
| `secondary` | `#BECBD8` | Neutral chrome: borders, muted fills, chart gridlines, inactive icons |
| `tertiary` | `#252E99` | Secondary accent: links on light, selected rows, chart series 2 |
| `light` | `#F6F7F8` | Light canvas |
| `dark` | `#0C1119` | Dark canvas |

### Contrast rules (computed, mandatory)

- `primary` on `dark`: about 5.0:1. Passes for text and UI.
- `primary` on `light`: about 3.5:1. **Only for large text (24px+), icons, fills and focus rings.** Never for body text or small labels on light.
- `tertiary` on `light`: about 10:1. Good for text and links in light mode.
- `tertiary` on `dark`: about 1.7:1. **Fails. Never use it as text or an icon on dark.** On dark use it only as a fill with white text (white on tertiary is about 11:1).
- Text on a `primary` fill: use `dark` in light mode or white (check ratio, AA 4.5:1 minimum). Verify with a script and record the results in `DESIGN.md`.

### CSS variables (single source: `web/app/tokens.css`, mirrored in `console.html`)

Generate `console.html`'s `:root` block from the same token file with a small build step, or copy it verbatim and add a CI check that diffs the two. Drift is a bug.

```css
:root {                       /* light */
  --bg: #F6F7F8;  --surface: #FFFFFF;  --surface-2: #EDF0F3;
  --border: #D9DFE6;  --border-strong: #BECBD8;
  --text: #0C1119;  --text-2: #4A5565;  --text-3: #6B7686;
  --primary: #FF2544;  --primary-text: #D4102E; /* darker, passes 4.5:1 on light */
  --secondary: #BECBD8;  --tertiary: #252E99;  --link: #252E99;
  --on-primary: #0C1119;  /* white on primary is only 3.75:1 */
}
[data-theme="dark"] {
  --bg: #0C1119;  --surface: #121926;  --surface-2: #19212F;
  --border: #232C3B;  --border-strong: #34405A;
  --text: #F6F7F8;  --text-2: #A9B4C2;  --text-3: #7C8798;
  --primary: #FF2544;  --primary-text: #FF4D66;
  --secondary: #BECBD8;  --tertiary: #252E99;  --link: #BECBD8;
  --on-primary: #0C1119;  /* white on primary is only 3.75:1 */
}
```

Tune the derived grays (`surface`, `border`, `text-2/3`) so every text/background pair meets WCAG AA. Only the five brand hexes are fixed.

### Semantic status colors (not the brand palette)

Status must not rely on red alone, since `primary` is red. Rules:

- Error and "needs you" use `primary`. Running uses `tertiary` fill or `secondary`. Idle uses `text-3`.
- Do **not** add green or amber as new brand colors. Use a single desaturated amber `#B7791F` (dark: `#E0A84A`) **only** for quota warnings, and always pair every status with a text badge: `RUN`, `WAIT`, `IDLE`, `LIMIT`, `ERR`. Color is never the only signal.
- Trust levels (invariant 6) render as badges on every number: `EXACT`, `EST`, `API`, `UNKNOWN`. Use outline style, uppercase, 11px, in `--text-2`.

### Type, spacing, shape

- Font: **Inter** (variable) for UI, **JetBrains Mono** for numbers in tables, IDs and commands. Self-host (no Google Fonts request: invariant 4, no phone-home). Console embeds the font in the binary (`rust-embed`); keep total under 300 KB (subset to Latin).
- Type scale (px / line-height): 12/16, 13/20, 14/20 (body default), 16/24, 20/28, 28/36, 40/44, 56/60 (marketing hero only). Max two weights per screen: 400 and 600.
- Numbers use `font-variant-numeric: tabular-nums` everywhere.
- Spacing scale: 4, 8, 12, 16, 24, 32, 48, 64. No other values.
- **Daytona language is kept everywhere, console included** (only the colors change):
  - Zero pills and **one radius scale used by every page, the console and the native app**: `--radius-sm: 4px` (buttons, inputs, badges, chips, tabs, tooltips), `--radius-md: 6px` (cards, panels, tables, drawers, popovers, dialogs, navbar island, screenshots). Nothing else, no per-component values, no `9999px`, no `50%` except the status dot. Nested elements use the smaller radius. Full-bleed regions (flush navbar, sidebar, strips) are 0.
  - Enforcement: a CI check greps CSS and TSX for `border-radius`, `rounded-` and raw px values and fails on anything but the two tokens (Tailwind `borderRadius` is overridden to only `sm`, `md`, `none`). Same check applies to borders (1px only), shadows (popover, dialog, scrolled navbar only), and spacing (scale values only). The shared `ui/` components are the only place these are defined.
  - Hairline grid: panels are cells of a border-collapsed grid with 1px `--border` lines and no gaps or drop shadows between them. Hover on a cell raises its border to `--border-strong`.
  - Labels: uppercase, 11-12px, 600, 0.05em tracking, `--text-3`. Numbers and IDs in mono. Section headers use the `/` separator style (`USAGE / THIS WEEK`).
  - Status badges: square, bracketed text (`[RUN]`, `[WAIT]`, `[LIMIT]`), mono 11px, 1px border.
  - Subtle dot-matrix texture on the page canvas only (not on cards or tables), masked with a radial gradient, 6% opacity or less, off in `prefers-reduced-transparency` and in print.
  - Expanders use square `[+]` / `[-]` boxes. Motion: stiff `cubic-bezier(0.16, 1, 0.3, 1)`, 0.18s controls, 0.32s panels, zero bounce.
- Shadows only on popovers, modals and the scrolled navbar island. No gradients, no glassmorphism except the navbar island blur.
- Motion: 120ms ease-out for hover and focus, 200ms for panels. Respect `prefers-reduced-motion` (disable everything but opacity). No bounce.
- Theme: follow `prefers-color-scheme`, with a manual toggle (System / Light / Dark) stored in `localStorage` (console) and a cookie-free equivalent on the site. Set `data-theme` before first paint to avoid flash.

## 3. Logo (Phase 1)

Source: `C:\Users\Wahaj\Desktop\fleethub.svg`.

1. Create `brand/spawnt-mark.svg`: viewBox `0 0 2048 2048`, two solid shapes with `fill="currentColor"` and **no stroke**: a right triangle (`points="290.67 1756.53 290.67 291.47 1755.73 291.47"`, the SVG's implicit fill closes the polyline) and a square (`x=799.39 y=789.88 width=957.95 height=957.95`). Your screenshot confirms this look: a solid mark with a diagonal cut and a square at the lower right. Do not treat it as an outlined bracket. Compare the render against your screenshot before moving on.
2. Color rules (confirmed by you):
   - **In-app marks** (navbar, sidebar, console top bar, splash, docs, installer UI): the mark is `light` (`#F6F7F8`, effectively white) on the dark theme and `dark` (`#0C1119`, effectively black) on the light theme, via `color: var(--text)` on `currentColor`. **No container, tile, rounded square or badge behind it.** The mark sits directly on the app background.
   - **Icons and favicons** (browser favicon, app icon, tray, `.ico`, `.icns`, Linux PNGs, apple-touch icon, OG image mark): the mark is `primary` (`#FF2544`). Ship two sets: red mark on a `dark` square background and red mark on a `light` square background (app icon tiles keep their square, since OS icons need a shape). Favicon uses `<link rel="icon" media="(prefers-color-scheme: dark)">` and a light twin so the tab matches the browser theme.
   - macOS tray template image is the exception: black with alpha, as the OS requires; Windows and Linux tray use red.
3. Export all from the one SVG with a script (`scripts/build-icons.mjs`, ImageMagick or `sharp`, dev-only):
   - `web/app/icon.svg`, `web/app/favicon.ico` (16, 32, 48)
   - `web/public/apple-touch-icon.png` (180, mark on `dark`, padded 20%)
   - `scripts/spawnt.ico` (16, 24, 32, 48, 64, 128, 256) for the Windows installer and tray
   - macOS `.icns` and Linux PNG set (16 to 512) for the Tauri bundle
   - Tray icons: monochrome template images (macOS needs a black-with-alpha template, 22px at 1x/2x; Windows 16/20/24 `primary`; Linux 22/24 `primary`). A status dot variant (`RUN`/`WAIT`/`LIMIT`) is drawn as a small badge in the corner.
   - Open Graph image 1200x630: dark canvas, mark, wordmark `spawnt`, tagline.
4. Wordmark: text `spawnt` set in Inter 600, lowercase tracking -1%. Lockup = mark + 12px gap + wordmark, mark height equals cap height x 1.3. Clear space equals the mark's width / 2. Minimum mark size 16px.
5. Update the daemon's `/favicon.ico` and `/spawnt.svg` routes (rename route to `/spawnt.svg`, keep the old path as an alias).
6. Document usage in `DESIGN.md` (do and don't: no recolor beyond `#FF2544`, `#0C1119`, `#F6F7F8`, no stretching, no effects).

## 4. Console information architecture (Phase 2 to 3)

The current console is one overloaded page. Replace it with a **left sidebar app shell** and separate views. The console must be usable at 1100px (native default window) and at 1920px. Below 900px collapse the sidebar into icons only; below 640px use a bottom tab bar (the SPEC supports mobile web access).

### 4.1 App shell

- **Console look:** model it on the Daytona dashboard, not on the old console. Flat, dense, hairline-divided cells, mono uppercase labels, square badges, 1px borders, no card shadows. The page is one border-collapsed grid: sidebar, top bar and every KPI, chart and table cell share hairlines. KPI row = 4 cells in a single bordered strip, not 4 floating cards. Tables have no outer radius beyond 6px and use row hairlines only. Nothing from the old `console.html` layout is reused.
- **Sidebar** (240px expanded, 56px collapsed, right edge is a 1px hairline, nav labels in uppercase 12px with the `/` section separators). **Auto-hide behavior:**
  - Three modes in `Settings / General`: `Auto-hide` (default), `Always expanded`, `Always collapsed`. A pin button at the bottom of the sidebar toggles pinned (expanded) and auto-hide. Mode is saved in `localStorage`.
  - Auto-hide: rests at 56px showing icons only. Hovering or keyboard-focusing anywhere in it expands it to 240px **as an overlay** (content does not reflow, so charts do not jump). Expands after a 120ms hover delay, collapses 300ms after the pointer leaves and focus is outside. Transition 0.18s `cubic-bezier(0.16, 1, 0.3, 1)`.
  - Collapsed state shows a tooltip with the label on icon hover, a 2px `primary` marker on the active item, and a count badge dot for items needing attention (Sessions `WAIT`, Limits near cap).
  - Pinned: content shifts to make room, with the same transition.
  - Under 900px it is always the 56px rail with the overlay on tap; under 640px it becomes a bottom tab bar (5 items plus `More`).
  - Keyboard: `[` toggles pin. Expanded state is announced to screen readers (`aria-expanded` on the toggle); items are always reachable by Tab even when collapsed.
  - Nav groups (each separated by a hairline and a mono label, labels hidden when collapsed): `MONITOR` (Overview, Sessions), `ANALYZE` (Usage, Projects), `CONTROL` (Limits, Automations), `SYSTEM` (Settings). Within a page, secondary views are tabs (for example Usage: `Overview | Agents | Models | Projects | Daily`), so no page holds more than one chart and one table. Top: logo lockup. Sections:
  - **Overview** (home)
  - **Sessions**
  - **Usage**
  - **Limits**
  - **Projects**
  - **Automations** (resume queue, relay, guardrails) with a `PRO` badge where gated
  - **Settings**
  - Bottom: connection indicator (`Live` / `Reconnecting` / `Offline`), account, version.
- **Top bar** (56px): page title, **global filters** (see 4.2), refresh state (`Updated 3s ago`), theme toggle, command palette button (`Ctrl/Cmd+K`).
- Active nav item: 2px `primary` left marker plus `surface-2` background plus `text` color. Inactive: `text-2`. Icons: 16px geometric stroke SVGs (1.5px stroke, `currentColor`). No emojis, no icon font.
- Routing: hash routes (`#/overview`, `#/sessions`, `#/sessions/<id>`, `#/usage`, `#/limits`, `#/projects`, `#/projects/<name>`, `#/automations`, `#/settings`) since the daemon serves a single HTML file. Browser back/forward must work. Deep links must restore filters (filters live in the query string inside the hash: `#/usage?range=7d&repo=api&agent=claude`).
- Keyboard: `g o`, `g s`, `g u`, `g l`, `g p` jump between pages. `/` focuses search. `?` shows a shortcut sheet. `Esc` closes drawers.
- Every interactive element: visible 2px `primary` focus ring with 2px offset, 44px minimum touch target on narrow layouts, proper `button`/`a`/`nav`/`main` semantics, `aria-current="page"`, skip-to-content link, `aria-live="polite"` for status changes.

### 4.2 Global filters (persist across pages)

| Filter | Control | Notes |
| :--- | :--- | :--- |
| **Date range** | Segmented: `Today`, `7 days`, `30 days`, `Custom` | Default `7 days`. Custom opens a two-month popover. Timestamps stored UTC, displayed local. |
| **Repo / project** | Searchable multi-select with counts | Options come from `GET /api/v1/projects`. Show repo name only, full path in tooltip. Paths go through redaction (never show content). |
| **Agent** | Multi-select with agent mark and name | Claude Code, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor |
| **Model** | Multi-select | Optional, collapsed under "More filters" |
| **Trust** | Toggle: `Exact only` | Hides `EST` data, so users can see what is verified |

Rules: active filters appear as removable chips under the top bar with a `Clear all` link. Filter state is in the URL and in `localStorage` (last used). Empty-result states must say which filter caused it and offer to clear it.

### 4.3 Pages (each: purpose, layout, data, states)

#### Overview (`#/overview`), answers "what is happening right now and how am I doing"
Layout, top to bottom (max 5 blocks; resist adding more):
1. **Attention strip** (only when needed): a single row such as "2 sessions waiting for approval, 1 near its limit" with a primary button `Review`. Hidden when empty.
2. **KPI row** (4 cards): `Spend (range)`, `Tokens (range)`, `Active sessions`, `Limit headroom` (lowest remaining across pools). Each card: label, big number (28px), delta vs previous period (`+12%`, with arrow glyph as SVG, not color alone), trust badge, 7-point sparkline.
3. **This week chart** (the "week in token usage" requirement): stacked bar chart, one bar per day for the last 7 days, stacked by agent (or by model, toggle), y-axis tokens (switch to cost with a `Tokens | Cost` segmented control), hover tooltip with per-series values and trust level, today highlighted. Below: a muted line "Week total, daily average, busiest day".
4. **Live sessions** (top 5 by recency) as a compact table with link `View all`.
5. **Top projects** (top 5 by spend, bar inline) with link `View all`.

#### Sessions (`#/sessions`, `#/sessions/<id>`)
- Table columns: Status badge, Agent, Project, Model, Started, Duration, Tokens, Cost (+ trust badge), Context % (thin bar). Sortable headers (`aria-sort`), sticky header, row height 44px, virtualised past 200 rows.
- Status tabs above table with counts: `All`, `Running`, `Waiting`, `Limited`, `Done`.
- Row click opens a **right-hand drawer** (480px; full-screen on narrow) with the session detail: timeline of state changes, token breakdown (input, output, cache read, cache write), cost breakdown, context usage, actions (`Focus window`, `Arm resume`, `Copy session id`). Deep link `#/sessions/<id>` opens the same drawer.
- **Never display prompts, code or tool output** (invariant 1). Show only metadata and redacted activity summaries.
- Empty: "No sessions yet. Start an agent in a terminal and it will appear here." plus the `spawnt doctor` command in a copy block.

#### Usage (`#/usage`), the analytics page
Tabs: `Overview | By agent | By model | By project | Daily`.
- Main chart: tokens or cost over time (area for ranges > 7 days, stacked bar for 7 days), group-by selector (`Agent | Model | Project`), compare-to-previous-period toggle.
- Breakdown table under the chart: group name, tokens in/out/cache, cost, sessions, share bar. Sortable, exportable (`Export CSV`, uses `/api/v1/export`).
- Cache efficiency card (cache read share), cost per session, avg tokens per session.
- **Customizable charts (Overview and Usage):** every chart is a widget the user can change. Defaults: weekly total tokens (all agents combined, bar), tokens by agent (stacked bar), cost per day (area), tokens by project (horizontal bar).
  - Chart menu (top right of each chart, 3-dot icon): `Edit chart`, `Duplicate`, `Export PNG`, `Export CSV`, `Remove`.
  - `Edit chart` opens a side drawer with: **Metric** (tokens in, tokens out, cache read, cache write, total tokens, cost, sessions, duration), **Group by** (none/collective, agent, model, project, day), **Chart type** (bar, stacked bar, line, area, horizontal bar, table), **Range** (inherit global or override: today, 7d, 30d, custom), **Interval** (hour, day, week), **Filters** (agent, model, project), **Compare** (previous period on or off), **Stack/Normalize** (absolute or 100%), **Title**.
  - `Add chart` button opens the same drawer with defaults. Cap at 12 charts per page.
  - Layout: charts live in a 12-column grid, each 4, 6, 8 or 12 wide; drag handle to reorder, resize control with those four sizes. Keyboard alternative to drag: move up/down buttons in the chart menu.
  - Saved per user in `localStorage` (`spawnt.charts.v1`, schema-versioned, validated on load, falls back to defaults if invalid). `Reset to default` in the page menu. Optional export/import as JSON in Settings.
  - All aggregation comes from `GET /api/v1/usage/daily` with `metric`, `group`, `interval`, `from`, `to`, and filters; the widget only stores the query, never computes totals. Each chart footnote shows trust level.
  - Constraints: the same chart config must render identically in the web and native console (it is the same bundle).
- Footnote on every chart: how the number was computed and its trust level.

#### Limits (`#/limits`), quota
- One card per pool (per-agent and per-model pools from SPEC 3.9): pool name, used / limit, progress bar, reset countdown (`Resets in 2h 14m`, absolute time on hover), trust badge, projected exhaustion ("At this rate you run out at 14:20") labeled `EST`.
- Progress bar colors: normal `tertiary`/`secondary`, above 80% amber token, above 95% `primary`. Always show the percentage as text.
- Unknown limits render "Limit unknown. Connect usage API (optional)" rather than a fake 0%.

#### Projects (`#/projects`, `#/projects/<name>`)
- Grid/table of repos: name, agents used (small marks), sessions, tokens, cost, last active. Sort by any column. Search box.
- Project detail: same widgets as Overview, pre-filtered to that repo, plus per-agent split and recent sessions.

#### Automations (`#/automations`), Resume Queue, Context Relay, Guardrails
- Three sections with short explanations, current state and an enable switch each. Anything arming auto-resume must show a plain explanation and never use permission-escalating flags (invariant 7). Pro-gated items show a locked state with an `Upgrade` link rather than being hidden.
- Resume queue: table of armed sessions, target time, `Disarm` action.

#### Settings (`#/settings`)
Sections in a left sub-nav: `General` (theme, startup on login, language), `Notifications` (per event toggles), `Agents` (integration status per agent, `Integrate` / `Remove` buttons that call the CLI behavior with backup-first wording), `Data` (retention, `Export`, `Delete local data`), `Privacy` (a static statement plus the redaction summary, update-check opt-out), `Account & license`, `About` (version, links, diagnostics copy button).

#### Cross-cutting states (every page)
Loading: skeletons matching the final layout (no spinners except on buttons). Empty: one sentence, one action. Error: what happened, what to do, `Retry`. Offline/daemon down: full-width banner "spawnt is not running" with the start command. Stale data: dim and show "Updated X ago". Destructive actions: confirmation dialog naming the thing being changed.

### 4.4 Backend work required (Rust, in `crates/store` and `crates/daemon`)

Add endpoints (all behind the existing bearer token plus Host/Origin checks, each with tests and a doc comment):

| Endpoint | Returns |
| :--- | :--- |
| `GET /api/v1/projects?range=` | Per-repo rollup: sessions, tokens, cost, agents, last active, trust level |
| `GET /api/v1/usage/daily?from=&to=&group=agent\|model\|project&repo=&agent=` | Daily buckets for charts, each value with trust level |
| `GET /api/v1/usage/summary?range=` | KPI numbers plus previous-period delta |
| Filter params on `/sessions`, `/report`, `/export` | `repo`, `agent`, `model`, `from`, `to`, `trust` |
| `GET /api/v1/live` (existing) | Keep as the poll/stream source; add `attention` array (waiting or near-limit sessions) |

Rules: aggregation in SQL via the store crate (wrap SQL with `obfstr!`), week boundaries computed in Rust using UTC and a `tz_offset_minutes` query param so the UI shows local days. Add migration tests if a rollup table or index is introduced. Update SPEC section 3.11 with the endpoint list.

## 5. Console implementation approach (Phase 2 to 3)

- Keep **no build step requirement for the daemon**, but a single 65 KB HTML file will not scale to this scope. Do this instead: create `console/` (Vite + Preact or React + TypeScript, CSS variables, no Tailwind in the console), output a single hashed bundle into `crates/daemon/assets/` and embed with `rust-embed` as SPEC section 3.2 already states. Presentation only. No business logic, no pricing math, no quota math in TS (invariant 8).
- Release build: **no sourcemaps**, minified, no devtools.
- Charts: write small SVG chart components (bar, stacked bar, area, sparkline, progress). Do not add a charting library unless the bundle stays under 250 KB gzipped. Charts need `role="img"` with an `aria-label` summary plus a "View as table" toggle for accessibility. Chart colors: series from `tertiary`, `primary`, `secondary` plus two tints; never rely on color alone (add direct labels or patterns for stacked series).
- Data layer: one `useLive()` hook (poll `/api/v1/live` every 2s, pause when the tab is hidden, backoff on error), one `useQuery(endpoint, filters)` for the rest. All requests send the bearer token injected by the daemon at page-serve time.
- State: URL is the source of truth for filters and the selected session.
- Components to build once and reuse (put in `console/src/ui/`): `AppShell`, `Sidebar`, `TopBar`, `FilterBar`, `Chip`, `Button` (primary / secondary / ghost / danger), `Badge` (status, trust), `Card`, `Kpi`, `Table`, `Tabs`, `Segmented`, `Drawer`, `Dialog`, `Popover`, `Select`, `Switch`, `Tooltip`, `Skeleton`, `EmptyState`, `ErrorState`, `Banner`, `CommandPalette`, `CopyBlock`, charts.
- A `/design` route (dev builds only) renders every component in light and dark for visual review.

## 6. Marketing site (Phase 4)

Stack stays Next.js 15. Replace the Tailwind theme with the token file (`tailwind.config.js` colors mapped to CSS variables). Keep the Daytona look and apply the new palette and name. Drop only the green accent, the glow and the `.html` preview files.

### Pages and sections

- **Nav: keep the Daytona floating-island behavior exactly** (source: `daytona-preview.html` lines 78-200 and the script at 1673-1694; port it into `MarketingNavbar.tsx`). Only the colors change.
  - Structure: `.nav-shell` (fixed, full width, `pointer-events: none`, centers the island) containing `.nav-island` (`pointer-events: auto`, height 60px).
  - **At top (flush):** shell padding `0`, island `width: 100%`, `border-radius: 0`, border is transparent on three sides and a 1px hairline on the bottom only, no shadow.
  - **Scrolled (popped out):** shell padding becomes `12px 1.25rem 0`, island `max-width: 1240px`, `border-radius: 6px`, full 1px border in `--border-strong`, slightly more opaque background, shadow `0 16px 36px -10px rgba(0,0,0,.85)` in dark (lighter in light mode). The glow uses `rgba(255,37,68,.06)` instead of green, or is dropped in light mode.
  - **Pop in and pop out:** the same transition on shell `padding` and island `max-width`, `border-radius`, `border-color`, `box-shadow`, `background-color`, all `0.32s cubic-bezier(0.16, 1, 0.3, 1)`. Keep a 1px border on all four sides (transparent at first) so border width never causes a reflow jump.
  - **Trigger:** `scroll` listener with `requestAnimationFrame` throttling and hysteresis: add `scrolled` above 45px, remove it below 15px. Use `{ passive: true }`. Also run the check once on mount so a reload mid-page starts in the correct state.
  - `prefers-reduced-motion`: keep the state change but set the transition to `0s`.
  - Contents: lockup (mark plus `spawnt`), a `/` separator in `--text-3`, links `Product`, `Pricing`, `Docs`, `Downloads` (12px, 600, uppercase, 0.05em tracking, `--text-2`, hover `--text`, with a 2px `primary` underline on hover and on the active page), right side: theme toggle, `Sign in` (outline button), `Download` (primary button, 4px to 6px radius, no pill). Background is translucent `--bg` with `backdrop-filter: blur(16px)`.
  - Mobile (under 768px): same island behavior, links collapse into a menu button that opens a full-screen sheet with 48px rows.
  - The console (app shell) does **not** use this navbar. It keeps the sidebar and top bar from section 4.
- **Home** (`/`), max 7 sections:
  1. Hero: H1 "See what your coding agents are doing." Sub-line (section 1). Two buttons: `Download` (primary, detects OS and shows e.g. `Download for Windows`), `View docs` (secondary). Under it, one install line in a copy block (`npm i -g spawnt` placeholder until the package exists; confirm before publishing) and the line "Free for individuals. Local-first."
  2. Product shot: a **real screenshot** (or live-rendered replica) of the console Overview in the user's theme. Generate it from the actual console build so it never drifts.
  3. Supported agents: a quiet row of six names with marks (monochrome, `text-2`).
  4. Three feature blocks, one per promise: **See it live** (sessions and status), **Know the cost** (tokens and spend, with trust levels), **Stay under limits** (quota and resume). Each: 1 headline, 2 lines of copy, 1 small visual.
  5. Privacy band: "Your logs never leave your machine." Four facts: reads local logs only, stores metadata only, no telemetry, loopback only.
  6. Pricing teaser (Free / Pro / Team cards, link to full page).
  7. Final CTA plus footer.
- **Pricing** (`/pricing`): three plans in cards, monthly/yearly toggle, comparison table below, FAQ accordion (native `details`). Use the actual tiers from SPEC 1.5. Do not invent prices; read them from SPEC or ask.
- **Downloads** (`/downloads`): OS-detected primary card, then all platforms (Windows installer, macOS, Linux, npm, script install), checksums, version and release notes link. Keep the existing download URLs unchanged.
- **Docs** (`/docs`): left nav, search, content column max 720px. Sections: Quick start, Agents, Console guide (one subsection per page in 4.3), CLI reference, Privacy and security, Troubleshooting.
- **Auth** (`/auth`), **Settings** (`/settings`), **Console** (`/console`, cloud multi-device view): reuse the console components so signed-in cloud views match the local console.
- Footer: 4 columns (Product, Resources, Company, Legal), small print, theme toggle. No fake compliance seals.
- SEO and meta: titles `Page | spawnt`, OG image, `sitemap.ts` and `robots.ts` updated, JSON-LD `SoftwareApplication`. Keep canonical URLs on the current domain until the new one is bought (via `SITE_URL`).
- Performance targets: Lighthouse 95+ on all four, LCP under 2s, no layout shift from fonts (`font-display: swap` with metric-matched fallback), images with width/height, hero text is HTML not an image.
- Accessibility: AA contrast, keyboard-complete nav and accordion, `lang`, landmarks, reduced motion.

### Copy rules (for every string in site, console, installer, tray, CLI)

- Sentence case. Plain words. Short sentences. Active voice. No jargon like "telemetry stream", "orchestration", "cockpit", "mission control".
- No emojis (invariant 9). No exclamation marks. No "revolutionary / seamless / powerful".
- Buttons are verbs: `Download`, `Retry`, `Clear filters`. Errors say what happened and what to do.
- Numbers always carry units and a trust badge in the console.
- Keep a copy deck at `docs/COPY.md` (single place for taglines, feature blurbs, error texts). The site, console and installer read from it by convention (not code).

## 6.1 Layout, grid and copy spec (follow literally)

### Global grid
- Marketing: 12 columns, 24px gutters, content max width 1200px, side padding 24px (16px under 640px). Section vertical padding 96px desktop, 64px tablet, 48px mobile. Body text column max 65 characters.
- Console: sidebar 240px plus content. Content max width 1440px, padding 24px, 12-column grid with 16px gaps. Cards snap to 3, 4, 6 or 12 columns only. Collapse rules: 12 to 6 under 1100px, to 12 (stacked) under 700px.
- Screen usage in the console: put at most one primary chart and four KPIs above the fold at 1180x760. Everything else moves to its own tab or page. Tables get the full width. Detail goes in drawers, not in the page.

### Console: placement per page (12-column grid)
| Page | Row 1 | Row 2 | Row 3 |
| :--- | :--- | :--- | :--- |
| Overview | Attention strip (12, conditional) | 4 KPI cards (3 each) | Week chart (8) + Limits summary (4); then Live sessions (7) + Top projects (5) |
| Sessions | Status tabs (12) | Table (12) | Drawer on the right (480px overlay) |
| Usage | Group-by and compare controls (12) | Main chart (12) | Breakdown table (8) + Cache efficiency and averages (4) |
| Limits | Pool cards, 3 per row (4 each) | Reset timeline (12) | none |
| Projects | Search and sort (12) | Table (12) | none |
| Automations | Three stacked sections (12) | none | none |
| Settings | Sub-nav (3) + form column (9, max 640px) | none | none |

The uncluttering rule: each page has one job. Move to a new tab anything that needs more than one chart plus one table. Overview shows summaries with `View all` links only. Raw tables live in Sessions, Projects and Usage.

### Console copy (exact strings)
- Page titles and subtitles: Overview "Overview" / "What your agents are doing now." Sessions "Sessions" / "Every agent run, live and past." Usage "Usage" / "Tokens and spend over time." Limits "Limits" / "How much you have left, and when it resets." Projects "Projects" / "Usage by repo." Automations "Automations" / "Resume work after a limit resets." Settings "Settings" / "Preferences, agents and data."
- KPI labels: `Spend`, `Tokens`, `Active sessions`, `Headroom`. Chart title: "This week". Chart toggles: `Tokens`, `Cost`, `By agent`, `By model`.
- Statuses: `RUN` Running, `WAIT` Needs you, `IDLE` Idle, `LIMIT` Limit reached, `ERR` Error.
- Empty states: Sessions "No sessions yet. Start an agent in a terminal and it will show up here." Projects "No projects yet. Repos appear after the first session." Filtered "No results for these filters." with `Clear filters`. Limits unknown "Limit unknown for this plan."
- Errors: "Can't reach spawnt. Start it with `spawnt daemon`." with `Retry`.
- Privacy line (Settings and footer): "spawnt reads local logs and stores metadata only. Prompts, code and output are never saved or sent."

### Marketing: section by section
1. **Hero** (12 columns, text left 6, product shot right 6 on desktop; stacked on mobile). Eyebrow "Free for individuals" (12px uppercase). H1 "See what your coding agents are doing." (56px, 600, max 12 words). Sub (18px, `--text-2`): "Live status, cost and limits for your agents. Local-first." Buttons `Download for [OS]` and `View docs`. Below, small `--text-3`: "Windows, macOS and Linux."
2. **Agent marquee** (12, full bleed, directly under the hero): label "Works with" in the 12px uppercase mono style, then a continuously scrolling strip of agent logos with names (Claude Code, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor, plus any added later). Implementation: the list rendered twice inside one flex track, CSS-only `translateX(0)` to `translateX(-50%)` keyframes, linear, 40s loop, hairline top and bottom borders, 24px edge fade masks (`mask-image` linear gradient) so logos fade in and out. Logos are monochrome (`--text-3`) and turn `primary` red on hover and focus (graphic only, so the 3.5:1 on light is fine), with a 0.18s color transition, 24px tall, 48px gap. Pauses on hover and focus. With `prefers-reduced-motion` it stops and shows a static wrapped row. The duplicate copy is `aria-hidden`, and the real list is available to screen readers as a normal list. Use official marks only if licensing allows, otherwise plain wordmarks; keep SVGs in `web/public/agents/`.
3. **Features** (3 equal columns, 4 each; stacked on mobile). Headlines and copy:
   - "See it live" / "Every session, its status and which ones need you, in one list."
   - "Know the cost" / "Tokens and spend per agent, model and repo. Every number says how sure we are."
   - "Stay under limits" / "Track what's left in each plan and pick up where you stopped when it resets."
4. **Console tour** (12, tabbed): tabs `Overview`, `Sessions`, `Usage`, `Limits`, each showing one real screenshot with a one-sentence caption. This replaces the cluttered feature grid.
5. **Privacy band** (4 columns): "Local only" / "Reads logs on your machine." "Metadata only" / "No prompts, code or output stored." "No telemetry" / "Nothing phones home." "Loopback" / "Only your machine can reach it."
6. **Pricing teaser** (3 cards, 4 each): plan name, one-line promise, three bullets, button. Prices from SPEC 1.5.
7. **Final CTA** (centered, 8 columns): "Start in a minute." / "Download spawnt and open your first session." Button `Download`.
8. **Footer** (12 columns: brand 4, then three link columns of 2.67): privacy line plus copyright.
- Pricing page: header (H1 "Pricing", sub "Start free. Upgrade when you need more."), 3 cards (4 each), comparison table (12), FAQ (8 centered, 6 to 8 questions in `details`).
- Downloads page: left 7 columns primary card (OS-detected, version, size, checksum, `Download`), right 5 columns "Other ways to install" list (npm, script, other OS).
- Docs page: sidebar 3, article 6, on-this-page 3 (hidden under 1100px).

## 6.2 Every page is redesigned, not recolored

Rule: no page keeps its old layout or markup with new colors. Each page below is rebuilt from the Daytona patterns (hairline grid cells, mono uppercase labels, square badges, island navbar, `/` separators) using shared components. Page inventory in `web/app/`: `page.tsx` (landing), `pricing`, `downloads`, `docs`, `auth`, `settings`, `console`, plus `layout.tsx`, `globals.css`, `icon.svg`, `favicon.ico`, `sitemap.ts`, `robots.ts`. Plus the daemon's local console and error/404 pages. All must be done before Phase 4 closes; the PR checklist lists each one.

- **Auth (`/auth`)**: full-height two-column hairline layout. Left (7 cols, hidden under 900px): mark, one line "Sign in to sync your usage across devices.", three short privacy facts in bordered cells. Right (5 cols): form cell with label `SIGN IN`, email field, `Continue` primary button, divider `OR`, OAuth buttons (outline, equal width), small print linking Terms and Privacy. Tabs `Sign in | Create account`. Inline validation under fields, button shows loading text not a spinner overlay, error banner above the form. Success: "Check your email." state. Local-only users see a note: "You don't need an account to use spawnt locally." with a link back. Handle the `/auth/callback` state with a minimal branded loading screen and a failure state with `Try again`.
- **Settings (`/settings`, web account area)**: left sub-nav (3 cols) and content (9 cols, max 640px), same structure as the console Settings. Sections: `Profile`, `Devices` (table of linked machines with last seen and `Remove`), `Plan and billing` (current plan cell, usage cell, `Manage billing`), `Team` (members table, invite, roles; shown only on Team), `Notifications`, `Data` (export, delete account, with typed-confirmation dialog), `Security`. Each section is a hairline-bordered form cell with a header, helper text and a single save button; unsaved changes show a sticky save bar.
- **Console (`/console`, cloud view)**: renders the same app shell and pages as the local console (section 4) from synced data, with a `Device` filter added to the global filters (All devices or one machine). Not-signed-in and no-devices states each get an empty state with the next step. Shares the component library; no separate implementation.
- **Landing, Pricing, Downloads, Docs**: per sections 6 and 6.1.
- **404 and error pages**: centered mark, mono label `404 / NOT FOUND`, one sentence, `Go home` button. The same pattern serves `500`.
- **Legal pages (Terms, Privacy)**: add if missing (docs-layout, text column 720px).
- **Footer and navbar**: shared by all marketing and auth pages; the island navbar from section 6 applies to every page except the console shell.
- **Emails and notifications** (auth emails, update notices): plain text first, dark text on light, red `Confirm` link, lowercase `spawnt` sign-off.

Acceptance for 6.2: open each route in dark and light at 1440, 1024 and 390px wide with no horizontal scroll, no old-theme remnants, keyboard navigable, and a screenshot of each attached to the Phase 4 PR.

## 7. Native app (Phase 5)

Goal: pixel-identical console on Windows, macOS and Linux, by construction.

1. Create `apps/tray` (Tauri v2, follow the `tauri-v2` skill). **The main window is a webview that loads `http://127.0.0.1:<port>/` from the running daemon.** It does not ship its own copy of the UI. Result: web console and native console are the same bytes.
   - On launch, start the daemon if not running (reuse the daemon lock), wait for `/health`, then load the URL. Show a minimal branded splash (dark canvas, mark) while waiting; show an error screen with a `Retry` button and logs location after 10s.
   - Auth: the daemon injects the token on serve. The webview needs no stored token.
   - Window: 1180x760 default, min 900x600, remember size and position, native title bar (do not draw a custom one on Linux). Mica/acrylic is **not** used: the UI is opaque so rendering matches everywhere.
   - Same URL opened in the browser must look the same. Differences are limited to the window chrome.
2. **Tray popover** (small, the quick view): 340x440 webview loading `http://127.0.0.1:<port>/#/tray`, a compact route in the same console bundle (header with lockup, attention list, up to 5 live sessions with status badges, spend today, lowest limit headroom, buttons `Open spawnt`, `Settings`, `Quit`). Same tokens and components, so no divergence. Replaces the tray menu text list. Provide a plain native menu as a fallback for Linux desktops without a webview popover.
3. Tray icon: states `idle`, `running`, `waiting`, `limited` (badge dot on the mark). macOS uses the template image. Tooltip: "spawnt: 2 running, 1 waiting".
4. Notifications: native, text only, title `spawnt`, body one line. Click opens the relevant session (`#/sessions/<id>`).
5. Autostart (via `crates/platform`), single instance, `Open` focuses the existing window.
6. Release hardening (SPEC 6.8): `devtools` disabled, CSP restricted to loopback, no remote content, strings wrapped with `obfstr!` where sensitive.
7. Retire `scripts/tray.ps1`, `open-console.vbs`, `open-dashboard.vbs`, `run-tray.vbs` only after the Tauri app reaches parity. Remove the references from installers.
8. **Parity test (the user's explicit requirement):** a Playwright script (dev tool) loads the console URL in Chromium and screenshots every route in light and dark at 1180x760 and 1920x1080; the Tauri app is screenshotted via its webview (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux) at the same size. Compare with a small pixel-diff threshold and attach the diffs to the PR. Font rendering differences under 1% are acceptable; layout differences are bugs. Self-hosted fonts are mandatory so system fonts never differ.

## 8. Installers (Phase 6)

- **Windows** (`scripts/Setup.cs`, `Install-Spawnt.bat`, `install.ps1`): rename to `spawnt-Setup`, new icon, product name, window text, install dir `spawnt`, Start Menu shortcut `spawnt`, uninstaller entry. Installer UI uses the palette: dark canvas, mark, one primary button. Keep the old uninstall entry detectable so upgrades from Spawnt replace the old install and keep data.
- **macOS**: `.dmg` background in `dark` with the mark and an arrow to Applications; `.icns`. Note the un-notarized flow (SPEC 7.4) in the first-run help text.
- **Linux**: `.deb`/`.AppImage` with desktop file (`Name=spawnt`, `Icon=spawnt`), icon set 16 to 512.
- `install.sh` / `install.ps1` / `npm/` shims: update printed text, name and ASCII banner (no emojis, no box art), keep URLs as they are today.
- Installer copy: "Installing spawnt", "Done. spawnt is running in your tray." Nothing else.

## 9. Phases, deliverables and acceptance

| Phase | PR | Done when |
| :--- | :--- | :--- |
| 1 | `feat(design): spawnt tokens, logo and DESIGN.md` | `tokens.css`, brand SVG and generated icons exist, `DESIGN.md` rewritten, contrast table recorded, `brand.ts` with unchanged URLs |
| 2 | `feat(daemon): usage, projects and filter endpoints` | New endpoints plus tests, SPEC 3.11 updated |
| 3 | `feat(console): new shell and pages` | All 7 pages from 4.3 work in light and dark with all states, filters, deep links, keyboard nav, zero axe-core violations, bundle embedded in daemon, `console.html` removed |
| 4 | `feat(web): new marketing site and copy` | All pages in section 6 redesigned, Daytona assets deleted, `docs/COPY.md` added, Lighthouse targets met |
| 5 | `feat(tray): tauri app loads console` | Parity test passes on all three OSes (CI matrix), `tray.ps1` and `.vbs` removed |
| 6 | `feat(installers): spawnt branding` | Installers build on all three platforms, upgrade from Spawnt keeps data |
| 7 | `refactor: rename internals to spawnt` | ADR 0004 merged, aliases and migration tested, `AGENTS.md` and `README.md` updated |

For each PR run the `rust-quality-gate` skill, and `cross-platform-review` for Phases 5 to 7. Use `impeccable` (critique, polish, audit) on Phases 3 and 4 before opening the PR. Use `verification-before-completion`: do not claim done without command output.

## 10. Definition of done (global)

- No remaining user-facing "Spawnt", "cockpit", "mission control" strings (grep check in CI, allow-list only for the migration code and the changelog).
- No emojis. No gradients. No leftover Daytona branding or green (`daytona`, `#2ecc71`, `#4ade80`) anywhere: grep check. The Daytona visual language (hairline grid, square badges, mono labels, island navbar) stays.
- Every number shows a trust badge. Every status has text, not just color.
- Light and dark both pass AA. Keyboard-only use covers every action. `prefers-reduced-motion` honored.
- Console in the browser and in the native app produce matching screenshots for every route.
- `cargo fmt`, `clippy -D warnings`, `cargo test --workspace`, `cargo deny check` pass. SPEC, DESIGN, SESSION_LOG updated.

## 11. Things the implementer must not do

- Do not copy the old console layout into the new colors.
- Do not add a second UI for the native app.
- Do not put pricing, quota or aggregation logic in TypeScript.
- Do not load remote fonts, scripts or analytics.
- Do not change any URL or download path until the domain is bought.
- Do not delete old comments or docs unrelated to the change.
- Do not use red-only status signaling, since `primary` is red.

## 12. Decisions and open items

Resolved:
1. Logo file is `fleethub.svg` (solid triangle plus square). Confirmed by screenshot.
2. In-app mark is white on dark and black on light with no container. Icons and favicons are red (`#FF2544`) on black and on white. See section 3.
3. Small red text on light uses `primary-text` (`#D4102E`). Approved.
5. Pricing comes from SPEC 1.5 and is current. Approved.

Still open:
4. **Package and CLI name:** check that `spawnt` is free on npm and crates.io before Phase 7.
