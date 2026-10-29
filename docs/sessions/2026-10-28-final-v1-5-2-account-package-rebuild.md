## 2026-10-28 — final v1.5.2 Account package rebuild

- Rebuilt all Windows x64 and Linux x64/ARM64 release files again after the Account panel refinement so embedded HTML/CSS match the final source. Updated combined manifests and retained the existing 1.5.2 version while the release is unpublished.
- Final parity review replaced marketing-only CSS aliases with canonical shared tokens and added a regression preventing their reuse. Custom-rate inputs now shrink/wrap on narrow layouts; session filters and pricing inputs have explicit accessible names.
- Final source passed the Windows Rust gate, Linux clippy/workspace tests, ARM64 smoke, production web build, source/version audit, native JavaScript/navigation/account-state/icon tests and shared console/settings/tour regressions. Windows payload/updater and Unix corrupted-manifest preservation checks passed against the final files. macOS and browser screenshot checks remain explicitly deferred/unavailable.

