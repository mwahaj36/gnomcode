---
name: gnomcode
description: Restrained mission control for AI coding agents.
colors:
  primary: "#ff2544"
  primary-text-dark: "#ff6177"
  primary-text-light: "#d4102e"
  canvas-dark: "#070708"
  surface-dark: "#0d0d0f"
  surface-raised-dark: "#141416"
  border-dark: "#242428"
  border-strong-dark: "#38383e"
  text-dark: "#f6f7f8"
  text-secondary-dark: "#a9b4c2"
  text-muted-dark: "#8d99aa"
  canvas-light: "#f6f7f8"
  surface-light: "#ffffff"
  surface-raised-light: "#edf0f3"
  border-light: "#d9dfe6"
  border-strong-light: "#becbd8"
  text-light: "#0c1119"
  text-secondary-light: "#4a5565"
  text-muted-light: "#657184"
  accent-dark: "#becbd8"
  accent-light: "#252e99"
  on-primary: "#0c1119"
typography:
  display:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "clamp(40px, 4.3vw, 56px)"
    fontWeight: 600
    lineHeight: 1.07
    letterSpacing: "-0.03em"
  title:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "18px"
    fontWeight: 600
    lineHeight: 1.4
  section:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.6
  conversation:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.75
  control:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.4
  data:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.5
rounded:
  code: "4px"
  control: "6px"
  panel: "12px"
  conversation: "16px"
spacing:
  tight: "4px"
  small: "8px"
  control: "12px"
  regular: "16px"
  section: "24px"
  wide: "32px"
  large: "48px"
  spacious: "64px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: "9px 14px"
  button-secondary-dark:
    backgroundColor: "{colors.surface-raised-dark}"
    textColor: "{colors.text-dark}"
    typography: "{typography.control}"
    rounded: "{rounded.control}"
    padding: "9px 14px"
  composer-dark:
    backgroundColor: "{colors.surface-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.panel}"
    padding: "10px 12px"
  agent-bubble-dark:
    backgroundColor: "{colors.surface-raised-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.conversation}"
    padding: "16px 20px"
---

# Design System: gnomcode

## Overview

**Creative North Star: "Mission control"**

Gnomcode keeps its lowercase wordmark, canonical solid triangle-and-square mark, red accent and near-black/light canvases. The interface is flat, precise and restrained: hairline divisions organize information; actions have modest corners and readable sans labels. The approved console redesign replaces the oversized branded hero and debug-style control rows with compact application chrome.

Marketing keeps its established editorial scale and detaching navigation. Application surfaces prioritize task space, readable conversations and predictable controls. The accepted composition and verification boundaries live in [the console surface brief](docs/design/console-workspace.md); product and security behavior remain governed by [SPEC](docs/SPEC.md).

**Key Characteristics:**

- Canonical red accents and quiet neutral surfaces.
- Sans-led hierarchy with monospace reserved for data and commands.
- Restrained controls, open divided sections and geometric SVG icons.
- Compact application chrome with independently scrolling work areas.

## Colors

The palette pairs a vivid red accent with cool neutral canvases, surfaces, borders and text. `web/app/gnomcode-tokens.css` is the runtime source; the frontmatter records its current values.

### Primary

- **Gnomcode red** (`primary`) identifies primary actions, attention and user chat bubbles. Text on red uses `on-primary` in both themes.
- **Readable accent text** (`primary-text-dark` / `primary-text-light`) is the theme-specific red used for small links and accent labels. Do not substitute the fill color for small text on light backgrounds.

### Secondary

- **Cool neutral accent** (`accent-dark`) and **deep blue accent** (`accent-light`) retain the existing theme-aware secondary emphasis.

### Neutral

- Canvas tokens supply the near-black or cool light ground. Surface and raised-surface tokens distinguish panes, fields and agent replies.
- Border tokens make hairline divisions; strong borders clarify controls and floating surfaces.
- Main, secondary and muted text tokens keep hierarchy readable in either theme.

**The Foreground Pair Rule.** A control's foreground and background must switch theme together; never carry a light foreground onto a light fill or the reverse.

Status and numeric trust remain separate from brand color. Use short human-readable statuses such as Running, Waiting, Idle and Ended. Explain observed/inferred status in source details rather than repeating uncertainty in every row. Declare numeric provenance at section or column level, in accessible source details and relevant tooltips; label estimated money visibly. Color never substitutes for the label.

