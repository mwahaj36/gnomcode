# Session: In-app Desktop Auto-Update (PR 3 / A2)

- Date: 2026-11-13
- Branch: `feat/auto-update`
- Spec & ADR: [`docs/SPEC.md`](../SPEC.md) (§6.4, §9), [`docs/adr/0033-desktop-auto-update.md`](../adr/0033-desktop-auto-update.md), [`docs/ROADMAP_1.0.md`](../ROADMAP_1.0.md)

## Goals

- Add in-app auto-update checking on daemon startup and every 6 hours using the already-allowed `{base_url}/downloads/latest.json` manifest GET.
- Verify installer SHA-256 checksums and optional Ed25519 signatures in Rust, stage downloaded installers under `GNOMCODE_HOME/updates`, and fail open on any error.
- Surface update state (`Update available`, `Restart to update`, auto-check opt-out) in Settings and the Electron tray menu without auto-installing mid-session.

## Changes

- `crates/core/src/update.rs`: Pure manifest parsing, strict SemVer comparison (`> current` without prerelease upgrades), allowed origin enforcement (matching `{base_url}` HTTPS or loopback test HTTP), path-traversal filename checks, SHA-256 and Ed25519 verification, and `UpdateStatus` state model.
- `crates/store/src/lib.rs`: Persistent `desktop_preferences` toggle for `auto_update_enabled`.
- `crates/daemon/src/update.rs` & `crates/daemon/src/api.rs` & `crates/cli/src/main.rs`: Background update worker (startup + 6h interval), background download & atomic staging, opt-out honoring DB preference and `GNOMCODE_AUTO_UPDATE`, and authenticated endpoints (`GET /api/v1/update/status`, `POST /api/v1/update/check`, `POST /api/v1/settings/update`).
- `apps/desktop/src/{updater,tray-menu,preload,main}.ts`: Tray menu `"Update available"` and `"Restart to update"` actions, IPC handler `desktop:restart-to-update`, independent pre-launch path-containment and SHA-256 re-verification, and install on quit or explicit user restart click only.
- `crates/daemon/src/{console.html,console-app.js}`: `#settings-updates` section with auto-check toggle, status text, Check now button, and Restart to update button.

## Verification

- Rust gate (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `CARGO_INCREMENTAL=0 cargo test --workspace`).
- Desktop tests, typecheck, and build (`apps/desktop`: `npm test`, `npm run typecheck`, `npm run build`, eslint on edited modules).
- Console syntax/markup validation (`node scripts/check-console.mjs`).
