# 0011 — Electron shell with a standalone Rust daemon

- **Status:** Accepted for Phase 1; prototype implemented; parity and packaging in progress
- **Date:** 2026-10-28
- **Supersedes:** the Tauri shell choices in 0001 and 0003; the shell deferral in 0006

## Decision

Use Electron for windows, tray and notifications. Keep the Rust daemon a separate singleton process that owns product logic and serves the standalone CLI and local dashboard. Closing or crashing Electron must not stop the daemon or an agent. Existing tray and dashboard behavior defines the parity baseline; platform-specific gaps remain explicit.

Load only packaged local UI, with context isolation, sandboxing, no Node integration, a restrictive CSP, a narrow typed preload, validated allowlisted IPC and no release devtools. Authenticate local daemon access and preserve its loopback, Host and Origin protections. Do not expose credentials to the renderer. Specify platform installers, signed updates and rollback before release; measure whole-process idle memory on each supported OS.

## Consequences

Electron adds runtime memory and packaging costs. Platform-native focus and notification behavior needs parity testing. This decision does not assert that Electron or an updater has shipped.

The daemon HTTP client uses Reqwest’s Rust TLS backend for cross-platform packaging, including Linux musl, avoiding a system OpenSSL requirement. Native trust-store handling remains provided by the TLS verifier.
