# 0033 — Rust-verified desktop auto-update on quit or user restart

- **Status:** Accepted
- **Date:** 2026-11-13
- **Amends:** 0011 (adds explicit manifest verification and install-on-quit desktop update lifecycle)

## Context

`docs/SPEC.md` §6.4 permits an opt-out update-manifest GET request, and `0011` deferred packaging/updater mechanics. Release assets and checksum manifests are hosted on the official downloads domain (`branding.baseUrl/downloads`) and `mwahaj36/gnomcode-releases`, while `AGENTS.md` §2 rule 8 requires keeping update parsing, version comparison, and verification logic in Rust rather than TypeScript.

## Decision

1. **Rust daemon owns update checks, verification, and staging:**
   - The daemon checks `{branding.base_url}/downloads/latest.json` (overridable by `GNOMCODE_UPDATE_MANIFEST_URL`) once on startup and every 6 hours unless opted out via Settings (`auto_update_enabled = false`) or `GNOMCODE_AUTO_UPDATE=0`.
   - `gnomcode-core::update` parses the manifest strictly, validates allowed HTTPS release origins, compares SemVer (`major.minor.patch`) without downgrading, downloads the matching platform installer in the background to a `.part` file under `GNOMCODE_HOME/updates/`, verifies its SHA-256 checksum (and Ed25519 signature when both a manifest signature and verifying key are present) before promoting the staged file to `ready`.
   - Every network, parse, verification, or I/O error fails open without interrupting running agents or the UI.
2. **Electron desktop shell handles presentation and explicit user-triggered installation:**
   - Tray menu and Settings poll `GET /api/v1/update/status` to display `"Update available"` and `"Restart to update"` once verified and staged.
   - Electron never applies or executes an update mid-session automatically. It re-verifies the staged file path and SHA-256 hash inside `GNOMCODE_HOME/updates/` and launches the installer detached only when the user clicks `"Restart to update"` (Settings button or tray item) or on explicit application quit (`before-quit` / Quit tray action).

## Consequences

- Zero verification or version-selection logic lives in JavaScript/TypeScript.
- Users can opt out in Settings; running agent sessions are never restarted automatically.
- Unsigned development installers remain protected by SHA-256 manifest verification and explicit HTTPS release origin checks until platform code-signing certificates are provisioned.