## Typography

The platform system sans is the user-approved application family. Use it for headings, body, controls, data and timestamps, with tabular numerals for aligned metrics. Monospace is reserved for code and executable commands. Existing bundled fonts remain available for non-application surfaces; no remote font request is needed.

- **Display:** the frontmatter's responsive display role belongs to marketing headlines, not console page chrome.
- **Application titles:** compact semibold titles distinguish the active view or conversation. Section headings are larger than body copy without becoming page heroes.
- **Body and conversation:** body uses the regular role; chat uses the more generous conversation leading, reducing to a smaller body size on phones.
- **Controls:** sentence-case sans labels share one weight and scale. Avoid technical-looking monospace labels for ordinary buttons.
- **Data:** system sans with tabular numerals keeps application rows aligned. Executable snippets and code retain monospace.

**The Task Before Chrome Rule.** Application headings identify the current task; they do not repeat a marketing hero above it.

## Layout

Use the established spacing rhythm in frontmatter. Marketing content remains centered around a 1200px maximum, with narrower mobile gutters. Open sections and aligned grids separate content without introducing a floating card for every group.

The native and web consoles occupy the available window. Their fixed left rail is compact (64px desktop, 56px phone). Fine-pointer desktop navigation reveals labels on hover or keyboard focus, overlaying at 208px without moving the conversation. Touch navigation uses an explicit toggle; there is no pin mode. Profile sits at the bottom of the full-height rail. Application views keep independent data/auth adapters while sharing presentation tokens and formatting.

Workspace places conversation history beside the transcript and anchors the composer beneath it. Native history uses a 280px desktop pane; Remote uses 260px. Conversation lists have an explicit collapse control. Phone layouts replace the list with a drawer anchored to the content edge, use one conversation toolbar with a New action, and retain readable touch targets. Paired and reconnecting Remote fill a bounded viewport updated from the visual viewport; only messages, lists and modal content scroll, keeping the composer above the keyboard and safe-area edge. The precise route composition belongs to the surface brief rather than a new global layout rule.

Dashboard presents activity before summary, then limits and ongoing sessions; narrow layouts stack them. Activity adapts its visible interval from daily to ISO-weekly, monthly or yearly totals as the reporting span grows, while Today remains hourly. Long-period reporting labels the selected period, aggregation interval and actual UTC chart window. Keep provenance at section or column level and distinguish unknown data from zero. Profile groups account, devices, appearance, data and desktop downloads, with advanced plans/pricing and offline license disclosed.

The landing tour preserves its compact illustrative overview: three KPIs, a short chart and short activity rows. Full daily grids and session details belong in the relevant tour views. Keep KPI labels, values and trust aligned; narrow tour rows omit a column consistently rather than changing tracks per row.

## Elevation & Depth

Flat surfaces and hairlines provide the default depth. Floating menus and dialogs use a separate tonal surface and border. The connection notification uses a modest downward soft shadow (`0 8px 24px rgb(0 0 0 / .18)`); the confirmation backdrop dims the rest of the interface. Marketing navigation retains its scroll-dependent detaching shadow. Do not add decorative halos or shadows to ordinary dashboard sections.

**The Flat At Rest Rule.** Elevation communicates a floating interaction layer, not an extra decoration behind content.

## Shapes

Controls use the restrained `control` radius, replacing the old square-control guidance. They are neither sharp boxes nor pills. Code spans use the smaller code radius. Composers and dialogs use the panel radius; chat bubbles use larger corners with one small trailing corner. Grid cells, full-width dividers and reporting rows keep straight edges. Borders remain thin (1px).

The canonical logo stays an unframed solid triangle-and-square mark in `currentColor`, from `web/public/gnomcode-mark.svg`. Windows app and installer icons center that mark at 0.65x on the opaque dark canvas; the separate full-size tray icon retains its badge appearance. Small Windows ICO frames use DIB encoding for the existing tray renderer. These platform assets preserve their established geometry.

## Components

### Buttons and fields

Primary actions use red fill with dark text; secondary actions use a quiet theme-aware neutral fill; ghost actions remain transparent until interaction. Shared button labels use the control role, consistent padding and restrained corners. Desktop density may use 40px controls; phone targets are at least 44px. Keyboard focus is a visible 2px outline in the theme accent (`--sp-accent`, never brand red), and pointer-down feedback is immediate. Disabled actions remain distinct and cannot be mistaken for available actions.

