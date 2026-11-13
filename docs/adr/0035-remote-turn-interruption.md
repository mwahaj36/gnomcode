# 0035 — Remote turn interruption

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

The existing hosted-session stop endpoint terminates the process. Remote has no interrupt request. G5 needs a bounded request to deliver Esc during an observed running turn while preserving native permissions and the hosted process. Screen emulation and universal native cancellation are not established by this change.

## Decision

Add a Rust-only `interrupt` request to the existing authenticated E2EE dispatcher and a matching local endpoint. Handler logic lives in a new daemon module; shared daemon files receive only new module, route or dispatch lines. Reuse `gnomcode_pty::send_input`, which authenticates to the loopback host and waits for the actual PTY write acknowledgment.

Resolve the stable conversation to its current unique official-agent host using existing binding helpers. Do not launch or resume. Require a source observation at or after host startup, reject future/stale observations using the existing Running classifier, and refuse idle, approval-wait, ended, error and pending-hook-approval states. Serialize with prompt/resume/switch control. Run database/discovery work on a blocking worker.

The request carries an 8–64-character ASCII request ID, limited to letters, digits, hyphens and underscores. Retain at most 256 response receipts for ten minutes in memory, without prompt or terminal content. Identical retries return the original response; cross-conversation ID reuse is rejected. Retain ambiguous write failures too, because replaying Esc might affect a later composer. Reject new requests at capacity. Receipts do not survive restart; the client must never automatically replay an old interrupt after reconnect or expiry.

Remote JSON: `{"type":"interrupt","session_id":"<conversation>","request_id":"<unique-id>"}`. Local JSON at `POST /api/v1/sessions/{id}/interrupt`: `{"request_id":"<unique-id>"}`. Both return `{"type":"interrupt_ack","session_id":"...","request_id":"...","sent":true|false}` or the existing `{"type":"error","message":"..."}` envelope. Local validation may return HTTP 400/401/403. `sent:false` is a no-op; `sent:true` establishes only the acknowledged write, not native cancellation. Error messages contain no raw host output or credentials.

## Consequences

The UI can request interruption without exposing a remote shell, arbitrary input bytes, permission decisions or process termination. Duplicate requests cannot emit additional Esc bytes within the receipt window. Native cancellation remains controlled by the official agent and version; live acceptance for all providers and a physical phone remains unverified. Transcript-based Running observations can lag the screen; this change does not claim screen-verified turn identity. Missing evidence causes a no-op.

## Alternatives considered

- Reuse process stop — ends the session instead of requesting turn interruption.
- Expose arbitrary PTY input through Remote — expands the control boundary unnecessarily.
- Retry on write timeout — an ambiguous first write could already have reached the CLI.
- Wait for the screen model — G5 can safely expose an explicitly labeled input-delivery acknowledgment now; accurate native cancellation evidence remains a separate check.
