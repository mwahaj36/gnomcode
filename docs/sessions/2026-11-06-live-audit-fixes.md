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

## Installed acceptance and notification follow-up

The verified 0.9.1 installer completed after unlock. The running engine reported 0.9.1 from the packaged Electron resources; Windows registered the Gnomcode display name. Three repeated launches retained exactly one application window. Real imported history rejected prompt submission with HTTP 403. The dashboard reported one ongoing session; default imported history had 35 rows and explicit subagent inclusion had 48, with no imported chat controls.

A disposable Codex startup rejected the first prompt with HTTP 409, preserving it rather than losing its prefix. The official CLI displayed its own optional update and folder-trust setup; no task was submitted during setup. These native decisions must not be mistaken for a ready agent.

Notification acceptance revealed a second-instance argument bug: a real Electron probe reordered Chromium switches between the notification flag and body. Parsing the reordered arguments produced a Chromium switch as the notification text. The follow-up uses validated original argv through Electron additionalData. Regression tests also retain Windows notification objects across timedOut events so garbage collection does not remove them from Notification Center, and repair the existing app shortcut AUMID without changing an unrelated shortcut. This follow-up is released as 0.9.2 rather than replacing the published 0.9.1 artifacts.

The live Codex acceptance continued in the already-trusted workspace: the complete prompt was acknowledged and returned the exact disposable marker. Switching from gpt-6-luna to gpt-5.6-luna left the conversation idle and the second model recalled the marker without replaying the original task. The disposable host was stopped afterward. The full Rust gate passed for 0.9.2, along with 25 desktop tests, 26 web tests and web typecheck. Running the gate while a real hosted chat existed exposed a test isolation bug: the handoff fixture discovered the user's live hosts. It now runs in a subprocess with a temporary GNOMCODE_HOME, matching the existing pairing-test isolation pattern.

## Final 0.9.2 deployed acceptance — 2026-11-07

- Published the immutable binary-only v0.9.2 release and promoted deployment `dpl_EdA1hMRR6fHU8uvCmgUv9m5woVfY`. A transient first deployment authorization error cleared on retry with the existing authenticated CLI; no permissions or credentials were changed.
- Downloaded the installer from the production website. Its 182,404,096 bytes matched SHA-256 `dad0f752d37d1485792ad048fda6f0497e08bee6e51deb4cafbc552386ef2d19` in the deployed manifest and GitHub asset digest. The normal installer completed, and the running packaged daemon's health endpoint reports 0.9.2.
- Verified the existing Start Menu shortcut now contains AUMID `app.gnomcode.desktop` and a registered toast activator CLSID. A native second-instance test delivered the expected notification body. Read-only queries restricted to Gnomcode's Windows notification handler found one notification, still present after banner timeout, with the exact disposable test body. This verifies Windows Notification Center persistence without relying on a server acknowledgment or changing DND settings.
- Three additional launches retained one visible Electron application window. The real dashboard showed one ongoing session, and automation guidance and rule controls were visible in the installed app.
- The production build, all required local checks and installer checks passed. No macOS/Linux native display test or phone display test is claimed. Phone delivery still requires a registered, opted-in phone browser; the last authenticated production send reported zero destinations. Provider state and filtering regressions cover all adapters, while live model-switch continuity was exercised with the installed Codex provider. Claude/Gemini were unavailable locally and Antigravity's provider account limitations prevent claiming full live acceptance there.
