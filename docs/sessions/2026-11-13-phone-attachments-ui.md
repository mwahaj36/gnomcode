# 2026-11-13 - Phone and console attachments UI

## Goals

Enable users on phone (Remote) and desktop (local console) to attach screenshots and small text/log files up to 15 MiB directly into a session's workspace and reference them cleanly when sending a prompt.

## Outcomes

- Added `MAX_ATTACHMENT_BYTES`, `validateAttachmentCandidate`, `renderAttachmentPills`, and `formatPromptWithAttachments` (with TypeScript signatures) in `web/lib/console-presentation.js` and `.d.ts`.
- Enforced client UX limits (<= 15 MiB, safe filename sanitization stripping traversal/`..`/control chars, allowlist for images/PDF/plain text/logs/JSON/CSV); Rust backend remains authoritative on the receiver side.
- Wired `#chat-attach-input`, `#chat-attach-btn`, `#chat-attachments-bar`, and `#chat-attach-status` into `crates/daemon/src/console.html` and `console-app.js` using `POST /api/v1/sessions/{id}/attachments` with calm unavailability notice when running against an older backend.
- Wired hidden file input, `.sp-attach-trigger`, `.sp-attachments-bar`, `{ type: 'upload_attachment', ... }`, `{ type: 'attachment_ack', ... }`, and calm `Unsupported remote command` handling into `web/components/RemoteCompanion.tsx`.
- Appended token-only CSS rules (`var(--sp-*)`, 44px mobile touch targets) to `web/app/gnomcode-console.css`.

## Verification

- `web/test/attachments-ui.test.mjs`, `web/` tests, `tsc`, Next build, `check-local-console.mjs`, headless Chromium visual audit at `390x844` and `1440x900` dark/light (plus composer `320x350` check), and `CARGO_INCREMENTAL=0` Rust workspace gate.
