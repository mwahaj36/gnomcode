## 2026-10-28 — settings route presentation

- Goal: bring Settings into the shared spawnt layout and remove the remaining bracketed cloud-sync navbar control.
- Outcome: replaced nested cards with three open, divided sections; aligned account state, password, subscription costs, and custom rates to square controls and shared typography. Missing price values now show `UNKNOWN` rather than `$0.00`; configured rates are identified as user-entered. The pricing table scrolls horizontally and can receive keyboard focus. Removed the `[Connect Cloud Sync]` header link; account connection remains available in Settings and Auth.
- Follow-up: replaced the filled Connect account button with a restrained text link after visual review.
- Verification: Next.js production build and `git diff --check` passed. Chromium review confirmed the route renders, sections and form controls are exposed accessibly, and the theme is applied. The local daemon is offline in this browser so live settings saves/password updates were not exercised; narrow viewport and keyboard-only review remain pending.

