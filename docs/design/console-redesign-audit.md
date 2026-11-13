# Gnomcode Console Redesign Audit

Date: 2026-11-14
Author: Hydrogen (Lead Product Designer & Frontend Engineer)
Scope: Local Console (`crates/daemon/src/console.html`, `web/local-console/*`), Web Console & Remote (`web/app/console/Client.tsx`, `web/components/RemoteCompanion.tsx`), Shared Presentation & Tokens (`web/lib/console-presentation.js`, `web/app/gnomcode-tokens.css`, `web/app/gnomcode-console.css`), Marketing & Docs (`web/app/*`).
Frameworks: React 19 + Next.js 15 (web/PWA) and Electron (desktop shell), sharing CSS tokens and presenters.
Viewports audited: `1440x900` (desktop), `1024x768` (compact desktop/tablet), `390x844` (phone), in both dark and light appearances.

---

## Executive Summary

**Overall rating: Needs work.**

The current console has strong product bones—honest trust provenance, local-first architecture, and a clear 4-place navigation model—but its execution feels like a collection of bolted-on panels rather than a cohesive commercial desktop and mobile app. The Workspace screen suffers from triple-column crowding, nested scrollbars, competing headers, mismatched tab controls, wrapping quota pills, an oversized 2010-era composer, and error messages buried below collapsed disclosures. Home feels sparse and unbalanced (a 12-week heatmap stranded in a full-width card), and multi-pool provider cards misreport partial exhaustion as total outage (`Out`).

### What works today (preserve)

- **Four-place IA:** Home, Workspace, Usage, Settings (`ConsoleShell.tsx`) is clean and focused.
- **Trust & provenance discipline:** `EXACT`, `EST`, `API`, `UNKNOWN` and `est.` suffixes on estimated spend (`calm-presentation.test.mjs`).
- **Read-only safety boundaries:** Files, Changes, and Artifacts panels strictly avoid editing/reverting, and HTML artifacts render in `sandbox=""`.
- **Collapsible provider cards (`details.sp-provider-card`):** Compact summary rows with meter bars and pace markers (`console-presentation.js`).

---

## Five-Lens Audit (Apple HIG + Impeccable)

### Lens 1: Accessibility (Critical / High)

1. **Light-mode `--sp-text-3` contrast failure on `--sp-surface-2` (Critical):** `#657184` on `#edf0f3` yields ~4.25:1, failing WCAG 2.1 AA (4.5:1 for text under 18pt) per `accessibility.md › Vision`. Fix: darken light-mode `--sp-text-3` to `#525e70` (5.7:1 on `#edf0f3`, 6.5:1 on `#ffffff`).
2. **Sub-minimum type sizes (High):** `gnomcode-console.css` contains 28 instances of non-scale font sizes (`10px`, `11px`, `13px`, `16px`, `18px`), violating `typography.md › Ensuring legibility` (macOS min 10pt/default 13pt; iOS min 11pt/default 17pt) and `DESIGN.md` (12px floor).
3. **Color-only / unclear quota urgency (High):** Conversation header quota pills (`renderConversationLimits`) wrap into 5 separate colored pills (`100% left`, `13% left`, `Exhausted`) with no unified status or accessible summary trigger, and `3% left` reads visually identical in weight to `100% left` aside from tint.
4. **Pagination instead of virtualized/scrollable list & missing keyboard list navigation (High):** `WorkspaceView.tsx` forces `1 / 1 · 3 conversations` pagination at the bottom of the sidebar and lacks arrow-key navigation in lists (`keyboards.md`).
5. **Phone Workspace landing hides the list without clear affordance (High):** At `390x844`, `#sessions` opens with the conversation list hidden and only a small sidebar icon in the top-left, causing confusion and empty-feeling screens.
6. **Hit targets under 44px on touch & dense desktop controls (Medium):** Secondary buttons, filter selects, and inline disclosures fall below 44px on touch viewports (`accessibility.md › Mobility`).

### Lens 2: Platform Conventions (High)

