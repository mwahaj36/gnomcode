# Console workspace and account surfaces

Mode: Operate. Primary artifact: `crates/daemon/src/console.html`. Related surfaces: web console, Remote, Profile, shared controls and account-aware marketing navigation.

## Direction contract

**THESIS:** Give conversation work a dedicated, breathable application workspace; keep tracking in Dashboard and personal/device configuration in Profile.

**OWN-WORLD:** Spawnt near-black/light canvases, Inter body text, red primary actions and right-hand user bubbles, neutral gray agent replies, geometric SVG icons, hairline divisions and restrained 6px controls. Avoid oversized headers, pills and debug-style toolbars.

**STORY:** Find a readable conversation, choose a desktop workspace, prompt the actual CLI and read formatted replies. Track usage separately. Pair by entering the desktop code in web Profile, without links or QR and followed by the phone six-digit security code entered on desktop and confirmed on phone. Remembered devices resume after reload/background through challenge-bound proof, expire after seven inactive days and can be revoked.

**FIRST VIEWPORT:** Far-left desktop hover/focus rail with a touch toggle, independently collapsible conversation list, compact conversation title/actions, readable center transcript and a one-line composer growing to 160px. Phone uses one title toolbar and a drawer; content wraps within bubbles.

**FORM:** Code-led application panes prescribed by the user's Codex/Antigravity references. Signature interaction: desktop rail reveals labels without shifting content; touch uses its toggle; composer grows with input. Reduced motion removes nonessential transitions. seed: none (unavailable).

**FINISH:** unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Verification boundaries

Browser captures use synthetic data at 1440px and 390px in both themes. Every native view, web route/console subview, mobile marketing menu and shared confirmation dialog is included. Paired Remote additionally uses an actual encrypted browser exchange through the public relay with a synthetic desktop. No physical phone, macOS or Linux runtime validation is claimed. Screenshots are development evidence and are not product raster assets.

## Final interaction details

Paired and reconnecting Remote occupy a fixed, visual-viewport-bounded workspace with a New toolbar action. Messages scroll above the safe-area-aware composer; read-only history offers a new conversation instead of prompting. The folder picker keeps Cancel and Use this folder outside its scrolling directory list. Conversation history collapses explicitly on desktop and uses a phone drawer.

Model selection applies immediately, with requested versus observed state kept distinct and dated change notices between rules in both histories; non-Git workspaces also support handoff. Shared scrollbars follow the active theme and defer to system colors in forced-colors mode. Existing native titlebar behavior remains unchanged.
