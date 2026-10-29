## 2026-10-28 — v1.5.3 native release

- Rebuilt Windows x64 engine/setup/archive and Linux x64/ARM64 archives, including the distinct full-size tray icon. Synchronized website, installers, shims and checksum manifests to 1.5.3.
- Windows Rust gate including cargo-deny, Linux clippy/workspace tests, ARM64 smoke, Next production build, native JS/navigation/account/icon regressions, presence boundaries, focus authorization/failure and protected credential ACL checks passed. Installer resource verification includes the separate tray icon; corruption/preservation and packaging checks pass.
- Browser visual tooling remains disabled and macOS builds remain deferred. Window focus cannot select an exact chat where the host offers no supported targeting; failures retain an explicit copy-workspace fallback.

