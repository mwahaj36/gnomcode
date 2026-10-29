## 2026-10-28 — platform brand assets and font parity

- Added a reproducible generator using the canonical spawnt SVG for ICO, ICNS, Apple, and Linux PNG sizes. Windows setup now uses that generated image, the approved canvas/text palette, plain spawnt branding, and a red progress track. Installation paths and registry identity remain compatible with existing installs.
- Bundled Inter and JetBrains Mono with their licenses for the web and embedded local console, avoiding platform-dependent font substitution and external runtime requests. Moved local view navigation below the heading to match the web console's hierarchy; corrected local help URLs.
- Verification: Windows installer source compiles with embedded canonical mark. Full Rust gate (including cargo-deny) and production web build passed. Platform packages will be recreated after console completion.
