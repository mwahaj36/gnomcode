---
name: gnomcode
description: Calm, conversation-first workspace for AI coding agents; datasheet precision for numbers.
colors:
  ink-dark: "#8ea2ff"
  ink-light: "#2d43c0"
  on-ink-dark: "#0a0f2e"
  on-ink-light: "#ffffff"
  canvas-dark: "#0d1015"
  surface-dark: "#12161d"
  surface-raised-dark: "#181d26"
  overlay-dark: "#1c222c"
  border-dark: "#252b36"
  border-strong-dark: "#363e4c"
  text-dark: "#edf0f5"
  text-secondary-dark: "#b4bccb"
  text-muted-dark: "#939cad"
  canvas-light: "#f5f5f0"
  surface-light: "#ffffff"
  surface-raised-light: "#ecece5"
  border-light: "#dadad0"
  border-strong-light: "#c2c2b6"
  text-light: "#13161c"
  text-secondary-light: "#434955"
  text-muted-light: "#5b616d"
  ok-dark: "#4ad295"
  warn-dark: "#f2b14c"
  danger-dark: "#ff6b72"
  ok-light: "#0f7f4f"
  warn-light: "#965800"
  danger-light: "#c0262d"
  brand-red: "#ff2544"
  heat-dark: ["#181d26", "#233063", "#3149a6", "#5670dc", "#8ea2ff"]
  heat-light: ["#e6e6de", "#c9d0f5", "#93a2ea", "#5a6fd6", "#2d43c0"]
typography:
  family: "Inter (self-hosted), -apple-system, Segoe UI, sans-serif"
  mono: "JetBrains Mono (self-hosted), for code, commands and terminals only"
  display: { fontSize: "clamp(40px, 4.3vw, 56px)", fontWeight: 600, letterSpacing: "-0.03em" }
  title: { fontSize: "15px", fontWeight: 600, letterSpacing: "-0.01em" }
  section: { fontSize: "16px", fontWeight: 600 }
  page-heading: { fontSize: "20-22px", fontWeight: 600 }
  body: { fontSize: "14px", lineHeight: 1.6 }
  conversation: { fontSize: "14px", lineHeight: 1.7 }
  control: { fontSize: "13px", fontWeight: 500 }
  data: { fontSize: "12px", fontVariantNumeric: "tabular-nums" }
  column-head: { fontSize: "11px", fontWeight: 600, letterSpacing: "0.05em", textTransform: "uppercase" }
  spec-value: { fontSize: "28px", fontWeight: 600, letterSpacing: "-0.02em" }
rounded:
  code: "4px"
  control: "6px"
  card: "10px"
  popover: "10px"
  dialog: "14px"
  composer: "14px console, 16px Remote"
  bubble: "16px with a 6px tail corner"
spacing:
  tight: "4px"
  small: "8px"
  control: "12px"
  regular: "16px"
  section: "24px"
  wide: "32px"
  large: "48px"
  spacious: "64px"
controls:
  page-action: "36px high, 0 14px padding"
  dense: "32px high, 0 10px padding (history column, conversation header, banners, tools pane, menus)"
  touch: "44px minimum at 899px and below or on coarse pointers"
  icon-only: "square, ghost, accessible name"
---

# Design System: gnomcode

## Overview

**Creative North Star: "Datasheet, conversation first."** Redesign of 2026-11-25. Owner brief: replace the visual world, use Inter, cover the whole app including marketing. Owner pin: simple and functional like the Claude app on a phone. The world comes from a component datasheet, where every number is a specification with its condition stated. It lends type, palette, density and one signature move; navigation and controls stay platform-standard.

Gnomcode keeps its lowercase wordmark and solid triangle-and-square mark. Interface chrome is calm: graphite (dark) and paper (light) grounds, hairline rules, one ink-blue interactive colour and Inter throughout. Brand red is retired from the interface; the activity heat ramp follows the ink primary (owner decision 2026-11-25, recorded in the e2e suite).

**Signature: the spec row.** Metrics read as an 11px uppercase column head, a tabular value, a smaller unit and a stated trust condition (Exact, Est, API, Unknown). Used for Usage totals, Home limits, the conversation header and the site's illustrative tour.

**Key Characteristics:**

- One interactive colour (ink) for primary actions, focus, selection, links and switches; state colours only where they carry meaning.
- Inter for every interface and site voice; monospace only for code, commands and terminals.
- Flat at rest with one shadow for floating layers; hairlines organise content.
- Shape tells function: chevrons disclose, carets open menus and pickers, three dots open overflow, arrows navigate.

## Colors

