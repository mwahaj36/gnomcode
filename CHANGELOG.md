# Changelog

All notable changes to FleetHUD are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Initial Rust Cargo workspace (`fleethud-core`, `fleethud-adapters`, `fleethud-store`, `fleethud-platform`, `fleethud-ingest`, `fleethud-daemon`, `fleethud` CLI).
- Single source of truth specification (`docs/SPEC.md`), engineering rules (`AGENTS.md`), and Architecture Decision Records (`docs/adr/0001`–`0003`).
- Workspace engineering skills (`.agents/skills/`): `ponytail`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `rust-best-practices`, `tauri-v2`, `supabase-postgres-best-practices`, `rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`.
- Binary and UI anti-reverse-engineering hardening (`Cargo.toml` release profile, `.cargo/config.toml` path remapping).
- GitHub Actions CI pipeline across Linux, Windows, and macOS (`.github/workflows/ci.yml`).
