# Gnomcode 1.0.0 packaged-app smoke checklist

Prepared 2026-11-13 by codex-release for Claude's release review. This is a procedure, not a passed smoke test. Leave each result pending until exercised against the exact final artifact. Do not publish, create tags/releases or change deployments without Wahaj's sign-off.

## Test environment and evidence

Use a disposable Windows account or VM with a snapshot, not Wahaj's existing installed app. The current installer changes shared per-user uninstall registration, shortcuts, PATH, autostart and Explorer entries even if another destination folder is chosen. A test directory alone does not isolate those effects. Use an owner-approved disposable repository and synthetic tasks only. Account/credential steps belong to Wahaj; never enter credentials or record tokens. Physical iPhone Safari and Android Chrome acceptance is separate from browser emulation.

Record candidate source SHA, merged PRs, exact artifact path, byte size, SHA-256, Windows build/architecture, Electron version, agent CLI versions, public-key fingerprint, OS-signature status and reviewer/date. Capture only sanitized outcomes; do not save real conversation/tool content. Keep at least 20 GB free. Preserve the user's agent-owned history and real configuration directories.

## 1. Artifact identity and clean installation

- [ ] In PowerShell, run `Get-FileHash -Algorithm SHA256 -LiteralPath <installer-path>` and compare with the reviewed SHA256SUMS entry. Record the actual digest and size, not placeholders.
- [ ] Run `Get-AuthenticodeSignature -LiteralPath <installer-path>`. Record Signed/NotSigned and signer details; update signatures and Windows Authenticode are separate checks. An unsigned launch needs explicit owner approval.
- [ ] In the disposable account/VM, run the installer interactively. Review destination, EULA and every integration option. Record selected options. Do not invent an unattended destination flag; current Setup.cs recognizes preview/quiet/uninstall options but has no CLI destination option.
- [ ] Verify Installed apps reports 1.0.0, then verify the installed CLI with `& <installed-gnomcode.exe> --version`. Confirm the packaged desktop, native engine, EULA and third-party notices are present. Do not substitute an older engine.
- [ ] Record shortcut targets and application identity. Launch the desktop through its installed shortcut. Confirm a single window, working four-place console and responsive tray. Launch the shortcut again and confirm one instance activates.

## 2. Detection and sessions

- [ ] With each owner-authenticated supported CLI available, record `claude --version`, `codex --version` and `agy --version`; mark absent CLIs unavailable rather than fabricating results. The supported set is exactly Claude Code, Codex and Antigravity.
- [ ] Confirm detection in the console and doctor. Check missing CLI behavior using a disposable environment, without renaming or deleting a real installation.
- [ ] Launch one synthetic task in a disposable repo for each available agent. Accept only that provider's normal native trust/permission dialogs. Confirm reply, local history, working terminal and idle transition. Never add permission-skipping flags.
- [ ] Continue an imported synthetic conversation, switch between two eligible providers and inspect the observed target model and reply. Record missing account/model entitlement as a gap. Restart the desktop and confirm history continuity without duplicate turns.

## 3. Limits and tray

- [ ] Open Usage. Compare each visible limit/reset with the supported native source at the same time. Record exact/api/estimated/unknown trust and freshness; unavailable sources must stay unavailable. Do not exhaust paid quota merely to complete this test.
- [ ] Open the tray context menu. Check Needs you/Running groups, provider submenus, Open limits, New session, Open Gnomcode and Quit Gnomcode. Confirm each navigation reaches its intended place. Missing data should say Limits unavailable or Daemon unavailable while Open/Quit remain usable.
- [ ] Confirm zero-session, unavailable-limit and long-title states remain readable. Check keyboard navigation and system scaling at 100% and 150%.

## 4. Remote pairing and revocation

- [ ] With Wahaj performing account steps, enable native Remote consent explicitly. Pair a disposable phone through the normal Devices flow. Record phone/browser versions and verified session identity; do not save pairing secrets or bearer tokens.
- [ ] Read a synthetic conversation and send a short follow-up from the phone. Confirm one delivery and reply. Reload/reconnect and background/foreground the phone; confirm drafts, conversation identity and encrypted reconnect behavior.
- [ ] Disable/revoke the disposable device or native app through its actual controls. Confirm the phone loses access and heartbeat/restart does not silently re-enable native consent. Re-enabling requires the native confirmation again.
- [ ] Exercise second-account denial only with owner-provided disposable accounts. Missing accounts and independent security review remain open gates.

## 5. Approvals

- [ ] In the disposable account only, inspect `gnomcode integrate --help`, then run `gnomcode integrate claude` in the disposable configuration. Confirm backup/remove behavior from the help and existing integration docs before changing configuration.
- [ ] Trigger a harmless synthetic native permission request. Approve one exact request in the console, deny another from the paired phone and confirm only the selected request is answered.
- [ ] Disconnect the phone/close the console and repeat. Timeout, daemon failure and an unanswered request must return control to native approval and must never default to allow.
- [ ] If testing Codex or Antigravity protocol approvals, inspect `gnomcode protocol-approvals --help`, run `gnomcode protocol-approvals enable codex` or `gnomcode protocol-approvals enable antigravity` in the disposable setup and launch a new session. Restore the tested agent with the corresponding `disable` command afterward. Confirm normal permission policy, exact request matching and cancel/no-decision behavior. Default terminal launches must remain available. Record actual CLI/protocol versions.

