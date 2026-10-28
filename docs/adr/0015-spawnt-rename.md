# 0015 — Adopt Spawnt as the sole product identity

- **Status:** Accepted; implementation in progress
- **Date:** 2026-10-28
- **Supersedes:** product and repository names in 0002 and 0003

## Decision

Use Spawnt for the product, CLI, Rust packages, desktop shell, installers and website. The working site is `https://spawntv.vercel.app`. Use `SPAWNT_HOME` and `~/.spawnt` exclusively. There are no customers, so no compatibility executable or automatic home migration is required. Existing local directories are left untouched. The Rust daemon owns product logic; the Electron shell is UI and tray. Version desktop and CLI together in the 0.x series until v1.0.

The private source repository is `mwahaj36/spawnt`. The separate `mwahaj36/spawnt-releases` repository remains private until launch. The owner reserved `spawnt@0.0.1` and created the npm organization.

## Consequences

Every active build, installer, package and download reference must use the sole product identity. Retire old unpublished artifacts without relabeling their contents. Legal review and trademark clearance remain outstanding. Do not claim the new domain or package-manager names are available until verified.
