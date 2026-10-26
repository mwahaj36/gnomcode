---
name: cut-release
description: >-
  End-to-end release procedure for FleetHUD: version bump, changelog generation,
  EULA and third-party license notice bundling (cargo-about), tagging in the
  private source repo, and verifying binary publication to fleethud-releases,
  npm, Homebrew, Scoop, winget, and AUR.
---

# Cut a FleetHUD Release

Follow this runbook when cutting any `vX.Y.Z` release (`docs/SPEC.md` §6).

## Preconditions

1. Working tree is on `main` and clean (`git status`).
2. Run the [`rust-quality-gate`](../rust-quality-gate/SKILL.md) skill and confirm all checks pass.
3. Confirm `docs/SPEC.md` §5 roadmap statuses reflect what is shipping in this release.

## Steps

1. **Bump version in lockstep:**
   - Update `[workspace.package] version` in `Cargo.toml`.
   - Update `"version"` in `npm/fleethud/package.json` (and per-platform packages once added in M1).
   - Update `apps/tray/src-tauri/tauri.conf.json` (from M3 onward).

2. **Generate third-party notices & changelog:**
   - Generate `THIRD_PARTY_NOTICES.html` with `cargo about generate about.hbs > dist/THIRD_PARTY_NOTICES.html` and verify zero copyleft licenses (`GPL`/`LGPL`/`AGPL`).
   - Update `CHANGELOG.md` from Conventional Commits (`git-cliff -o CHANGELOG.md`).

3. **Commit and tag in the private repo (`mwahaj36/fleethud`):**
   ```sh
   git commit -am "chore(release): vX.Y.Z"
   git tag -a vX.Y.Z -m "FleetHUD vX.Y.Z"
   git push origin main --follow-tags
   ```

4. **Verify automated release pipeline (`docs/SPEC.md` §6.2):**
   - Confirm GitHub Actions builds all 7 CLI targets + Tauri tray bundles (M3+), signs binaries, bundles `EULA.txt` + `THIRD_PARTY_NOTICES.html`, and publishes to:
     - `mwahaj36/fleethud-releases` (binaries + `latest.json` + `SHA256SUMS`, zero source code)
     - npm (`fleethud` + `@fleethud/cli-*`)
     - `mwahaj36/homebrew-tap`, `mwahaj36/scoop-bucket`, `microsoft/winget-pkgs`, AUR `fleethud-bin`
   - Smoke-test `npx fleethud@X.Y.Z --version` after publish.