Fields use theme-aware surfaces, main text and subtle borders. Associate visible labels with their fields. Error, pending and successful states use readable messages rather than color alone. Never render missing values as `undefined` or `NaN`.

### Navigation and theme

Marketing headers retain Product, Agents, Docs, Pricing, an account link and Download. Log in/Console is the primary red action. Desktop places secondary Download to its left; phone headers keep the account action visible and put Download in the menu. The landing hero illustrates a concise conversation handoff between agents. Its full-width moving agent strip remains a separate section, with a static reduced-motion fallback; usage reporting stays in the lower product tour. The account link shows Console and opens it directly for a signed-in user; a signed-out user sees Log in. Mobile menus originate beneath their trigger, expose expanded state, dismiss on Escape and restore focus.

Application navigation uses the left rail and compact current-view title rather than the removed top brand/settings/theme bar. Profile contains theme, account and devices. Theme cycles System, Light, Dark; System follows the OS, stored overrides apply before first paint, and controls stay readable throughout the switch. The marketing footer retains its small theme control; Auth's phone menu also exposes it.

Desktop rail labels reveal on hover or visible keyboard focus; touch uses a labeled toggle exposing expanded state to assistive technology. Default transitions use the existing exponential ease-out curve; remove nonessential looping or travel for reduced motion while preserving clear state changes. The supported-agent strip becomes a static wrapping list in reduced motion. Workspace itself avoids an entry transform that would offset the fixed drawer.

### Reporting containers and badges

Reporting uses hairline grids, divided rows and truthful unavailable states. Repeated trust badges are replaced by plain-language section labels and a source-details disclosure. Keep labels separate from values and align comparable rows. Shared thin scrollbars follow theme surfaces and text tones; forced-colors restores the system scrollbar palette. Dense populated metadata tables may use an explicit scroll container; empty-state text must fit the phone without horizontal scrolling.

### Conversation bubbles and composer

User messages align right in red with dark text; agent replies align left on a neutral gray surface. Submission immediately adds the optimistic user bubble and a gray agent waiting bubble. Reconcile that feedback with recorded history without duplicating messages; failed delivery preserves the draft. Keep user prompts literal. Replies may render the supported safe Markdown subset, with headings, lists, links, code and tables fitting the bubble. Long code lines wrap without changing copied text; simple tables fit their width, while wide phone tables become labeled rows. Bubble content must not require sideways scrolling.

The composer starts compact and grows with text to a bounded maximum (160px), then scrolls vertically inside its field. The provider/model control belongs beside the composer, groups choices by provider and includes each provider's default model. Selecting a model requests the change immediately within the same visible conversation. Distinguish the request from agent-observed confirmation using centered status text and a centered timestamp, without side rules. Keep CLI commands, session IDs and generated continuation briefings out of ordinary chat. Preserve the title, history, selection and draft across provider switches. Saved supported chats offer Resume conversation on desktop and mobile; mobile requires the laptop and Gnomcode online. Conversation headers show compact remaining-limit readings with source details available on demand, refreshed with incoming observations; unknown and older readings remain explicit. Handoff also supports a workspace outside Git. Put secondary session actions in an anchored menu. Keep per-session drafts and delivery feedback independent of history updates, and preserve the reader's position when new messages arrive. The workspace path field includes a compact geometric folder control rather than a separate bulky button. A same-workspace xterm pane keeps interactive terminal controls beside the conversation; preserve its terminal rendering and keyboard behavior rather than presenting it as chat output.

### Profile, pairing and feedback

Profile uses open sections and aligned action groups. Advanced settings use disclosures. Account connection distinguishes account state from publication state, masks the desktop token field, names the actual web action, and offers a separate Disconnect action when connected.

Devices is one account-wide screen. Before linking a native app through either sign-in or browser-account transfer, show the existing confirmation modal explaining that every current and future device signed into the account can access this native app's available conversations and supported controls. Cancel submits nothing. After confirmation, phones and browsers connect automatically without approval steps or code fields. Existing links require this new confirmation before broader access is enabled. Devices shows each native app's availability and can disable that host for every signed-in device. Re-enabling on native uses the same disclosure. Approval does not expire with updates or inactivity. Keep connection availability separate from account permission, preserve drafts during bounded recovery, and never expose credentials or proof material.

