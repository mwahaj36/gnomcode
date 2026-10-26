# DESIGN.md — FleetHUD Design System (Daytona Aesthetic)

## 1. Visual World & Philosophy
FleetHUD adheres to an uncompromising, high-density developer infrastructure aesthetic modeled after Daytona (`daytona.io`). The interface rejects generic "AI slop" (cluttered gradients, oversized padding, generic cards, pastel cards) in favor of high-contrast engineering restraint, crisp 1px borders, and zero-bounce spring physics.

## 2. Color Tokens
- **Canvas Base**: `#070708` with Daytona's authentic `68x68` dot-matrix background texture (`data:image/svg+xml` micro-dots) masked with a radial gradient.
- **Elevated Surfaces**:
  - `Surface 1`: `#0d0d0f` (cards, containers, window chrome)
  - `Surface 2`: `#091a11` (active states, green hover overlays)
  - `Surface 3`: `#141416` (secondary controls, icon tiles)
- **Borders & Dividers**:
  - `Border Subtle`: `rgba(255, 255, 255, 0.08)` (hairline structural grid)
  - `Border Bright`: `rgba(255, 255, 255, 0.18)` (hovered cells, floating navbar border)
  - `Border Green`: `rgba(46, 204, 113, 0.40)` (hovered logo cells, active agent state)
- **Typography Tokens**:
  - `Text Primary`: `#ffffff` (headings, active labels)
  - `Text Secondary`: `#8e8e93` / `#a1a1aa` (subtitles, body copy, inactive tabs)
  - `Text Muted`: `#55555c` (grid labels, timestamps, line numbers)
- **Functional Accents (Strict Unified Green Theme — No Blue)**:
  - `Accent Emerald Green`: `#2ecc71` / `#4ade80` (primary CTA buttons, links, active indicators, live telemetry)
  - `Accent Green Hover`: `#091a11` surface tint with `rgba(46, 204, 113, 0.25)` ambient glow
  - `Accent Amber Warning`: `#f59e0b` (quota approaching reset, approval required `[WAIT]`)
  - `Accent Rose Error`: `#f43f5e` (parse failure, agent crash, runaway loop)

## 3. Motion & Micro-Animation Specifications
- **Animating Floating Island Navbar**:
  - Flush at top of viewport (`top: 0`, `width: 100%`, `border-radius: 0`).
  - On scroll down (`scrollY > 35px`), it animates with spring physics into a floating island (`top: 14px`, `width: min(1240px, calc(100% - 2rem))`, `border-radius: 6px`, `box-shadow: 0 16px 36px -10px rgba(0, 0, 0, 0.85)`).
  - On scroll back to top, smoothly rejoins the viewport header.
- **Physics Engine**: Zero-bounce stiff springs (`cubic-bezier(0.16, 1, 0.3, 1)`, duration `0.3s` for navbar, `0.18s` for buttons/cards).
- **Zero Pills Rule**: Strictly no oval pills (`border-radius: 9999px`). All badges, tags, buttons, and status bars use crisp rectangular geometry with subtle 3px–6px radius.

## 5. Component Implementations
- **Web App**:
  - `MarketingNavbar.tsx`: Sticky glassmorphism header with logo monogram, `/` separator, uppercase monospace nav links, and high-contrast actions.
  - `Hero`: Asymmetric 2-column layout with bold headline, quickstart command bar, and interactive multi-language/multi-agent telemetry switcher.
  - `Status Pill`: Rounded dark oval with glowing neon blue dot, clean label, and chevron (`>`).
  - `Grid Rows`: Crisp 1px border-collapse layout for agent support logos and features.
  - `Computer Use`: Mock multi-window desktop interface showing live agent cursors.
  - `FAQ Accordion`: Clean monospace toggle list with square `[+]` / `[-]` indicator boxes.
  - `Footer`: 3-column navigation, Startup/Pro banner, popular topics, and round compliance seals (`100% LOCAL`, `FAIL OPEN`, `NO TELEMETRY`).
- **Native Tray App**:
  - Compact acrylic/mica dark window matching `#070708`.
  - High-density list of active agent sessions with plain-text status badges (`[WAIT]`, `[RUN]`, `[IDLE]`).
