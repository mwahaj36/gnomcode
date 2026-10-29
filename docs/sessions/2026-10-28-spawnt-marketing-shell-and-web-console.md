## 2026-10-28 — spawnt marketing shell and web console

- Goal: bring the signed-in web console and shared marketing header/footer into the approved spawnt design while retaining the existing cloud data/auth path.
- Outcome: added seven console destinations, dedicated Usage and Limits views, a responsive console shell, updated trust badges, and a system/light/dark footer control. Rebuilt the shared marketing and account headers and footer to match the spawnt visual language. API-equivalent pricing is labeled estimated.
- Verification: `npm run build --prefix web` passed and `git diff --check` was clean. Desktop/narrow browser and keyboard/reduced-motion checks remain pending. This phase does not complete local console, native, icon, or installer redesign.
