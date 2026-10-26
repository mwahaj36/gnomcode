# Changelog

All notable changes to FleetHUD are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [1.3.0] — 2026-10-27

### Added
- Automatic cloud session collection with existing account pairing and explicit identities for all supported agents.
- Metadata-only measured-usage imports, invocation deduplication and estimated API-equivalent costs.
- Account-isolated cloud session storage and dashboard visibility with honest unknown-usage states.
- Windows x64 and Linux x86_64/ARM64 release archives with third-party notices and SHA-256 manifests.

### Fixed
- Patched ctutils vulnerability RUSTSEC-2026-0385 and repaired cargo-deny configuration (Rust 1.87 minimum).
- Unix installer now verifies, extracts and installs its package instead of reporting success after only downloading it.
- Windows PowerShell updater verifies its archive and native installer version before launching an upgrade.
- Installer packaging validates the embedded engine and notices, preserves account data, and synchronizes release versions.

### Availability
- macOS packages and native macOS execution remain pending a working build runner. GitHub Actions is currently blocked by account billing/spending limits.
- npm publication and .deb/.rpm/AppImage packages are not part of this release; download labels reflect actual availability.

## [Unreleased]

### Added
- Initial Rust Cargo workspace (`fleethud-core`, `fleethud-adapters`, `fleethud-store`, `fleethud-platform`, `fleethud-ingest`, `fleethud-daemon`, `fleethud` CLI).
- Single source of truth specification (`docs/SPEC.md`), engineering rules (`AGENTS.md`), and Architecture Decision Records (`docs/adr/0001`–`0003`).
- Workspace engineering skills (`.agents/skills/`): `ponytail`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `rust-best-practices`, `tauri-v2`, `supabase-postgres-best-practices`, `rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`.
- Binary and UI anti-reverse-engineering hardening (`Cargo.toml` release profile, `.cargo/config.toml` path remapping).
- GitHub Actions CI pipeline across Linux, Windows, and macOS (`.github/workflows/ci.yml`).