## 6. Notifications

- [ ] Enable the relevant notification event in Settings and permit Windows notifications for Gnomcode. Trigger a harmless approval wait, then complete a synthetic turn longer than the configured completion threshold (default strictly over two minutes). Avoid tool-output/private-content captures.
- [ ] Confirm Gnomcode attribution, redacted title/agent/event only, click activation of the correct conversation and no duplicate alert for the same event. Inspect Notification Center after banner timeout.
- [ ] Disable one event and repeat its trigger. Confirm suppression. Check quota-low/reset notifications only with safe available evidence; otherwise record unverified.

## 7. Auto-update authenticity, offline use and restart

Use a disposable VM snapshot and a reviewed previous build whose embedded public key matches the final candidate's signing key. The candidate must have a strictly newer real version. Do not relabel an old installer or advance the public channel for this test. Wahaj supplies release signing outside chat; follow docs/release-signing.md.

- [ ] Make a private test copy of the reviewed manifest/artifact directory. In that copy only, set the win32-x64 asset URL to `http://127.0.0.1:41892/Gnomcode-Setup.exe`, preserving actual version/hash/signature. The signature covers artifact bytes. Never change the public release manifest for this test.
- [ ] Run `python -m http.server 41892 --bind 127.0.0.1 --directory <reviewed-test-directory>` (Python 3 help verified during preparation). Before launching the disposable desktop/daemon, set `$env:GNOMCODE_UPDATE_MANIFEST_URL='http://127.0.0.1:41892/latest.json'` in its PowerShell environment. Serve only the reviewed test directory; stop the server after the test.
- [ ] Unset GNOMCODE_UPDATE_PUBLIC_KEY at runtime while retaining the prior build's embedded key. In Settings -> Desktop updates, turn automatic checks on and use the actual Check now control. Confirm valid SHA-256 **and** Ed25519 signature produce ready status and the tray shows Restart to update. Running agents must not restart during download.
- [ ] Repeat from a clean VM snapshot with missing signature, altered signature, wrong signer, corrupted bytes, malformed manifest and an unreachable server. Each must refuse installation and keep the app/agents usable. Also test a previously staged tampered file. Runtime blank/replacement public values must not bypass the pinned key.
- [ ] Verify an equal/older manifest cannot downgrade the app. Disable automatic checks and confirm no staging occurs. Record observed network destinations; no analytics traffic is permitted.
- [ ] Restore the valid candidate. First test explicit Restart to update; separately restore the VM and test explicit Quit Gnomcode with a ready update. Confirm exactly one installer launches, the new version starts and local synthetic history remains available. Check no installer launches when status is not ready.
- [ ] Unplug networking and relaunch the installed app. Local history and local operations remain usable. Record update errors as nonfatal. Restore the VM snapshot to validate rollback without inventing an automatic rollback feature.

## 8. Doctor, restart and uninstall

- [ ] Run `& <installed-gnomcode.exe> doctor --json` from the synthetic repository. Save a sanitized report. Inspect agent_clis, claude_signin, limit_sources, claude_integration, daemon, disk_space and keep_awake check results; record actual identifiers from output if they differ. A warning is not a passing provider check; failure exits nonzero.
- [ ] Run `& <installed-gnomcode.exe> doctor --network` and review the disclosed endpoints against shipping behavior. Do not paste credentials or local bearer metadata into the report.
- [ ] Restart the disposable Windows VM and verify only the chosen autostart behavior. Check launcher/tray recovery and absence of duplicate managed delivery.
- [ ] Uninstall through Installed apps in the disposable environment. Confirm owned integration files/registration/shortcuts are removed and agent-owned histories remain. Record any remaining local Gnomcode data with the documented retention policy; do not delete user history as cleanup.

## Acceptance record

For each section record: pass/fail/blocked, reviewer/date, exact build, sanitized evidence and follow-up PR. Keep physical-phone, macOS/Linux, owner-key signing, OS trust, independent Remote review and legal approval pending until evidence exists. A completed procedure document does not close the packaged-app smoke roadmap gate.

## Preparation verification and decisions

Procedure checked against source at 17267e5 and real 1.0.0 CLI doctor/integrate/protocol-approvals help output: CLI Doctor/Daemon flags, desktop tray/update launch handling, Rust update staging, Settings update controls and Setup.cs integration scope. No installer, login, provider operation, update application or uninstall was executed while preparing this file. The current account is unsuitable for a safely isolated install. All results above remain pending for Claude.

Preparation gate: git diff --check, CARGO_INCREMENTAL=0 cargo fmt --all, workspace clippy with -D warnings and 499 workspace tests passed (dev/test debug symbols disabled). No Cargo.lock/dependency changes. U+00C2 diff count 0; checklist is UTF-8 without BOM, LF. These are source/preparation checks, not executed packaged acceptance. The procedure was rebased onto 3943dbb and the Rust fmt/clippy/499-test gate passed again. Python http.server bind/directory options were verified with real help output; the updater test server was not started during preparation.
