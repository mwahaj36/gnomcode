# 0026 — Adopt Gnomcode as the product identity

- **Status:** Accepted
- **Date:** 2026-11-02
- **Supersedes:** 0015 for current product identity and home resolution

## Context

The owner renamed Spawnt to Gnomcode and selected `https://gnomcode.vercel.app` as the working site. Existing local conversation history must remain accessible.

## Decision

Use Gnomcode in UI, legal copy, current documentation, CLI, Rust crates, npm manifests, desktop identity, installer names and generated assets. Use `gnomcode` for commands and package names and `GNOMCODE_` for environment variables.

Data-home precedence is a nonblank `GNOMCODE_HOME`, a nonblank legacy `SPAWNT_HOME`, an existing `~/.spawnt`, then `~/.gnomcode`. Reuse the old directory without moving or deleting history. The desktop shell and Windows launchers use the same order. The legacy directory wins over a new default directory to avoid silently hiding old history.

Keep actual GitHub repository/release URLs, the existing deployed Cloudflare Worker name/address, deployed `spawnt_desktops` and `spawnt_device_requests` table identifiers, and Remote cryptographic domain labels stable. These are compatibility identifiers, not public branding. Do not rewrite applied database migrations. Historical session records and ADRs retain their original names and facts.

## Consequences

A rename does not itself publish npm packages, rename external repositories, deploy the website or certify installers. Rebuild binaries under their new identities; never rename an old executable to pretend it is a Gnomcode build. Browser credentials are origin-scoped, so the new website origin requires signing in and reconnecting through account Devices.

## Alternatives considered

- Blindly rename storage, schema and crypto labels — hides history and breaks deployed clients.
- Keep the old public executable/package names — leaves the product rename incomplete.
