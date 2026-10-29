## 2026-10-28 — landing tour alignment

- Goal: fix crowded KPI labels and inconsistent tour table columns reported in the landing page screenshots.
- Outcome: split each overview metric into a label and value/trust row; set shared table tracks so agent, state, usage, and trust columns line up. Narrow layouts consistently hide the usage value column.
- Verification: Next.js production build and `git diff --check` passed. Desktop landing preview screenshot and accessibility tree reviewed; the tour section still needs a fresh browser visual review after restarting the stale dev server, plus narrow viewport and keyboard review.