`web/app/gnomcode-tokens.css` is the runtime source; the frontmatter records its values. Pairings meet WCAG 2.2 AA in both themes (text at least 4.5:1; ink on canvas above 7:1).

### Primary

- **Ink** (`ink-dark` / `ink-light`) is the single interactive colour. Text on ink uses `on-ink`.
- **Activity heat ramp** runs from the raised neutral to full ink in five steps, so activity reads as the product colour. Its legend labels the scale; colour never stands alone.
- **Brand red** (`brand-red`) is kept as a recorded brand value only; it is not used in the interface.

### Secondary

- State colours (`ok`, `warn`, `danger`) carry status and always pair with a text label. `danger` marks destructive actions: outlined in menus and lists, filled only in the confirming step.

### Neutral

- Canvas, surface, raised and overlay form a four-step ladder: rail and history on canvas, the work pane on surface, controls and user bubbles on raised, menus and dialogs on overlay.
- Hover, pressed and selected are shared tints (`--sp-hover`, `--sp-pressed`, `--sp-selected`), so every control responds the same way.

**The Foreground Pair Rule.** A control's foreground and background switch theme together; never carry a light foreground onto a light fill or the reverse.

Status and numeric trust remain separate from colour. Use short human-readable statuses such as Running, Waiting, Idle and Ended. Declare numeric provenance at section or column level and label estimated money visibly. Colour never substitutes for the label.

## Typography

Inter (self-hosted, no remote font request) is the application and site family, with `cv05`/`cv08` for an unambiguous l and I in paths and tabular numerals for aligned metrics. Monospace is reserved for code, executable commands and terminals and is never decoration for labels or navigation.

- **Display:** marketing headlines only.
- **Titles:** 15px semibold for the toolbar and conversation title; 16px for section heads; 20-22px for Settings and Account page headings.
- **Body and conversation:** 14px; conversation leading 1.7.
- **Column heads:** 11px uppercase with 0.05em tracking, for spec labels and grouped list heads only. No kicker or eyebrow labels above headings.
- **Controls:** 13px medium, sentence case.

**The Task Before Chrome Rule.** Application headings identify the current task; they do not repeat a marketing hero above it.

## Layout

Use the established spacing rhythm in frontmatter. Marketing content remains centered around a 1200px maximum, with narrower mobile gutters. Open sections and aligned grids separate content without introducing a floating card for every group.

The local and account web consoles occupy a bounded viewport. A compact desktop primary rail (64px) expands only through its toggle (176px), pushing content rather than covering it. Workspace history owns an independent resizable column (260px by default, 220-440px within desktop layouts). The page toolbar is a separate row (52px); notices flow beneath it within a bounded scrolling region. Page chrome, history, transcript and tools each own their available space, without adding document scrolling.

At compact widths (899px and below), labeled bottom navigation (56px) replaces the desktop rail and history opens as a full-work-area drawer above it. Selecting a session or New closes the drawer. Compact interactive targets are at least 44px; short landscape windows arrange history controls across a small grid, preserving space for the scrollable list. The compact page toolbar is 48px. Electron adds its own titlebar above the same layout, rather than changing page composition.

The conversation transcript takes the remaining workspace height and scrolls independently above its composer. Both use a readable maximum width (800px). The Tools control opens an anchored overview menu; a selected tool docks beside the conversation at a preferred 560px, resizable from 360px to 960px and bounded to 55% of the available workspace width. Resize handles support pointer dragging, arrow keys and Home reset; overlay tools hide their resize handle. When the workspace container is 880px or narrower, that tool occupies the conversation area instead of squeezing it. Files, Changes, Artifacts, automation and terminals share the same tool surface. File navigation stacks above the preview below a 600px tool-content width. Remote retains its own encrypted data adapter and bounded visual-viewport treatment; its transport and permissions remain separate from account snapshots.

Home aligns Sessions and Limits at their headers in a two-to-one grid, stacking below 1100px. Usage owns reporting periods, refresh, activity and cost information. Home and Usage content can extend to 1440px; Settings uses a 1200px maximum with a 200px category column, switching to its section selector below 1100px. Unknown values and estimates remain explicit. Route-specific capability and security behavior belongs in the product specification rather than this visual system.

The landing tour preserves its compact illustrative overview: three KPIs, a short chart and short activity rows. Full daily grids and session details belong in the relevant tour views. Keep KPI labels, values and trust aligned; narrow tour rows omit a column consistently rather than changing tracks per row.

## Elevation & Depth

