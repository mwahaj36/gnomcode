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
  text-muted-light: "#525e70"
  accent-dark: "#becbd8"
  accent-light: "#252e99"
  on-primary: "#0c1119"
  action-dark: "#f6f7f8"
  action-text-dark: "#0c1119"
  action-light: "#252e99"
  action-text-light: "#ffffff"
  heat-dark-0: "#17171a"
  heat-dark-1: "#4a121b"
  heat-dark-2: "#7d1626"
  heat-dark-3: "#ba1c35"
  heat-dark-4: "#ff2544"
  heat-light-0: "#e4e9ee"
  heat-light-1: "#fcd1d8"
  heat-light-2: "#f799a8"
  heat-light-3: "#ec526b"
  heat-light-4: "#ff2544"
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
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.7
  toolbar:
    fontFamily: "system-ui, Segoe UI, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.5
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
  composer: "10px"
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
  button-send-dark:
    backgroundColor: "{colors.action-dark}"
    textColor: "{colors.action-text-dark}"
    typography: "{typography.control}"
  button-send-light:
    backgroundColor: "{colors.action-light}"
    textColor: "{colors.action-text-light}"
    typography: "{typography.control}"
  composer-dark:
    backgroundColor: "{colors.surface-raised-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.composer}"
    padding: "12px"
  user-prompt-dark:
    backgroundColor: "{colors.surface-raised-dark}"
    textColor: "{colors.text-dark}"
    rounded: "{rounded.conversation}"
    padding: "12px 16px"
---

# Design System: gnomcode

## Overview

**Creative North Star: "Mission control"**

Gnomcode keeps its lowercase wordmark, canonical solid triangle-and-square mark, red accent and near-black/light canvases. The interface is flat, precise and restrained: hairline divisions organize information; actions have modest corners and readable sans labels. The approved console redesign replaces the oversized branded hero and debug-style control rows with compact application chrome.

Marketing keeps its established editorial scale and detaching navigation. Application surfaces prioritize task space, readable conversations and predictable controls. The accepted composition and verification boundaries live in the maintainer-local console surface brief; public product boundaries are recorded in [the product brief](docs/PRODUCT.md).

**Key Characteristics:**

- Canonical red accents and quiet neutral surfaces.
- Sans-led hierarchy with monospace reserved for data and commands.
- Restrained controls, open divided sections and geometric SVG icons.
- Compact application chrome with independently scrolling work areas.

## Colors

The palette pairs a vivid red accent with cool neutral canvases, surfaces, borders and text. `web/app/gnomcode-tokens.css` is the runtime source; the frontmatter records its current values.

### Primary

- **Gnomcode red** (`primary`) identifies brand emphasis and deliberate attention. It also supplies the activity heat ramp; it is not the default user-message fill. Text on red uses `on-primary` in both themes.
- **Theme-paired Send action** (`action-dark` / `action-light` and their text tokens) keeps the composer action distinct: light-on-dark in the dark theme and deep-blue-on-light in the light theme. Stop remains a separate red action.
- **Readable accent text** (`primary-text-dark` / `primary-text-light`) is the theme-specific red used for small links and accent labels. Do not substitute the fill color for small text on light backgrounds.

### Secondary

- **Cool neutral accent** (`accent-dark`) and **deep blue accent** (`accent-light`) retain the existing theme-aware secondary emphasis.

### Neutral

- Canvas tokens supply the near-black or cool light ground. Surface and raised-surface tokens distinguish panes, fields and controls.
- Border tokens make hairline divisions; strong borders clarify controls and floating surfaces.
- Main, secondary and muted text tokens keep hierarchy readable in either theme.

The activity scale progresses through the recorded theme-specific red tones. Its legend and labels establish that these cells measure activity, rather than reporting an error.

**The Foreground Pair Rule.** A control's foreground and background must switch theme together; never carry a light foreground onto a light fill or the reverse.

Status and numeric trust remain separate from brand color. Use short human-readable statuses such as Running, Waiting, Idle and Ended. Explain observed/inferred status in source details rather than repeating uncertainty in every row. Declare numeric provenance at section or column level, in accessible source details and relevant tooltips; label estimated money visibly. Color never substitutes for the label.

## Typography

The platform system sans is the user-approved application family. Use it for headings, body, controls, data and timestamps, with tabular numerals for aligned metrics. Monospace is reserved for code and executable commands. Existing bundled fonts remain available for non-application surfaces; no remote font request is needed.

- **Display:** the frontmatter's responsive display role belongs to marketing headlines, not console page chrome.
- **Application titles:** compact semibold titles distinguish the active view or conversation. Section headings are larger than body copy without becoming page heroes.
- **Body and conversation:** body and transcript text use the regular application size, with conversation leading from its own role. Code and Markdown retain readable line spacing.
- **Metadata:** navigation labels, session metadata and timestamps use at least the 12px data role. The collapsed desktop rail deliberately hides its text labels while retaining accessible names; compact bottom navigation shows labels.
- **Controls:** sentence-case sans labels share one weight and scale. Avoid technical-looking monospace labels for ordinary buttons.
- **Data:** system sans with tabular numerals keeps application rows aligned. Executable snippets and code retain monospace.

**The Task Before Chrome Rule.** Application headings identify the current task; they do not repeat a marketing hero above it.

