# Changelog

## Unreleased

- Relicense Gnomcode under the Apache License 2.0: free and open source with no paid tier, Remote included. Remove the closed-source string obfuscation, the Pro/Remote license gate and the EULA; add CONTRIBUTING (DCO), TRADEMARKS, CODE_OF_CONDUCT.
- Route the Remote relay and Supabase defaults through the single branding constant; swap the personal email contact for GitHub Issues.
- Let the quota-triggered context relay hand off to Antigravity, not just Claude Code and Codex.
- Scope 1.0 to Claude Code, Codex and Antigravity: remove the Gemini CLI adapter and every Gemini CLI launch, resume, limits and model-catalog path, and remove the Teams tier and its feature toggle. Existing rows from removed agents stay stored and show as unsupported. Antigravity keeps using Gemini models.

## 0.9.4 — 2026-11-12

- Read every Claude Code, Codex and Antigravity surface: Codex archived and compressed (`.jsonl.zst`) sessions, the legacy Antigravity IDE history folder, and Claude subagents as linked child sessions. Fix Codex double-counted turns, forked-session identity and inverted Antigravity subagent links.
- Show limits for all three providers: Codex from its logs; Claude Code from its status line or, opt-in, Anthropic's usage endpoint with Claude Code's own sign-in; Antigravity pooled 5-hour and weekly limits (including Gemini Argon) from its running language server. Percent left, reset countdowns, exhausted and reset-passed states.
- Detect Claude Code bundled with the Claude desktop app; label desktop-app sessions; stop showing stale sessions as Running and make tray, summary and list counts agree.
- Cut a steady-state ingest pass from about 1.3 s to about 75 ms on large Antigravity histories; keep the Gnomcode icon on the taskbar and notification shortcut; stop the nav rail covering content.
- Remove the Cline/Roo, Cursor and Aider adapters.

## 0.9.3 — 2026-11-07

- Simplify application pages with system typography, consistent spacing, aligned responsive forms and compact checkboxes.
- Replace repeated trust badges and uncertainty warnings with plain labels and accessible source details; preserve estimated, unknown and stale reporting.
- Honor hidden UI regions, simplify empty states and disclose activity breakdowns, secondary histories and setup explanations on demand.

## 0.9.2 — 2026-11-07

- Preserve original second-instance activation arguments so Chromium switches cannot replace notification text or workspace paths.
- Retain Windows toasts after banner timeout and repair the installed app shortcut identity for Notification Center.


## 0.9.1 — 2026-11-06

- Ship the Electron Windows shell and use one app instance for tray, shortcuts and workspace activation. Attribute native notifications to Gnomcode.
- Fix false completion and model-change states; correct Claude new-turn and Gemini tool-completion states.
- Separate owned conversations from imported view-only history, hide subagents by default, filter legacy tool IDs and paginate after filtering.
- Retire context-fill rotations in favor of native provider compaction. Add usable guardrail setup and clearer reconnection guidance.
- Preserve drafts and confirmed model selection on errors, guard cold startup input and render interactive terminals with xterm.
- Add opt-in authenticated background device notifications with owner isolation and vendor destination validation. Publish metadata and Rust-computed ROI per desktop.
- Publish Windows binaries separately from private source; retain stable website download URLs and checksum verification.


All notable changes to Gnomcode are documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and [Semantic Versioning](https://semver.org/).

## [0.9.0 development refresh] — 2026-11-01

- Confirm account-wide access before native linking; signed-in devices then connect automatically to all linked native apps without phone pairing or per-desktop approval.
- Preserve account consent across updates and inactivity, keep account-side disable effective across restarts, and bound reconnect recovery while preserving drafts without replaying prompts.
- Publish the matching Windows x64 development installer and account UI under the owner's scoped rollout approval. Physical-phone/platform acceptance and independent Remote security review remain open.

## [1.5.3] — 2026-10-28

- Order Overview as activity, compact spend summary, then aligned provider-limit and ongoing-session columns. Group quota windows per provider and remove duplicate chart cost.
- Apply shared presence rules to all agents: two-minute idle grace, retained approvals/open execution, inactive stale observations and explicit-only ended status. Show ongoing sessions and last activity on Sessions.
- Repair session-window focus, restore minimized Windows windows, report actual outcomes and offer workspace copying when unavailable.
- Keep the tray mark full size while insetting the Windows app icon to 0.65x; simplify tray tooltip wording.
- Simplify console navigation and reporting controls, move Docs/Pricing into Settings, and remove correction prose from public copy.
- Preserve protected owner-only Windows credential ACLs without requesting audit privileges.
- Rebuild Windows x64 setup/archive and Linux x64/ARM64 archives. macOS remains deferred.

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
- Applied gnomcode branding, theme tokens, self-hosted fonts, canonical icons and balanced square controls across marketing, account, settings, both consoles and Windows setup/tray.
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
- Initial Rust Cargo workspace (`gnomcode-core`, `gnomcode-adapters`, `gnomcode-store`, `gnomcode-platform`, `gnomcode-ingest`, `gnomcode-daemon`, `gnomcode` CLI).
- Single source of truth specification (`docs/SPEC.md`), and engineering rules (`AGENTS.md`).
- Workspace engineering skills (`.agents/skills/`): `ponytail`, `test-driven-development`, `systematic-debugging`, `verification-before-completion`, `rust-best-practices`, `tauri-v2`, `supabase-postgres-best-practices`, `rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`.
- Binary and UI anti-reverse-engineering hardening (`Cargo.toml` release profile, `.cargo/config.toml` path remapping).
- GitHub Actions CI pipeline across Linux, Windows, and macOS (`.github/workflows/ci.yml`).

## 1.0.0 release candidate - 2026-11-13

- Support exactly Claude Code, Codex and Antigravity, preserving removed-agent history as unsupported.
- Add the four-place console, provider limits and usage views, isolated session worktrees, Continue here and handoff quality reports.
- Add bounded workspace files, diffs, artifacts, attachments, quick prompts and state-gated interruption in the console and encrypted Remote.
- Add native Claude approvals and opt-in Codex/Antigravity protocol approval bridges, phone app lock, redacted notifications, desktop update staging and doctor diagnostics.
- Improve quota exhaustion handling, Antigravity ingestion/resume, Claude desktop/subagent visibility, idle watcher efficiency and process recovery.
- Synchronize CLI, desktop, installers, web and npm package versions to 1.0.0. Publication and packaged acceptance remain pending. See [full release notes](docs/release-notes/1.0.0.md).
