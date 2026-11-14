# Gnomcode Console Redesign Specification

Date: 2026-11-14
Author: Hydrogen (Lead Product Designer & Frontend Engineer)
Status: **Proposed — awaiting owner approval of mockup before implementation**
Mockup: [`docs/design/console-redesign-mockup.html`](console-redesign-mockup.html)

---

## 1. Design Thesis

Calm, precision mission control for local AI coding agents. One unified sidebar, one centered `72ch` reading column, one right inspector when needed, and one quiet accent color—so the developer's conversation, diffs, and usage limits are effortless to scan at a glance.

---

## 2. Design Tokens (`web/app/gnomcode-tokens.css`)

All hard-coded colors and off-scale font sizes in `gnomcode-console.css` are replaced by these semantic tokens.

### 2.1 Color Palette (Dark & Light — Canonical Gnomcode Palette)

- **Canonical surfaces & text:** Preserves Gnomcode's exact neutral ramp (`#070708` / `#0d0d0f` / `#141416` dark; `#f6f7f8` / `#ffffff` / `#edf0f3` light) from `web/app/gnomcode-tokens.css` and `DESIGN.md`.
- **One accent color:** `--sp-accent` (`#becbd8` dark / `#252e99` light) for active nav indicators, focus rings, and interactive emphasis; primary CTAs (`Send`, `New`) use crisp high-contrast `--sp-cta-bg` (`#f6f7f8` with `#0c1119` text in dark; `#252e99` with `#ffffff` text in light).
- **Red strictly for danger (and owner-approved activity heatmap):** `--sp-danger` (`#ff4d5e` dark / `#d4102e` light) is used **only** for destructive actions, errors, and exhausted quota states—never for ordinary primary buttons (`Send`, `New`, `Save`).

| Token | Dark | Light | Contrast / Role |
| :--- | :--- | :--- | :--- |
| `--sp-bg` | `#070708` | `#f6f7f8` | App canvas |
| `--sp-surface` | `#0d0d0f` | `#ffffff` | Sidebar, cards, inspector, composer |
| `--sp-surface-raised` | `#141416` | `#edf0f3` | Hover rows, subtle fills, agent bubbles |
| `--sp-surface-overlay` | `#18181b` | `#ffffff` | Popovers, menus, command palette, dialogs |
| `--sp-border` | `#242428` | `#d9dfe6` | Hairline dividers |
| `--sp-border-strong` | `#38383e` | `#becbd8` | Focused inputs, active card edges |
| `--sp-text` | `#f6f7f8` | `#0c1119` | Primary text (>15:1 on surface) |
| `--sp-text-2` | `#a9b4c2` | `#4a5565` | Secondary text (>7:1 on surface) |
| `--sp-text-3` | `#8d99aa` | `#525e70` | Tertiary / meta text (6.4:1 dark, 5.7:1 light on raised) |
| `--sp-accent` | `#becbd8` | `#252e99` | Active focus ring, nav emphasis |
| `--sp-cta-bg` | `#f6f7f8` | `#252e99` | Primary CTA (`Send`, `New`) |
| `--sp-cta-text` | `#0c1119` | `#ffffff` | Text on `--sp-cta-bg` (>11:1) |
| `--sp-accent-soft` | `rgb(190 203 216 / .12)` | `rgb(37 46 153 / .10)` | Active nav tint, subtle selection |
| `--sp-ok` | `#3fcf8e` | `#12955c` | Healthy quota, running dot |
| `--sp-warn` | `#f5a524` | `#b86e00` | Low / pace warning, awaiting approval |
| `--sp-danger` | `#ff4d5e` | `#d4102e` | Exhausted quota, errors, destructive actions |
| `--sp-unknown` | `#5b6472` | `#8a94a3` | Inactive / unknown state |

### 2.2 Spacing Scale (4px base)

`--sp-1: 4px`, `--sp-2: 8px`, `--sp-3: 12px`, `--sp-4: 16px`, `--sp-5: 24px`, `--sp-6: 32px`, `--sp-7: 48px`, `--sp-8: 64px`.

### 2.3 Typography Scale (6 steps)

System sans (`-apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`) for UI and prose; `JetBrains Mono` for code, diffs, and CLI commands. Tabular numerals (`font-variant-numeric: tabular-nums`) on all numbers.

- `--sp-fs-meta: 12px` (timestamps, badges, axis labels; heatmap day labels `10px` is the sole documented exception)
- `--sp-fs-control: 13px` (buttons, tabs, sidebar items, form labels)
- `--sp-fs-body: 14px` (default body, session titles)
- `--sp-fs-strong: 15px` (section headings, conversation prose)
- `--sp-fs-title: 20px` (page titles)
- `--sp-fs-number: 28px` (KPI stat tiles)

### 2.4 Radius, Elevation & Motion

