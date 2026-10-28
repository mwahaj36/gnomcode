# 0007 — Native release fallbacks during spawnt rollout

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

The shipped Windows surface is a script tray and browser app window; there is no Tauri application. The existing Unix installer accepts Darwin but no macOS archive was published. Distribution domains, binary names and data paths must remain compatible.

## Decision

Release the existing Windows installer/tray and build native Linux and macOS CLI/daemon archives on their respective architectures. Each Unix archive includes a launcher, canonical icon and notices. macOS also includes an unsigned browser-console app bundle. These launch the same authenticated daemon console, not a second UI. Unix installations atomically replace the verified binary and leave user data untouched. No additional privileges, autostart or background services are enabled by the Unix installer.

Keep existing download URLs and internal identifiers. Publish release binaries in the existing private repository and static download directory; public distribution repositories, signing infrastructure, native Linux/macOS tray integration and package-manager channels remain future work. Never label an unsigned browser launcher a notarized Tauri app.

## Consequences

Every supported OS has the same local console design with a terminal/browser fallback. Windows has a tray; Linux/macOS have explicit browser-console launchers. Native CI verifies both architectures and updater checksum rejection. Signing/notarization remains unavailable and is disclosed in Downloads.
