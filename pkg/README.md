# Package-manager manifests

Pre-release stubs for Homebrew, Scoop, winget, and AUR. Replace `PLACEHOLDER` values with real release URLs and checksums from the first signed public release.

| File | Target registry | Future repo |
| :--- | :--- | :--- |
| `homebrew/gnomcode.rb` | Homebrew tap | `mwahaj36/homebrew-tap` |
| `scoop/gnomcode.json` | Scoop bucket | `mwahaj36/scoop-bucket` |
| `winget/gnomcode.yaml` | `microsoft/winget-pkgs` | PR to winget community repo |
| `aur/PKGBUILD` | Arch User Repository | AUR `gnomcode-bin` package |

**Do not submit any of these until:**
1. The `mwahaj36/gnomcode-releases` repository is made public.
2. A signed v1.0 release exists with verified SHA-256 checksums.
3. Code-signing (Windows) and notarization (macOS) are complete.
4. Release inputs use the Apache-2.0 LICENSE.

## Verified name availability (2026-10-28)

- npm: `gnomcode@0.0.1` is published; `@gnomcode` org exists.
- Homebrew `formulae.brew.sh`: no existing `gnomcode` formula.
- Scoop (ScoopInstaller/Main, ScoopInstaller/Extras, ScoopInstaller/Versions): no existing `gnomcode` entry.
- winget (`microsoft/winget-pkgs`): no existing `mwahaj36.gnomcode` or `gnomcode` identifier.
- AUR (`aur.archlinux.org`): no existing `gnomcode` or `gnomcode-bin` package.

All four package-manager names appear available as of the date above.
