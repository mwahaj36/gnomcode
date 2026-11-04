# Windows downloads, email signup and laptop automation

Date: 2026-11-04

## Goals

Make the Windows development installer downloadable from the website; finish email signup, laptop-owned scheduling from desktop/phone, model/provider failover, reset reconnection and event notifications. Fix the narrow conversation header reported during verification.

## Outcomes and decisions

- Include the Windows installer, portable archive, standalone CLI and SHA-256 manifests under `web/public/downloads`, with an explicit checksum link and unsigned development-build label. Rebuild from this phase's source instead of relabeling an older binary.
- Verify live public Supabase Auth settings: signup enabled, email provider enabled, email confirmation required. Keep both confirmed-email and immediate-session signup paths; add resend, expired-link handling and confirmation-required sign-in recovery. No privileged frontend key or blanket verification bypass.
- Store redacted scheduled text only in laptop SQLite. Use authenticated local/E2EE commands, UTC deadlines, stable conversation identity, cancellation and UI acknowledgments. A separate daemon worker continues after the phone closes. Wait for idle/approval clearance and fresh native evidence between deliveries.
- Claim delivery durably; retain uncertain outcomes without automatic replay. Clear text after terminal outcomes and cancel schedules on chat deletion. Expire terminal metadata 30 days after the outcome, including cancellation of a future message.
- Connect automatic routing to recorded rate limits and fresh known model/provider quota. Honor policy/provider restrictions, recheck policy and current conversation, and persist the attempt before switching. Do not replay a previous task.
- Replace detached reset-queue process spawning with hosted native conversation reconnection. Require known exhaustion and fresh recovered quota across known windows, serialize cancellation and reconnection, and cancel old reset entries on model changes. Reconnection submits no task.
- Persist generic lifecycle feedback and dispatch bounded native notifications. Avoid duplicate approval/reset balloons from the legacy Windows tray. Hide internal reset markers from public notifications. Keep Windows notification helpers hidden and alive long enough to display a balloon.
- Fix the header's zero-width title column by allowing its controls to wrap with a minimum title width. Verify schedule save/cancel and narrow layouts using an isolated fixture console.

## Root-cause fixes

The old resume queue launched a separate process, which bypassed hosted ownership. Routing policy had no automatic execution path. Phone timers cannot own durable jobs, so deadlines and claims now belong to Rust/SQLite. Scheduling initially allowed reused keys to acknowledge different payloads; keys now preserve their original message. Stale idle evidence could permit adjacent deliveries; subsequent jobs now wait for fresh native activity. The header's non-wrapping flex row reduced its title column to zero width. Reset cancellation originally matched only an exact native ID; it now cancels every alias belonging to the same stable conversation.

## Verification

The Rust gate, web tests/typecheck, embedded-console/navigation checks, tray check and release packaging are run for this phase. Final results and artifact hashes are recorded before commit. New tests cover durable save/restart, duplicate-key rejection, cancellation/privacy, uncertainty/no replay, retention, notification deduplication and loopback authentication. An isolated HTTP terminal test verifies busy waiting, actual paste/submit framing, one delivery per fresh idle observation and quota-confirmed reconnection without submitting a task.

Browser checks use anonymized fixture data and a temporary in-memory console, never the user's agent homes. The scheduling UI saves only after laptop acknowledgment, clears its draft after that acknowledgment and displays cancellation. The header is checked at its failing width and at 390px.

## Remaining acceptance limits

Mailbox delivery and hosted Auth redirect allow-list/SMTP configuration are not available through the public settings endpoint. Supabase email verification is enabled; do not claim a real inbox flow was completed. Real authenticated native model activation, a physical phone, and macOS/Linux packaged notifications remain release acceptance checks. Native terminal acknowledgment is not model completion. The laptop must remain awake with Gnomcode running; no cloud execution or wake-from-sleep is provided.

## References

- [ADR 0027](../adr/0027-laptop-owned-scheduling-and-quota-recovery.md)
- [Supabase password authentication](https://supabase.com/docs/guides/auth/passwords)
- [Supabase confirmation resend](https://supabase.com/docs/reference/javascript/auth-resend)
