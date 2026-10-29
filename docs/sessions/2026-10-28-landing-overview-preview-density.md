## 2026-10-28 — landing overview preview density

- Replaced the full daily grid and five-column session table in the Overview tour with a compact seven-day chart and three short agent/state/token rows, preserving the same illustrative fixtures, KPIs and trust labels.
- Kept detailed sessions in their dedicated tab, reduced tour-only row/padding density and retained reduced-motion behavior. The actual web/local console layouts are unaffected.
- Verification: a rendered-component regression failed on the full table before the change, then passed for three compact rows/seven bars and retained Sessions details. Production Next build passed. Browser visual tooling remains unavailable.