Flat surfaces and hairlines provide the default depth; cards declare a border, never a border plus a wide shadow. Menus, popovers, dialogs, toasts and the Remote composer use the overlay surface and the single popover shadow. Dialogs sit over a theme-aware scrim and enter with one 180ms rise, which reduced motion removes.

**The Flat At Rest Rule.** Elevation communicates a floating interaction layer, not decoration behind content.

## Shapes

Controls use the 6px control radius; cards and popovers 10px; dialogs 14px; composers 14-16px; user bubbles 16px with a 6px tail corner. Segmented controls (theme, conversation filters) use a raised track with a surface thumb. Grid cells, dividers and reporting rows keep straight edges. Borders remain 1px.

The canonical logo stays an unframed solid triangle-and-square mark in `currentColor`, from `web/public/gnomcode-mark.svg`. Platform icon geometry is unchanged.

## Components

### Buttons and fields

One control scale: page actions are 36px high with 14px side padding; dense panes (history column, conversation header, banners, tools pane, menus) use 32px with 10px; every control is at least 44px at 899px and below or on coarse pointers. Labels and icons centre on one axis with a 6px gap. Primary is filled ink, secondary an outlined surface, and ghost controls (header tools, icon-only buttons) stay transparent until hover. Destructive actions are outlined in danger and filled only in the confirming step. Pressed state applies immediately; disabled controls drop to 50% with a not-allowed cursor.

Focus is a 2px ink outline with a 2px offset on every interactive element. Composers show focus once, on the outer frame; the editor inside has no outline or border. Fields use the surface fill with the strong border; placeholders use muted text at full opacity. Binary settings are switches (a styled native checkbox that falls back to the system control in forced colors). Never render missing values as `undefined` or `NaN`.

Accessible names contain the visible label (WCAG 2.5.3): a button that shows "Tools" is named Tools, and the limits trigger reads "Remaining limits:" followed by its visible text. Scrollable regions are keyboard focusable. An axe-core WCAG 2.2 AA audit (`web/e2e/axe.spec.ts`) runs on every console view in both themes at desktop and phone widths.

### Navigation and theme

Marketing headers retain Product, Agents, Docs, Pricing, an account link and Download. Log in/Console is the primary red action. Desktop places secondary Download to its left; phone headers keep the account action visible and put Download in the menu. The landing hero illustrates a concise conversation handoff between agents. Its full-width moving agent strip remains a separate section, with a static reduced-motion fallback; usage reporting stays in the lower product tour. The account link shows Console and opens it directly for a signed-in user; a signed-out user sees Log in. Mobile menus originate beneath their trigger, expose expanded state, dismiss on Escape and restore focus.

Application navigation uses Home, Workspace, Usage and Settings in the left rail and a compact current-view toolbar. Settings contains theme, account and devices. Theme is a three-option segmented radio group (System, Light, Dark) with arrow-key selection; System follows the OS, stored overrides apply before first paint, and controls stay readable throughout the switch. The marketing footer retains its small theme control; Auth's phone menu also exposes it.

Desktop rail expansion is explicit and exposes expanded state to assistive technology; hover and focus never change its width. Its tab orientation is vertical on desktop and horizontal in compact bottom navigation. Shared state transitions use the recorded easing and duration; remove nonessential looping or travel for reduced motion while preserving clear state changes. The supported-agent strip becomes a static wrapping list in reduced motion. Workspace itself avoids an entry transform that would offset the fixed drawer.

### Reporting containers and badges

Reporting uses hairline grids, divided rows and truthful unavailable states. Repeated trust badges are replaced by plain-language section labels and a source-details disclosure. Keep labels separate from values and align comparable rows. Shared thin scrollbars follow theme surfaces and text tones; forced-colors restores the system scrollbar palette. Dense populated metadata tables may use an explicit scroll container; empty-state text must fit the phone without horizontal scrolling.

### Conversation bubbles and composer

User prompts align right on a bordered neutral surface. Agent replies align left directly on the conversation surface, with metadata beneath the reply. Submission provides optimistic user and waiting feedback. Reconcile that feedback with recorded history without duplicating messages; failed delivery preserves the draft. Keep user prompts literal. Replies may render the supported safe Markdown subset, with headings, lists, links, code and tables fitting the bubble. Long code lines wrap without changing copied text; simple tables fit their width, while wide phone tables become labeled rows. Bubble content must not require sideways scrolling.

