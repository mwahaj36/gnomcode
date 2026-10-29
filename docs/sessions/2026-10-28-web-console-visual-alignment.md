## 2026-10-28 — web console visual alignment

- Goal: make the signed-in console feel like part of the spawnt marketing product rather than a separate card-heavy interface.
- Outcome: aligned the app navbar to the shared brand mark and token system, replaced boxed data widgets with open divided sections, used the marketing sans-led page hierarchy, and unified theme-aware button/link treatments. Removed square-bracket wrappers from status labels in the console and landing tour.
- Decision: preserve the separate local and cloud data/auth paths and the console's existing view navigation, filters, and refresh semantics.
- Follow-up: enlarged and top-aligned the mark with the title, then combined the disconnected-account and missing-desktop notices into one contextual connection row beneath the heading.
- Follow-up: the connection row is now signed-out only. Shared action buttons across marketing, downloads, Auth, Settings, and console use a consistent square 44 px treatment.
- Verification: `npm run build --prefix web` passed, `git diff --check` was clean, and Chromium desktop renders were reviewed in dark and light themes with the accessibility tree. Narrow viewport, keyboard-only, and reduced-motion checks remain pending; signed-in data was unavailable for this visual review.