- **Radius:** `--sp-r-code: 4px`, `--sp-r-control: 6px`, `--sp-r-popover: 8px`, `--sp-r-card: 12px`, `--sp-r-bubble: 14px`, `--sp-r-pill: 999px`.
- **Elevation:** Flat hairlines at rest; `--sp-shadow-popover: 0 12px 32px rgb(0 0 0 / .36)` (dark) / `0 8px 24px rgb(15 23 42 / .10)` (light) for floating popovers and command palette.
- **Motion:** `150ms cubic-bezier(.2, .8, .2, 1)` for state/hover/drawer transitions; completely disabled under `prefers-reduced-motion: reduce`.

---

## 3. Layout Model & Key Decisions

### 3.1 Unified Collapsible Left Sidebar + Mobile Bottom Tab Bar

**Decision (Owner preference):** Integrate navigation and Conversations into **one unified, collapsible `268px` left sidebar** (`Ctrl+\` / `Cmd+\`) so no horizontal space is wasted on a separate 64px icon rail.
- **Top of sidebar:** Brand header + collapse toggle, followed by a compact 4-item nav strip (`Home`, `Chats`, `Usage`, `Settings`).
- **Integrated Conversations:** `Search sessions` + `New`, a smoothly scrolling list grouped `Needs you` / `Running` / `Recent`, and an `All chats (N) ›` footer link.
- **Subagents & expandable trees (never 40 rows inline):** Subagents are **collapsed by default** behind a single compact disclosure row under their parent (e.g., `▸ 1 running · 37 subagents`). Expanding inline caps at **5 rows** (prioritizing running/waiting first, then most recent), with a `View N more in All chats ›` link when more exist.
- **Dedicated `All chats` pane:** Accessible from `All chats ›` in the sidebar footer, `View all ›` in Home `Sessions`, `View N more in All chats ›` in subagent disclosures, or `Cmd/Ctrl+K`. Provides full-width search, filter chips (`All`, `Running`, `Needs approval`, `Claude Code`, `Codex`, `Antigravity`), an `Include subagents` toggle, and a clean scannable list.
- **Mobile (`< 768px`):** The 4-item nav moves to a **bottom tab bar** in the thumb zone (with `env(safe-area-inset-bottom)`), keeping the top of the screen uncluttered for the Workspace header; Conversations opens as a sheet/drawer with 44px touch targets.

### 3.2 Workspace: Single-Scroll Transcript + Unified Right Inspector

**Decision:** Use a **Right Inspector** (`380px` desktop, slide-over sheet on phone) with a single 4-segment control (`Files | Changes | Artifacts | Terminal`).
- **Why Right Inspector over replacing the chat pane:** Developers inspect diffs, file contents, or resolve CLI trust prompts in Terminal *alongside* the conversation and composer. One unified inspector replaces both the old `Files / Changes / Artifacts` button row and the separate `Terminal` text link.
- **Workspace Header (1 compact 52px bar):** Sidebar toggle (when collapsed) + single-line session title + agent/status meta (`Claude Code · Running`) on the left; **one compact IDE-style Quota trigger** (e.g. ring gauge + `13% left` in urgent amber/red when low) + `Inspector` toggle + `Actions` menu (`Schedule & automation`, `Handoff / Reset`, `Worktree`, `Focus window`) on the right.
- **Transcript:** Fills the main pane with **one page-level scroll** (no scroll-inside-scroll). Prose is centered at `~72ch` (`max-width: 72ch; margin-inline: auto`), while fenced code blocks and tables may expand to `min(100%, 88ch)` for readability.

### 3.3 Composer, Inline Model Menu & Usage Popover

**Decision:** Dock a compact, auto-growing composer at the bottom of the Workspace pane (above safe-area/virtual keyboard on mobile):
- **Quick prompts (`Continue`, `Run tests`, `Summarize`, `Customize`):** Single-line, horizontally scrollable strip (`flex-wrap: nowrap; overflow-x: auto; scrollbar-width: none;`) with generous `12px` horizontal padding and `32px` height (44px touch area on mobile) tucked directly above the composer border—never wrapping an orphan chip onto a second row.
- **Inline error banner:** Delivery errors (`Could not send...`) render **directly inside the top of the composer** with `Retry` and `Open terminal` buttons—never stranded below collapsed disclosures.
- **Bottom toolbar inside composer:**
  - Left: Quiet inline **Provider & Model picker** (`Gemini 4 Hydrogen Medium ⌄`) listing **all switchable providers** (`Antigravity`, `Claude Code`, `Codex`) with clear per-model quota badges (`Out` in red, `13%` in amber when close, `97%`/`100%` in green when healthy). Clicking **`View Usage ›`** inside the model menu (or clicking the header **Quota pill**) opens the **Usage popover showing all providers** and their pools, remaining %, reset countdowns, and ring/meters.
  - Right: Paperclip `Attach` icon button (44px hit target) + crisp accent `Send` button (`--sp-accent`, never red).

### 3.4 Home & Multi-Pool Provider Cards

1. **Full-width asymmetric Desktop Home grid (Owner decision + macOS HIG density):**
   - Instead of a narrow `1200px` centered stack with wasted side margins, Home uses the full available width (with `32px` desktop gutters).
   - **Top row (full width):** `Limits` (3 provider cards across).
   - **Below Limits (`>= 1024px`, `2fr / 1fr` split):**
     - **Left 2/3:** `Activity` (full 53-week brand-red grid + stats + legend, filling the 2/3 column cleanly) followed by `Sessions` (`Needs you` / `Running` / `Recent`).
     - **Right 1/3:** `Today` (`Tokens today`, `Est. spend today`, `Active sessions` tiles) + pending `Approvals` when active.
   - **Compact / Phone (`< 1024px`):** Stacks to 1 column in order: `Limits -> Activity -> Today -> Sessions`.
   - **Result:** Limits, Activity, Today, and Sessions all sit above the fold on `1440x900`.
2. **Activity heatmap ramp (Owner decision):** Use Gnomcode brand red (`#ff2544` scale) for the 4-level activity heat ramp on both Home and Usage (`--sp-heat-1` to `--sp-heat-4`), clearly distinct from `--sp-danger` quota alerts via stepped saturation/luminance.
3. **Multi-pool Provider Card State (Rust API proposal for `SPEC.md`):**
   - Currently `ProviderOverview.state` takes the worst window across all pools, causing Antigravity to show red `Out` when 1 of 3 pools (`Gemini 4 Argon`) is exhausted while `Claude and GPT models` is at 97%.
   - **Proposed Rust rule (requires SPEC §3.9 update before implementation):** `ProviderOverview.state` is `Exhausted` (`Out`) **only when every window/pool is exhausted**; if at least one pool has remaining capacity while another is exhausted or low, `ProviderOverview.state` is `Low` (amber) and `headline_window` / summary text reads `Gemini Models 13% · 1 pool out` (never truncated `ba...`).

---

## 4. Component Inventory (One Component Each)

| Primitive | Purpose | Where Used |
| :--- | :--- | :--- |
| `AppShell` | Collapsible sidebar + top mobile bar + Electron drag region + `Cmd/Ctrl+K` listener | Desktop & Web |
| `CommandPalette` | `Cmd/Ctrl+K` quick-switcher for views, sessions, new session, theme, inspector | Desktop & Web |
| `Dialog` | Accessible modal (`Esc` close, focus trap, Cancel default) | Confirmations, Quick Prompts, Tour |
| `Toast` | Non-blocking bottom-right notice with dismiss & optional action | Copy feedback, Save status, Offline notice |
| `Popover` | Anchored floating panel (`Esc` / click-outside close) | Model menu, IDE Usage popover, Filter popover |
| `Menu` | Keyboard-navigable action menu (arrow keys, `Enter`) | Workspace `Actions`, Export, Context menu |
| `Tooltip` | Accessible `title` / hover-focus hint | Trust provenance, reset timestamps |
| `SegmentedControl` | Single-row pill switcher | Inspector (`Files / Changes / Artifacts / Terminal`), Theme (`System / Light / Dark`) |
| `FormControls` | Unified `Button` (`primary`, `secondary`, `ghost`, `danger`), `Input`, `Select`, `Switch` | Launcher, Settings, Account |

---

## 5. Planned PR Sequence

1. **PR 1 — Tokens & Primitives:** `gnomcode-tokens.css` (new semantic tokens, accent/danger separation, contrast fix), `gnomcode-console.css` cleanup, shared UI primitives (`Dialog`, `Toast`, `Popover`, `Menu`, `SegmentedControl`, `FormControls`).
2. **PR 2 — Shell & Navigation:** Unified collapsible `ConsoleShell`, `CommandPalette` (`Cmd/Ctrl+K`), `Cmd/Ctrl+N`, Electron drag region, non-overlapping tour banner.
3. **PR 3 — Workspace Screen:** Grouped virtualized/scrollable sidebar, clean 52px header, `72ch` single-scroll transcript, unified Right Inspector (`Files / Changes / Artifacts / Terminal`), compact composer with inline model menu + IDE Usage popover + inline error recovery.
4. **PR 4 — Home & Usage:** Full 53-week Activity grid on Home, `ProviderOverview` partial-exhaustion state (with `SPEC.md` §3.9 update + Rust test), `ApprovalsCard` & `UsageView` polish.
5. **PR 5 — Settings, Account & Secondary:** `SettingsView`, `AccountView`, `Doctor` / `Updates` / `About` / `Tour` using unified primitives.
6. **PR 6 — Web Console, Remote & Mobile/PWA:** `/console`, `/remote` (`RemoteCompanion`), `AppLockGate`, phone stack/sheet navigation, 44px targets, keyboard avoidance & safe-area insets.
7. **PR 7 — Marketing & Final Polish:** Visual token alignment across `/`, `/pricing`, `/downloads`, `/docs`, `/privacy`, `/terms`, plus full `apple-design` + `impeccable` final audit.
