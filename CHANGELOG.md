# Changelog

All notable changes to FleetHUD are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [1.5.2] — 2026-10-28

- Restore the Windows running-count badge; use a red Windows mark on a black canvas and readable tray tooltips.
- Lead both console Overviews with ongoing-session identity beside provider limits. Keep history in Sessions.
- Show available cost estimates across summaries, intervals, providers, models, sessions and projects; explicitly label partial pricing coverage.
- Show Today as hourly usage since local midnight, add calendar month, and label the period next to token totals.
- Add project-card navigation and a native project filter before pagination; web session names remain scoped to their signed-in account.
- Reduce repeated trust tags and tidy Account, active-license and export controls.
- Rebuild the desktop Account screen with clear connection/publication states, a numbered pairing flow and duplicate-write protection without page reloads.
- Rebuild Windows x64 and Linux x64/ARM64 artifacts. macOS remains deferred.

## [1.5.1] — 2026-10-28

### Fixed
- Keep the landing Overview tour compact: three summary metrics, seven short chart bars and three agent rows. Detailed sessions remain in the Sessions tab.
- Preserve local-console authorization when switching views or following the brand link, so reloading does not lose access.
- Resolve failed report loading states and show actionable authorization recovery guidance for local reads.
- Initialize native response caches and clear them after failed reads so healthy responses and retries render correctly.
- Rebuild Windows x64 and Linux x64/ARM64 packages and synchronize website/install versions. macOS builds remain deferred.

## [1.5.0] — 2026-10-28

### Changed
- Applied spawnt branding, theme tokens, self-hosted fonts, canonical icons and balanced square controls across marketing, account, settings, both consoles and Windows setup/tray.
- Shared console summary, daily activity grid/chart, provider/model breakdown, quota panels and session details while retaining separate local/cloud authentication and data.
- Kept Settings in the console shell and replaced generic landing tour content with consistent illustrative samples using completed presenters.

### Fixed
- Active context follows compaction rather than cumulative lifetime usage; latest context regressions cover every provider.
- Quota provenance, desktop-generated snapshot freshness, past-reset messaging and cloud publication failures are explicit.
- Rust-produced daily/project/session totals count cached tokens once; local project identity stays local.
- Reload and settings writes report actual success/failure, prevent concurrent requests and preserve polling state/focus.
- Escaped local metadata/action arguments; moved report/project queries off async workers; integrated accessible clipboard/check controls.

### Release artifacts and limits
- Rebuilt Windows x64 setup/archive and Linux x64/ARM64 archives with canonical icons, font/dependency notices and SHA-256 checksums. Updater tests reject corruption and preserve data.
- macOS packaging source includes Intel/Apple Silicon browser-console app bundles, but native builds are blocked by GitHub account billing. No macOS binary, notarization or Unix native tray is claimed.
- Final browser visual parity checks and fresh cloud publication verification remain pending browser access and desktop re-pairing. The user authorized release after available verification, with macOS builds deferred and unavailable visual/live checks disclosed.
## [1.4.1] - 2026-10-27

- Withdraw hosted cloud-session tracking, provider runtime detection, automatic collectors, usage-file imports and the new browser pairing flow.
- Remove cloud-presence fetching, session merging, tray sections and the /pair screen. Preserve older desktop sign-in, manual desktop pairing and desktop summary sync.
- Preserve console loading/error fixes, pricing fixes, private credential handling, daemon locks and verified installers.
- Ignore legacy local cloud-presence rows without deleting local history. Include a repeatable SQL cleanup for retired backend endpoints and tables that preserves desktop sync and Auth.
- Publish replacement Windows x64 and Linux x86_64/ARM64 downloads.

## [1.4.0] - 2026-10-27

- Add explanatory browser pairing for cloud workspaces and desktop devices with an independent, encrypted, single-use account login.
- Fix embedded console JavaScript failing before any session or quota data could load; display authorization and connection errors.
- Show remote cloud sessions in native consoles and a separate connected-cloud section in the Windows tray.
- Keep cloud publishers from overwriting desktop summaries; serialize credential writes and enforce private atomic persistence.
- Enforce daemon file locking on Unix and preserve Windows lock behavior; recover unavailable tray connections visibly.
- Replace raw credential-copy pairing controls and unsafe callback redirects with the new approval flow.
- Update Windows x64 and Linux x86_64/ARM64 packages and checksum manifests. macOS packages still require a native build runner.

## [1.3.0] — 2026-10-27

### Added
- Automatic cloud session collection with existing account pairing and explicit identities for all supported agents.
- Metadata-only measured-usage imports, invocation deduplication and estimated API-equivalent costs.
- Account-isolated cloud session storage and dashboard visibility with honest unknown-usage states.
- Windows x64 and Linux x86_64/ARM64 release archives with third-party notices and SHA-256 manifests.

### Fixed
- Patched ctutils vulnerability RUSTSEC-2026-0385 and repaired cargo-deny configuration (Rust 1.87 minimum).
- Unix installer now verifies, extracts and installs its package instead of reporting success after only downloading it.
- Windows PowerShell updater verifies its archive and native installer version before launching an upgrade.
- Installer packaging validates the embedded engine and notices, preserves account data, and synchronizes release versions.

### Availability
- macOS packages and native macOS execution remain pending a working build runner. GitHub Actions is currently blocked by account billing/spending limits.
- npm publication and .deb/.rpm/AppImage packages are not part of this release; download labels reflect actual availability.

## [Unreleased]

### Added
- Initial Rust Cargo workspace (`fleethud-core`, `fleethud-adapters`, `fleethud-store`, `fleethud-platform`, `fleethud-ingest`, `fleethud-daemon`, `fleethud` CLI).
- Single source of truth specification (`docs/SPEC.md`), engineering rules (`AGENTS.md`), and Architecture Decision Records (`docs/adr/0001`–`0003`).
- Workspace engineering skills (`.agents/skills/`): `ponytail`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `rust-best-practices`, `tauri-v2`, `supabase-postgres-best-practices`, `rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`.
- Binary and UI anti-reverse-engineering hardening (`Cargo.toml` release profile, `.cargo/config.toml` path remapping).
- GitHub Actions CI pipeline across Linux, Windows, and macOS (`.github/workflows/ci.yml`).