Remote history without a live Gnomcode-controlled agent offers Resume conversation where supported, otherwise Read-only history and New conversation. Mobile chat uses direct conversation-list, new-chat and desktop-connection controls; no generic Actions menu. Desktop connection and model selection use separate labeled bottom sheets with close controls. Keep model selection beside the composer. During reconnection, retain visible chat and the draft, disable delivery, and offer Try again plus Dashboard/Profile navigation without requiring the user to forget their desktop. The desktop-folder dialog scrolls its directory list independently while Cancel and Use this folder remain in a fixed footer.

Confirmation dialogs describe the action and consequence, focus Cancel initially, use an explicit action label and cancel on Escape. Ordinary feedback uses a dismissible app notice rather than a blocking browser alert.

### Command snippets

Commands keep an integrated clipboard icon with a 44px target, an accessible name, keyboard focus and visible success/failure feedback. Copy the exact displayed command. Long snippets safely wrap or use a deliberate code container while the copy control remains visible.

## Console overhaul system

The local console uses four places, a token set, a small component vocabulary and a list of anti-clutter rules. [The overhaul plan](docs/design/console-overhaul-plan.md) and [its mockup](docs/design/console-mockup.html) are the build reference; this section is the standing contract.

### Information architecture

The rail has exactly four places: Home, Workspace, Usage and Settings. Home shows provider cards, sessions grouped by Needs you, Running and Recent, three stat tiles and a 12-week activity grid, in that order. Workspace opens on the conversation list; the launcher is behind New. Usage holds limits, the 53-week activity grid with stats and legend, the Today hourly strip and cost reporting. Settings holds providers, appearance, devices, automations and license. The old `#limits`, `#projects` and `#automations` addresses redirect to Usage and Settings. The rail is a toggle-only column that pushes content; labels never cover it.

### Tokens

`web/app/gnomcode-tokens.css` is the source. Spacing uses `--sp-1` to `--sp-7` (4, 8, 12, 16, 24, 32, 48). Type uses `--sp-fs-meta` 12, `--sp-fs-body` 14, `--sp-fs-strong` 15, `--sp-fs-title` 20 and `--sp-fs-number` 28; nothing smaller than 12px except heatmap day labels, and 16px only for phone form inputs. Cards use 12px radius, controls 6px, meters 3px. State colors `--sp-ok`, `--sp-low`, `--sp-exhausted`, `--sp-unknown` (with soft backgrounds) mean limit state only. The activity ramp `--sp-heat-0..4` is blue so it never reads as an alarm; in light theme level 0 carries a strong-border outline (at least 1.5:1 against the card) and every step is distinct. Tabular numerals apply to every number in the native console.

### Components

Card, section header, chip (`sp-chip` with `ok`, `low`, `exhausted`, `unknown`, `neutral` states; also used per window in the conversation header), meter row (role `meter`, pace marker), collapsible provider card (`details.sp-provider-card`, collapsed by default, equal height in a grid row), stat tile, session row, activity grid with legend, copy command button and empty state (one line plus one action).

### Anti-clutter rules

One primary number per card. No nested borders. Three text colors only. Color means state; brand red is reserved for the single primary button per view and user chat bubbles. No explanatory paragraphs in default views; use a chip with a tooltip or a disclosure. Trust is shown once per card footer and inline only for estimated money. Empty states are one line plus one action. Content is at most 1200px wide. Tap targets are at least 44px on phones and for rail buttons, copy buttons, session rows and clickable chips; dense desktop form controls may stay at 40px.

## Do's and Don'ts

### Do:

- **Do** use the canonical mark, shared palette and theme-specific foreground/background pairs.
- **Do** use restrained control corners, aligned action groups and visible keyboard focus.
- **Do** keep application chrome compact and make mobile conversation content fit its bubble.
- **Do** label unknown values and numeric trust accurately, preserving the Rust-owned reporting boundary.
- **Do** verify both themes, narrow layouts, open menus, dialogs and actual loading/error states.

### Don't:

- **Don't** use emojis, oversized pills, decorative gradients or color-only status indicators.
- **Don't** restore the oversized console hero, duplicate top navigation or debug-style action toolbar.
- **Don't** turn ordinary dashboard sections into nested floating cards.
- **Don't** substitute a display or monospace face for the user-approved system application typography.
- **Don't** infer release readiness, physical-device support or security certification from design captures.
