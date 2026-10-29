## 2026-10-28 — platform-selectable downloads

- Goal: replace the crowded three-column installer instructions with the platform-tab interaction shown in the console tour and default to the visitor's platform.
- Outcome: added Windows, macOS, and Linux tabs with browser-platform detection, arrow/Home/End keyboard navigation, focus states, a single focused release panel, and reduced-motion behavior. Kept published URLs and marked macOS unavailable until a verified package exists.
- Verification: production Next.js build and `git diff --check` passed. Chromium accessibility tree confirmed tab roles, selected state, Windows release links, and the macOS unavailable state after selection; desktop light and dark styling were inspected. Narrow viewport and non-Windows platform default require additional review.