1. **Three stacked columns on Desktop Workspace (High):** `ConsoleShell` left rail (64px) + `ide-sidebar` (300px) + `ide-main` eat 364px before content starts; when Files/Changes/Artifacts/Terminal open, the chat column is squeezed or pushed off-screen (`designing-for-macos.md › Best practices`).
2. **Scroll-inside-scroll on Workspace (High):** `.sp-chat-thread` scrolls inside `.ide-main` while the outer workspace/page also has scrollbars, breaking natural reading flow and keyboard PageUp/Down.
3. **Two competing tab systems in Workspace header (High):** `Files / Changes / Artifacts` are styled as large rounded buttons below a divider, while `Terminal` and `Actions` sit as plain text links in the top-right (`buttons.md`, `segmented-controls.md`).
4. **Missing desktop native ergonomics (Medium):** No `Cmd/Ctrl+K` command palette, no `Cmd/Ctrl+N` shortcut for new session, no right-click context menus on conversation rows, and no dedicated Electron title bar drag region.
5. **Phone navigation & keyboard avoidance (High):** Mobile top rail works for 4 places, but Workspace lacks a proper stack/sheet pattern for the conversation list and bottom-docked composer above the virtual keyboard with safe-area insets (`designing-for-ios.md`).

### Lens 3: Visual Design & Craft (High / Medium)

1. **Brand red (`#ff2544`) overused on non-danger actions (High):** Red is used on `Send`, `New session`, `Open guide`, `Continue here`, `Sign in`, and user chat bubbles. Red should be reserved for danger; one calm accent color should carry primary actions and active states (`color.md › Best practices`).
2. **Duplicate / noisy Workspace header chrome (High):** Top bar says `Workspace`, then the pane header repeats a 3-line wrapping session title + `antigravity · Observed` + 5 wrapping quota pills + `Terminal` + `Actions` + `Files / Changes / Artifacts` + `1 message`.
3. **Oversized, dated composer (High):** Native `<select>` for model (`Agent default`) looks like a 2010 form control, sits next to mismatched `Attach` and red `Send` buttons, and leaves `QuickActionsBar` (`Continue / Run tests / Summarize changes / Customize`) floating with no visual attachment to the composer.
4. **Home Activity card looks "empty and sad" (High):** Home renders a 12-week mini heatmap inside a full-width 1200px card, leaving ~85% empty black void on the right. Usage's 53-week grid with stats (`tokens`, `active days`, `day streak`, `busiest day`) and legend fills the card naturally and should be shared on Home.
5. **Multi-pool provider card chip misleads on partial exhaustion (High):** Antigravity shows `Out` (red) in the summary header when only 1 of 3 pools (`Gemini 4 Argon`) is exhausted while `Claude and GPT models` is 97% and `Gemini Models` is 83%/13%. Should only show `Out` when all pools are exhausted; otherwise `Low` (amber) with `1 pool out` in the summary line.
6. **Hard-coded colors & spacing drift (Medium):** `gnomcode-console.css` still has 14 hard-coded `#hex`/`rgb()` values and mixed 8px/12px/40px+ gaps outside the 4px spacing scale.
7. **First-run onboarding tip overlaps content (Medium):** The floating bottom-right tip card (`Open guide / Dismiss`) overlaps the bottom tile on Home and the `Continue here` banner on Workspace at `1440x900` and `390x844`.

### Lens 4: Interaction & Error Recovery (High)

1. **Delivery error stranded far from composer (High):** In `WorkspaceView.tsx:2167`, `{deliveryStatus}` renders *below* `<details className="sp-chat-automation">` and `.sp-workspace-terminal`, completely detached from the composer and with zero recovery actions (`Retry`, `Open terminal`).
2. **Sidebar filter clutter (Medium):** Stacked `New` button + `History` native `<select>` + `Search conversations` + `<details>Filters</details>` + `1 / 1 · 3 conversations` pagination eat half the sidebar before showing a single session.
3. **Inconsistent dialogs, toasts, and menus (Medium):** Mix of native `<dialog>`, `<details>` dropdowns, inline status `<p>`, and ad-hoc banners instead of unified Dialog, Toast, Menu, Popover, Tooltip, and Form Control primitives.

### Lens 5: Content & Writing (Medium)

1. **System-centric labels (`writing.md`):** Labels like `Observed`, `1 / 1 · 3 conversations`, `Continue here`, `Focus Agent Window` mix casing and expose internal mechanics.
2. **Verbose helper paragraphs:** Settings and Workspace still contain multi-sentence helper blocks in default views instead of concise labels + on-demand tooltips/disclosures.

---

## Full Surface & State Audit Table

