# Windows launch readiness acceptance

Date: 2026-11-04. Source inspected: `cedeca596fa08b93077b99d9b85dc509db14815a` (0.9.0, PR #128). Windows NT 10.0.26300, x64. The current downloads are the earlier PR #127 development artifacts, not a build of PR #128. This session does not approve a beta or public launch.

## Goal and outcome

Reconcile the launch checklist against shipped scheduling, auth, switching and download features; gather real and simulated evidence without modifying agent configuration or the existing installation. The checklist records the remaining owner actions and separates test levels.

## Findings and decisions

- Codex CLI 0.166.0, authenticated `gpt-6-sol`, returned a requested synthetic reply in a disposable directory with read-only sandbox. Antigravity CLI 1.5.0 returned the reply with `gemini-3.8-flash` requested; its catalog reports high/medium/low variants, so exact variant activation is not established. Neither direct CLI request tests Gnomcode handoff or approvals. No deliberate quota exhaustion was attempted.
- Supabase dashboard showed signup, email provider and confirmation enabled, custom SMTP disabled, and default Site URL still set to the old Fleethud domain. On explicit owner instruction, changed Site URL to `https://gnomcode.vercel.app` and verified it persisted after reload. The allow-list still contains the broad `https://*.vercel.app/**` entry; owner review and inbox tests remain required. No SMTP or allow-list settings changed.
- The Windows installer shares uninstall registration, shortcuts, user PATH and autostart across install directories. A separate folder is therefore not a fully isolated installation. No install, uninstall, reboot or existing-device revocation was performed. The owner offered this host, but a separate Windows account or VM protects the working setup more reliably.
- Two-account testing is blocked: the owner reports disposable accounts unavailable. Emulation was requested but cannot establish physical keyboard or background behavior. Independent review and owner/legal approval remain open.
- The first synthetic public-relay probe returned DNS `ENOTFOUND`; a subsequent OS lookup and retry succeeded. The transient failure is not sufficient evidence of a relay outage or recovery readiness.

## Verification

- `.agents/skills/rust-quality-gate/scripts/check.ps1`: passed formatting, clippy, workspace tests, cargo-deny and core purity. Cargo-deny allowed duplicate-version warnings.
- `npm test` in `web`: 20 passed; `npm run typecheck` passed. Node emitted existing module-type warnings.
- `node scripts/check-local-console.mjs`, `node scripts/check-console.mjs`, and `node scripts/check-native-navigation.mjs`: passed.
- `node scripts/check-agent-ide.mjs` with Playwright in a temporary module path: passed simulated interaction and responsive layout checks at 180–1440px. This is not physical-device acceptance.
- `node scripts/check-live-relay.mjs https://spawnt-remote-relay.mwahaj25.workers.dev`: passed synthetic encrypted peer traffic and replay rejection on retry.
- `Get-FileHash` matched each PR #127 Windows artifact to `web/public/downloads/SHA256SUMS`; `Get-AuthenticodeSignature` reported `NotSigned` for setup and CLI. No clean-machine installation was tested.

## Remaining work

See the [launch checklist](../LAUNCH_CHECKLIST.md) for per-gate status and next actions. Real Gnomcode model handoff, approval and quota recovery; physical iPhone/Android; inbox acceptance and production SMTP; Windows lifecycle; two-account RLS and revocation; independent security, legal and signing decisions; and operational recovery remain open.
