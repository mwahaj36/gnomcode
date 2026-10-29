## 2026-10-28 — v1.5.2 verified packages

- Rebuilt Windows x64 engine/setup/ZIP and Linux x64/ARM64 archives from the tray, coverage, period and ongoing-session fixes. Version metadata, website output, installer mirrors and SHA-256 manifests use 1.5.2.
- Windows full Rust gate including cargo-deny, Linux clippy/workspace tests, ARM64 version smoke, native icon/runtime/navigation tests, shared presenter and settings tests, Next production build, packaging checks and isolated upgrade/corrupt-manifest preservation checks passed. Native icon drawing now gates Windows release packaging.
- Clear cloud project-filter names on account changes. Project rollups and local IDs remain local. Scope assumes calendar month means the data source's local month; totals are never relabeled from All Time.
- Browser visual tooling remains unavailable, so no desktop/narrow screenshot parity pass is claimed. macOS builds remain explicitly deferred. Published historical artifacts and user data are retained.