| Page / Surface | State | Platform | Issue | Severity | Fix PR |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Tokens & Primitives** | Light & Dark | Shared | Light `--sp-text-3` (`#657184`) fails 4.5:1 on `--sp-surface-2`; 14 hard-coded colors & 28 non-scale font sizes in `gnomcode-console.css`; red `#ff2544` used for primary actions instead of danger-only | Critical | PR 1: Tokens & Primitives |
| **App Shell (`ConsoleShell`)** | Default / Narrow / Phone | Desktop & Web | Rail + sidebar double-column eats width; missing `Cmd/Ctrl+K` palette, `Cmd/Ctrl+N`, drag region; onboarding tip overlaps bottom content | High | PR 2: Shell & Layout |
| **Home (`HomeView`)** | Populated / Empty | Desktop & Web | 12-week mini heatmap leaves 85% empty card space; use full 53-week Activity grid with stats; multi-pool card shows `Out` when only 1 pool is out | High | PR 4: Home & Usage |
| **Home (`HomeView`)** | Empty / Daemon down | Desktop & Web | Error fallback sets raw `textContent` string with no Retry action or structured empty state | Medium | PR 4: Home & Usage |
| **Workspace — Sidebar** | 0 / 100+ items | Desktop & Web | Native `<select>`, stacked Search + Filters + Pagination (`1 / 3`) instead of clean search + filter popover + virtualized/scrollable list + keyboard arrows | High | PR 3: Workspace |
| **Workspace — Header** | Chat open | Desktop & Web | Wrapping title + 5 wrapping quota pills + two mismatched tab systems (`Files/Changes/Artifacts` buttons vs `Terminal/Actions` text) | High | PR 3: Workspace |
| **Workspace — Transcript** | Scrolling / Long code | Desktop & Web | Nested scroll-inside-scroll; lacks centered ~72ch readable measure with full-width code blocks | High | PR 3: Workspace |
| **Workspace — Inspector** | Files / Changes / Artifacts / Terminal | Desktop & Web | Panels stack awkwardly inside `.ide-main` instead of a single consistent right inspector (desktop) / sheet (mobile) | High | PR 3: Workspace |
| **Workspace — Composer** | Idle / Typing / Error | Desktop & Web | Oversized 2010 `<select>`, mismatched `Attach` & red `Send`, floating `QuickActionsBar`, `deliveryStatus` error stranded below collapsed Automation with no `Retry`/`Open terminal` | High | PR 3: Workspace |
| **Workspace — Quota** | Multi-pool / Urgent | Desktop & Web | 5 pills clutter header; replace with single compact IDE-style trigger + Usage popover showing all pools, rings/bars, and reset times; `3% left` must read as urgent | High | PR 3: Workspace |
| **Workspace — Launcher** | New session | Desktop & Web | Form fields use inconsistent heights/spacing and native selects; needs unified form controls and clean directory picker | Medium | PR 3: Workspace |
| **Usage (`UsageView`)** | All time / Today / Empty | Desktop & Web | Period select & Export disclosure styling inconsistent with new controls; provider cards need partial-exhaustion state fix | Medium | PR 4: Home & Usage |
| **Approvals (`ApprovalsCard`)** | Pending / Busy | Desktop & Web | Banner buttons lack unified hierarchy; needs clean inline action card matching new tokens | Medium | PR 4: Home & Usage |
| **Settings (`SettingsView`)** | All tabs/sections | Desktop & Web | Long single scroll with mixed disclosure styles, native inputs, and verbose helper text; organize cleanly with unified form controls & sections | Medium | PR 5: Settings, Account & Secondary |
| **Account (`AccountView`)** | Signed out / Paired / Error | Desktop & Web | Token input & sign-in form use inconsistent spacing and raw status paragraphs | Medium | PR 5: Settings, Account & Secondary |
| **Doctor / Updates / About / Tour** | Modal / Status | Desktop & Web | Native `<dialog>` and update rows lack unified modal/banner styling and keyboard focus polish | Medium | PR 5: Settings, Account & Secondary |
| **App Lock (`AppLockGate`)** | Locked / Error | Web / PWA | PIN/unlock screen needs token alignment and 44px touch targets | Medium | PR 6: Web Console, Remote & Mobile/PWA |
| **Web Console (`/console`)** | Unpaired / Synced | Web / PWA | Unpaired state shows 3 stacked empty boxes with `Unknown` tiles; needs calm onboarding/connect state and matching shell | High | PR 6: Web Console, Remote & Mobile/PWA |
| **Remote (`/remote`, `RemoteCompanion`)** | Connected / Reconnecting / Phone | Web / PWA | Bottom sheets, composer above keyboard, safe-area insets, and conversation header chips need alignment with new Workspace & popover model | High | PR 6: Web Console, Remote & Mobile/PWA |
| **Marketing (`/`, `/pricing`, `/downloads`, `/docs`, `/privacy`, `/terms`)** | Desktop & Phone | Web | Token alignment pass (border, surface, accent, button radius/contrast) so marketing and console share one visual language | Low | PR 7: Marketing & Final Polish |
