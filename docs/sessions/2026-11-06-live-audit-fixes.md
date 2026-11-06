# Live audit fixes — 2026-11-06

## Goals

Resolve the production audit across providers; restrict control to Gnomcode-owned conversations; repair session discovery, notifications, automation setup and the shipped Windows shell. Package, deploy, install and verify the update against the signed-in account.

## Findings and fixes in progress

- Codex usage events incorrectly set idle, producing false completion alerts. Settings events incorrectly started a turn.
- Legacy Codex tool IDs created phantom sessions. Preserve records but exclude them from session views and counts.
- Conversation pagination and filters now run together in Rust. Imported sessions are view-only and subagents are optional.
- Native metadata repair preserves usage, checkpoints and deleted chat markers.
- Context compaction belongs to providers; retire pending automatic rotations and collapse history.
- Add guardrail setup and explain quota reconnection accurately.
- Service-worker readiness could hang desktop notifications. Add fallback and delivery feedback.
- Package Electron and route launcher/tray activation through its single-instance lock. Native alerts use Gnomcode attribution.
- Add opt-in authenticated background push, owner RLS and vendor endpoint validation; no chat payloads.
- Store metadata snapshots per desktop and publish Rust-computed ROI by timeline.

## Verification

Targeted red/green tests reproduced token-count false completion, settings state regression, history truncation, service-worker hang, missing guardrail setup, stale rotations, metadata repair and background-delivery retry state. The full Rust gate passed (fmt, clippy with warnings denied, workspace tests, cargo-deny and core purity). Web checks passed 26 tests; Electron checks passed 21 tests including a real binary terminal roundtrip. The Windows installer passed payload/version, archive path and unrelated-file preservation checks. The additive notification/desktop schema passed transactional owner-isolation tests and was applied to production. Deployed installation and live acceptance remain pending. Production notification display must be verified on actual devices; do not claim a server acknowledgment proves display.

## Cross-platform review

Rust filesystem work still uses platform paths and existing shared-read ingestion. Notification shell discovery is Windows-only; macOS/Linux keep their native fallback. Process cleanup targets only Gnomcode executables under the selected installation, not provider IDEs. Archive extraction rejects traversal, alternate streams and linked paths; uninstall removes shipped files without recursively deleting the install directory. Linux tray retains its Open menu fallback. No permission-escalation flags were added. Native macOS/Linux acceptance is not available on this laptop.

## Distribution

Created the public binary-only `mwahaj36/gnomcode-releases` repository because the Electron installer exceeds the website's static-asset limit. Private source is not published there. Website redirects allow only the current named release assets; binaries are no longer tracked in the source checkout. Version is synchronized to 0.9.1.

## Deployed acceptance — 2026-11-07 local time

- Published the binary-only 0.9.1 development release and promoted Vercel deployment `dpl_EYRmH3qfuJWtxX5wcS2sDAHSWwrD` to `https://gnomcode.vercel.app`.
- Downloaded the installer through the deployed website URL. Its 182,401,536 bytes matched SHA-256 `67f008b149e0a2227045156ed7173e2f4ec6643689ee17d0c3de7f456948f340`, matching both the website manifest and GitHub asset digest.
- Production push endpoints reject unauthenticated requests (401), arbitrary send destinations (400) and unsupported registration destinations (400). Authenticated generic send returns 200 with zero devices: there is no registered phone destination yet. Configuration exists; notification display has not been verified.
- Saved a consistent SQLite backup in ignored local acceptance artifacts before installing. No real history was deleted.
- The installer was opened in its normal installation flow. `--quiet` applies only to uninstall and did not initiate an installation; that idle attempt was stopped. The verified installer is waiting for Install.
- Native acceptance is blocked by the Windows lock screen. Computer Use input stopped and the user was asked to unlock. The running installed engine remains 0.9.0; do not claim installation, tray/window reuse or Gnomcode toast attribution has been accepted yet. PR #140 remains a draft until this acceptance is completed.
