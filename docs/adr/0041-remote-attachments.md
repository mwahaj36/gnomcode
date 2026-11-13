# ADR 0041: Bounded workspace attachments over local API and E2EE Remote

Date: 2026-11-13
Status: Accepted

## Context
Phone and local console users need to attach screenshots, logs, and documents to a session prompt so the agent can inspect them, without opening arbitrary filesystem writes or adding `--add-dir` permission escalation.

## Decision
Add `crates/daemon/src/attachments.rs` exposing `POST /api/v1/sessions/{id}/attachments` and E2EE Remote `{ "type": "upload_attachment", ... }` -> `attachment_ack`:
- Extension allowlist: `png`, `jpg`, `jpeg`, `webp`, `gif`, `txt`, `md`, `log`, `json`, `csv`, `pdf`.
- Size limits: 15 MiB per file, 200 MiB total per session attachment directory; declared size must match decoded bytes.
- Filenames are stripped to a sanitized basename and prefixed with 6 random bytes (`getrandom`) before writing into `<workspace>/.gnomcode/attachments/<prefix>-<name>` (canonicalized to ensure workspace confinement).
- Returns relative `workspace_path` so the prompt references it within the session's existing workspace without `--add-dir` or bypass flags.

## Consequences
- Phone companion (#194) and local console can stage attachments safely over E2EE Remote.
- Existing agent permission posture is preserved.
