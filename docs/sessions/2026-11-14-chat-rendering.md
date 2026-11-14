# 2026-11-14 - Chat rendering and harness entry classification

## Goals
- Render chat like the Claude apps in the desktop console, web console/PWA and Remote: code blocks with label, Copy, scroll and Wrap; diffs; message Copy; image thumbnails; tool chips.
- Stop Claude Code harness entries (`<task-notification>`, `<system-reminder>`, slash-command records and similar) from showing as user bubbles.

## Outcomes
- `core::chat_meta::classify_user_entry` classifies user-role entries; the Claude adapter emits them as `system` events with a subtype, and `store` reclassifies older rows on read.
- `web/lib/console-presentation.js` extends the existing markdown-it renderer (no second renderer): `renderCodeBlock`, `highlightCode`, system and tool chips, attachment thumbnail placeholders, `bindChatInteractions` for Copy/Wrap/image viewer.
- External images never load automatically; a link names the host.

## Problems and root causes
- Harness entries are stored as role `user` in Claude JSONL, and the adapter copied every text block into a user chat event. Fixed at the adapter (plus read-time reclassification) instead of hiding them in the UI.
- A global `pre { white-space: pre-wrap }` override mangled code; code blocks now scroll by default and wrap only on request.

## Decisions
- Highlighting is a small display-only tokenizer to avoid a large new dependency.
- The synthetic fixture `fixtures/claude-code/synthetic/harness-entries.jsonl` is hand-written with neutral text; the anonymize script would replace the tags under test with `[REDACTED]`.

## Verification
- `cargo test -p gnomcode-core -p gnomcode-adapters -p gnomcode-store`, new snapshot `claude_harness_entries`.
- `npm test`, `npm run typecheck`, Playwright `e2e/chat-rendering.spec.ts` at 1280px and 390px.
