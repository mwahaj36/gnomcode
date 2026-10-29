## 2026-10-27 — Native release v1.3.0

- Built a new Windows x64 engine and standalone installer containing the cloud collector. Verified PE/assembly version 1.3.0.0 and exact embedded engine/notices hashes. Native CLI version and collector operations passed under Wine 10; an actual Windows GUI upgrade/uninstall has not been exercised here.
- Built Linux x86_64 and ARM64 releases with static OpenSSL using updated Ubuntu 22.04 packages. Version checks passed on Linux and under ARM64 emulation; the ARM64 collector smoke check passed.
- Bundled third-party and native runtime notices, generated SHA-256 manifests and verified all archive payloads. Fixed Unix installation/extraction and tested that a corrupt manifest cannot replace an existing installation. Updated the PowerShell updater to verify the archive and native installer version before launching it.
- Synchronized release versions, public scripts, native payloads and download links. Removed claims for unavailable macOS, npm and .deb/.rpm/AppImage distributions. macOS artifacts remain blocked by the lack of a native runner and GitHub account billing limits.
- The full release Rust quality gate passed. Publication and Vercel deployment are performed through the release PR and GitHub release rather than relabeling an old binary.
