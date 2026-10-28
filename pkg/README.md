# Package-manager manifests

Pre-release stubs for Homebrew, Scoop, winget, and AUR. Replace `PLACEHOLDER` values with real release URLs and checksums from the first signed public release.

| File | Target registry | Future repo |
| :--- | :--- | :--- |
| `homebrew/spawnt.rb` | Homebrew tap | `mwahaj36/homebrew-tap` |
| `scoop/spawnt.json` | Scoop bucket | `mwahaj36/scoop-bucket` |
| `winget/spawnt.yaml` | `microsoft/winget-pkgs` | PR to winget community repo |
| `aur/PKGBUILD` | Arch User Repository | AUR `spawnt-bin` package |

**Do not submit any of these until:**
1. The `mwahaj36/spawnt-releases` repository is made public.
2. A signed v1.0 release exists with verified SHA-256 checksums.
3. Code-signing (Windows) and notarization (macOS) are complete.
4. Legal review of the EULA is done and the draft marking is removed.

## Verified name availability (2026-10-28)

- npm: `spawnt@0.0.1` is published; `@spawnt` org exists.
- Homebrew `formulae.brew.sh`: no existing `spawnt` formula.
- Scoop (ScoopInstaller/Main, ScoopInstaller/Extras, ScoopInstaller/Versions): no existing `spawnt` entry.
- winget (`microsoft/winget-pkgs`): no existing `mwahaj36.spawnt` or `spawnt` identifier.
- AUR (`aur.archlinux.org`): no existing `spawnt` or `spawnt-bin` package.

All four package-manager names appear available as of the date above.
