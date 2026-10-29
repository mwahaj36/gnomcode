## 2026-10-28 — canonical spawnt mark correction

- Goal: remove the incorrect outlined bracket mark and align every current web/local console and Windows icon use with the approved solid triangle-and-square geometry.
- Outcome: added `web/public/spawnt-mark.svg` as the web/local console mark source; both console headers and marketing header/footer/tour now use that asset. Removed the incorrect SVG copies, updated browser and local-console icon references, regenerated the website, tray, and download ICOs from the canonical asset, and corrected both Windows installer screen marks.
- Decision: keep `/spawnt.svg` as a daemon compatibility alias that serves the canonical mark; `/spawnt-mark.svg` is the preferred path. No data, auth, or business rules changed.
- Verification: `npm run build --prefix web`, full Rust quality gate, and `git diff --check` passed; cargo-deny is not installed. Inspected the generated mark and desktop landing/local-console headers in Chromium. The updated installer source was reviewed but its native runtime preview and macOS/Linux installer paths are outside this icon-only correction; native tray window design remains rollout work.

