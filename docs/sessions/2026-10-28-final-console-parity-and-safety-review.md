## 2026-10-28 — final console parity and safety review

- Kept Settings inside the shared console rail and heading; the existing `/settings` URL forwards to its console destination. Extracted the settings panel without changing its independent account data path.
- Replaced separate web/local provider breakdowns with one keyboard-accessible presenter that preserves expanded rows on polling. Removed boxed trust treatments, made chart entry sequential, and labeled missing session details unknown.
- Escaped local automation, relay and pricing metadata, including safe inline action arguments; added a persistent regression check. Moved report and project SQLite queries to blocking tasks. Local project date filters now reject obsolete responses and use the same local-day boundary as reports.
- Verification: presenter/settings/local-script checks and full Windows Rust gate passed. Production web build and Rust 1.87 minimum-version check passed. Source-matching Windows/Linux artifacts are rebuilt; macOS and visual checks remain externally blocked. Browser visual parity cannot be asserted while browser tools remain disabled.