## Layout

Use the established spacing rhythm in frontmatter. Marketing content remains centered around a 1200px maximum, with narrower mobile gutters. Open sections and aligned grids separate content without introducing a floating card for every group.

The local and account web consoles occupy a bounded viewport. A compact desktop primary rail (64px) expands only through its toggle (176px), pushing content rather than covering it. Workspace history owns an independent resizable column (260px by default, 220–440px within desktop layouts). The page toolbar is a separate row (52px); notices flow beneath it within a bounded scrolling region. Page chrome, history, transcript and tools each own their available space, without adding document scrolling.

At compact widths (899px and below), labeled bottom navigation (56px) replaces the desktop rail and history opens as a full-work-area drawer above it. Selecting a session or New closes the drawer. Compact interactive targets are at least 44px; short landscape windows arrange history controls across a small grid, preserving space for the scrollable list. The compact page toolbar is 48px. Electron adds its own titlebar above the same layout, rather than changing page composition.

The conversation transcript takes the remaining workspace height and scrolls independently above its composer. Both use a readable maximum width (800px). The Tools control opens an anchored overview menu; a selected tool docks beside the conversation at a preferred 560px, resizable from 360px to 960px and bounded to 55% of the available workspace width. Resize handles support pointer dragging, arrow keys and Home reset; overlay tools hide their resize handle. When the workspace container is 880px or narrower, that tool occupies the conversation area instead of squeezing it. Files, Changes, Artifacts, automation and terminals share the same tool surface. File navigation stacks above the preview below a 600px tool-content width. Remote retains its own encrypted data adapter and bounded visual-viewport treatment; its transport and permissions remain separate from account snapshots.

Home aligns Sessions and Limits at their headers in a two-to-one grid, stacking below 1100px. Usage owns reporting periods, refresh, activity and cost information. Home and Usage content can extend to 1440px; Settings uses a 1200px maximum with a 200px category column, switching to its section selector below 1100px. Unknown values and estimates remain explicit. Route-specific capability and security behavior belongs in the product specification rather than this visual system.

The landing tour preserves its compact illustrative overview: three KPIs, a short chart and short activity rows. Full daily grids and session details belong in the relevant tour views. Keep KPI labels, values and trust aligned; narrow tour rows omit a column consistently rather than changing tracks per row.

## Elevation & Depth

Flat surfaces and hairlines provide the default depth. Floating menus and dialogs use a separate tonal surface and border. The connection notification uses a modest downward soft shadow (`0 8px 24px rgb(0 0 0 / .18)`); the confirmation backdrop dims the rest of the interface. Marketing navigation retains its scroll-dependent detaching shadow. Do not add decorative halos or shadows to ordinary dashboard sections.

**The Flat At Rest Rule.** Elevation communicates a floating interaction layer, not an extra decoration behind content.

## Shapes

Controls use the restrained `control` radius, replacing the old square-control guidance. They are neither sharp boxes nor pills. Code spans use the smaller code radius. Dialogs use the panel radius; composers use their own slightly tighter radius. User prompts use larger, uniformly rounded corners; agent replies remain open on the conversation surface. Grid cells, full-width dividers and reporting rows keep straight edges. Borders remain thin (1px).

The canonical logo stays an unframed solid triangle-and-square mark in `currentColor`, from `web/public/gnomcode-mark.svg`. Windows app and installer icons center that mark at 0.65x on the opaque dark canvas; the separate full-size tray icon retains its badge appearance. Small Windows ICO frames use DIB encoding for the existing tray renderer. These platform assets preserve their established geometry.

## Components

### Buttons and fields

Brand and general primary actions retain the red treatment; the composer Send action uses its dedicated theme pair. Secondary actions use quiet neutral fill; ghost actions remain transparent until interaction. Shared labels use the application sans and restrained corners. Dense desktop tools use 32-40px controls where appropriate, while primary rail destinations are 44-48px; compact targets are at least 44px. Buttons and standalone fields use a visible 2px focus outline in the theme accent (`--sp-accent`, never brand red), and pointer-down feedback is immediate. The composer uses the single outer focus treatment described below. Disabled actions remain distinct and cannot be mistaken for available actions.

Fields use theme-aware surfaces, main text and subtle borders. Associate visible labels with their fields. Error, pending and successful states use readable messages rather than color alone. Never render missing values as `undefined` or `NaN`.

### Navigation and theme

Marketing headers retain Product, Agents, Docs, Pricing, an account link and Download. Log in/Console is the primary red action. Desktop places secondary Download to its left; phone headers keep the account action visible and put Download in the menu. The landing hero illustrates a concise conversation handoff between agents. Its full-width moving agent strip remains a separate section, with a static reduced-motion fallback; usage reporting stays in the lower product tour. The account link shows Console and opens it directly for a signed-in user; a signed-out user sees Log in. Mobile menus originate beneath their trigger, expose expanded state, dismiss on Escape and restore focus.

Application navigation uses Home, Workspace, Usage and Settings in the left rail and a compact current-view toolbar. Settings contains theme, account and devices. Theme cycles System, Light, Dark; System follows the OS, stored overrides apply before first paint, and controls stay readable throughout the switch. The marketing footer retains its small theme control; Auth's phone menu also exposes it.

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
