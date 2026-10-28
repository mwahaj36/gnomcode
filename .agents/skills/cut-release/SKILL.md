---
name: cut-release
description: >-
  End-to-end automated release procedure for Spawnt: lockstep version bumping across
  all crates, web app, npm shims, installers, and download portals, followed by binary
  compilation, packaging, and tagging.
---

# Cut a Spawnt Release

Follow this runbook when cutting any release (`docs/SPEC.md` §6).

## Automated Version Synchronization

Spawnt maintains synchronized versions across 8 files in the repository. Instead of manual editing, run the automated version bumper:

```sh
# Explicit version:
node .agents/skills/cut-release/scripts/bump-version.mjs 1.1.2

# Or automatic semver increments:
node .agents/skills/cut-release/scripts/bump-version.mjs patch
node .agents/skills/cut-release/scripts/bump-version.mjs minor
node .agents/skills/cut-release/scripts/bump-version.mjs major
```

### Files Synchronized Automatically:
| File | What Gets Updated |
| :--- | :--- |
| `Cargo.toml` | `[workspace.package] version = "X.Y.Z"` |
| `scripts/Setup.cs` | `AssemblyVersion("X.Y.Z.0")`, `AssemblyFileVersion`, GUI badge `vX.Y.Z`, Windows Add/Remove Programs `DisplayVersion` |
| `web/package.json` | `"version": "X.Y.Z"` |
| `npm/package.json` | `"version": "X.Y.Z"` |
| `npm/spawnt/package.json` | `"version": "X.Y.Z"` |
| `scripts/install.ps1` | `Spawnt-vX.Y.Z-windows-$Arch.zip` download URL and installation completion banner |
| `web/public/downloads/install.ps1` | Direct mirror of `scripts/install.ps1` |
| `web/app/downloads/page.tsx` | UI badge `Production Release • vX.Y.Z` and zip download href |
| `apps/tray/src-tauri/tauri.conf.json` | `"version": "X.Y.Z"` (when present) |

---

## Automated Packaging Pipeline (Windows)

To compile release binaries, build the standalone installer with uninstaller, package the release zip, and verify the web build:

```powershell
powershell -ExecutionPolicy Bypass -File .agents/skills/cut-release/scripts/package-release.ps1
```

Or via npm:
```sh
npm run --prefix web release-pack
```

This runs:
1. `cargo.exe build -p spawnt --release` (native 64-bit binary).
2. Copies `target/release/spawnt.exe` -> `web/public/downloads/spawnt.exe`.
3. `csc.exe` compiles `scripts/Setup.cs` -> `web/public/downloads/Spawnt-Setup.exe` with embedded payload, 0.65x icon, and Windows Add or Remove Programs registration.
4. `Compress-Archive` generates `web/public/downloads/Spawnt-vX.Y.Z-windows-x64.zip`.
5. `npm run build --prefix web` rebuilds Next.js static assets with the updated download routes.

---

## Release Checklist

1. **Preconditions**:
   - Clean working tree (`git status`).
   - Run the [`rust-quality-gate`](../rust-quality-gate/SKILL.md) skill (`cargo test --workspace`, `cargo fmt --check`).

2. **Bump & Package**:
   ```sh
   node .agents/skills/cut-release/scripts/bump-version.mjs <new-version>
   powershell -ExecutionPolicy Bypass -File .agents/skills/cut-release/scripts/package-release.ps1
   ```

3. **Changelog & Notices**:
   - Update `CHANGELOG.md` with Conventional Commits.
   - Bundle third-party notices (`cargo about generate about.hbs > dist/THIRD_PARTY_NOTICES.html`).

4. **Commit & Tag**:
   ```sh
   git commit -am "chore(release): vX.Y.Z"
   git tag -a vX.Y.Z -m "Spawnt vX.Y.Z"
   git push origin main --follow-tags
   ```

5. **Post-Release Verification**:
   - Smoke-test installer: `web/public/downloads/Spawnt-Setup.exe`.
   - Verify Windows Settings -> "Installed apps" displays `Spawnt Mission Control vX.Y.Z`.
   - Test `Spawnt-Uninstall.exe` cleanly purges services, registry entries, and shortcuts.