The composer separates a scrollable editor region from its fixed model, attachment and Send toolbar. Text grows within the available editor region; the toolbar never joins that scrolling area. Short compact landscape windows preserve an editor region of at least 44px, reduce the field height and omit message-count-only status and keyboard-help copy when needed to retain input and Send. Focus is shown once, on the outer composer: its existing 1px border changes to the theme accent with no shadow. The inner textarea has no separate rectangular outline or shadow. Local and Remote composers share this treatment. The provider/model control belongs beside the composer, groups choices by provider and includes each provider's default model. Selecting a model requests the change immediately within the same visible conversation. Distinguish the request from agent-observed confirmation using centered status text and a centered timestamp, without side rules. Keep CLI commands, session IDs and generated continuation briefings out of ordinary chat. Preserve the title, history, selection and draft across provider switches. Saved supported chats offer Resume conversation on desktop and mobile; mobile requires the laptop and Gnomcode online. Conversation headers show compact remaining-limit readings with source details available on demand, refreshed with incoming observations; unknown and older readings remain explicit. Handoff also supports a workspace outside Git. Put secondary session actions in an anchored menu. Keep per-session drafts and delivery feedback independent of history updates, and preserve the reader's position when new messages arrive. The workspace path field includes a compact geometric folder control rather than a separate bulky button. A same-workspace xterm pane keeps interactive terminal controls beside the conversation; preserve its terminal rendering and keyboard behavior rather than presenting it as chat output.

### Settings, pairing and feedback

Settings uses open sections and aligned action groups. Advanced settings use disclosures. Account connection distinguishes account state from publication state, masks the desktop token field, names the actual web action, and offers a separate Disconnect action when connected.

Devices is one account-wide screen. Before linking a native app through either sign-in or browser-account transfer, show the existing confirmation modal explaining that every current and future device signed into the account can access this native app's available conversations and supported controls. Cancel submits nothing. After confirmation, phones and browsers connect automatically without approval steps or code fields. Existing links require this new confirmation before broader access is enabled. Devices shows each native app's availability and can disable that host for every signed-in device. Re-enabling on native uses the same disclosure. Approval does not expire with updates or inactivity. Keep connection availability separate from account permission, preserve drafts during bounded recovery, and never expose credentials or proof material.

Remote history without a live Gnomcode-controlled agent offers Resume conversation where supported, otherwise Read-only history and New conversation. Mobile chat uses direct conversation-list, new-chat and desktop-connection controls; no generic Actions menu. Desktop connection and model selection use separate labeled bottom sheets with close controls. Keep model selection beside the composer. During reconnection, retain visible chat and the draft, disable delivery, and offer Try again plus Dashboard/Profile navigation without requiring the user to forget their desktop. The desktop-folder dialog scrolls its directory list independently while Cancel and Use this folder remain in a fixed footer.

Confirmation dialogs describe the action and consequence, focus Cancel initially, use an explicit action label and cancel on Escape. Ordinary feedback uses a dismissible app notice rather than a blocking browser alert.

### Command snippets

Commands keep an integrated clipboard icon with a 44px target, an accessible name, keyboard focus and visible success/failure feedback. Copy the exact displayed command. Long snippets safely wrap or use a deliberate code container while the copy control remains visible.

### Reporting and workspace primitives

The runtime token file owns the 4-64px spacing scale and application type roles. Reporting cards use the panel radius, controls the restrained control radius, and meters the small meter radius. Provider cards are collapsed disclosures with a monochrome provider mark, current summary, status label and optional command or recovery action. Session rows keep the title above agent/status metadata and a timestamp; their separate action control uses the shared geometric SVG treatment. Filters and selected tabs use a neutral raised fill rather than a new accent color.

Keep numeric trust at the section or value that needs it. Meter state, descriptive statuses, source disclosures and explicit empty/error/loading states remain separate from decoration. Use a small component vocabulary across Home, Workspace, Usage and Settings; preserve distinct states without repeating a floating container around each subsection.

## Do's and Don'ts

### Do:

- **Do** use the canonical mark, the ink interactive colour and theme-specific foreground/background pairs.
- **Do** use restrained control corners, aligned action groups and visible keyboard focus.
- **Do** keep application chrome compact and make mobile conversation content fit its bubble.
- **Do** label unknown values and numeric trust accurately, preserving the Rust-owned reporting boundary.
- **Do** verify both themes, narrow layouts, open menus, dialogs and actual loading/error states.

### Don't:

- **Don't** use emojis, oversized pills, decorative gradients or color-only status indicators.
- **Don't** restore the oversized console hero, duplicate top navigation or debug-style action toolbar.
- **Don't** turn ordinary dashboard sections into nested floating cards.
- **Don't** use monospace for labels, navigation or decoration; it is for code, commands and terminals.
- **Don't** infer release readiness, physical-device support or security certification from design captures.
