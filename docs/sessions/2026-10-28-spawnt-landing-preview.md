## 2026-10-28 — spawnt landing preview

- Goal: build and show a local landing page using the attached `spawnt_redesign_plan.md`, while preserving the Daytona reference navbar behavior.
- Outcome: replaced the Next.js home route with a spawnt landing preview, including the supported-agent strip, feature cells, interactive console tour, privacy facts, and Free/Pro/Teams teaser. Kept other routes intact.
- Decision: console figures and tour screens are labeled illustrative until the redesigned console exists. Current downloads and pricing routes still carry Spawnt branding and need their planned redesign in a later phase. Removed a duplicate favicon asset that caused a route conflict.
- Verification: Next.js production build passed; local server and browser review performed in dark and light themes, including scroll navbar, tour tabs, selectable bars and rows, scroll-triggered graph motion, self-hosted agent logos, a tighter final CTA, a typographic footer inspired by footer.design, and device-default theme control moved to the footer.
- Added a Log in nav action (in the mobile menu on narrow screens) and routed the landing's console links through web authentication, since the web console requires a signed-in account to be useful.
- Preserved the user-provided `spawnt_redesign_plan.md` verbatim under `docs/`, rewrote `DESIGN.md` for the current red and near-black palette, and added `docs/spawnt_rollout_plan.md` to reconcile the draft with the repo's separate local and signed-in web consoles. No console implementation was changed in this documentation pass.

