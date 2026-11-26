# Publishing the npm release

GitHub releases, website deployment and npm publication are separate steps. The launcher pins seven optional platform packages to the same version: Windows x64/ARM64, macOS x64/ARM64, Linux x64/ARM64, and Linux x64 musl.

Stage genuine release binaries with `node npm/stage-release.cjs <artifacts> LICENSE <notices>`, then run `node npm/check-release.cjs` and `node npm/smoke-release.cjs`. Do not relabel older executables. The extra Windows ARM64 and static musl builds are produced by `.github/workflows/npm-release-build.yml`; ARM64 PE architecture and absence of redistributable runtime DLL imports are checked, while musl runs the packaged smoke suite. Windows ARM64 uses `+crt-static` so the CLI does not require a separately installed Visual C++ runtime.

When packing on Windows, verify native Unix archive entries have mode `0755`. Publish the prepared `.tgz` files rather than repacking Linux/macOS platform directories on Windows; that can lose executable permissions.

From an authenticated npm terminal, publish all seven `@gnomcode/cli-*` package tarballs with `npm publish <tarball> --access public`, then publish the `gnomcode` launcher tarball last. Use the prepared `Publish-npm.ps1` alongside the verified release tarballs when available. It stops on the first failure; complete any npm authentication/2FA prompts yourself. The agent prepares and verifies packages but does not publish them without an explicit request.

After publishing, verify `npm view gnomcode version` reports the new version, then test `npm install -g gnomcode@<version>` and `gnomcode --version`. OS code signing and Ed25519 automatic-update signing are separate from npm's registry integrity checks.
