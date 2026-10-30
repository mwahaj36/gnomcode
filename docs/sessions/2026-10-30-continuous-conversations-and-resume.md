# Continuous conversations, native resume and header limits

Date: 2026-10-30

## Goals

Keep one visible conversation across model/provider changes, hide generated technical briefings, resume saved conversations after laptop restart from desktop or paired mobile, center model notices without side rules, and show refreshing remaining limits in the conversation header.

## Outcomes and root-cause fixes

The original switch path always launched a new session and immediately submitted a task briefing. The UI selected the new native identity, splitting history and changing its title. Schema v9 now records stable local conversation membership, ordered native/host segments, launch metadata and pending context. Named-model changes use native CLI commands; cross-provider changes attach another backing agent and defer bounded quoted context until the next actual user message. The normal conversation filters that envelope after redaction, including normalized line breaks. No automatic workspace-inspection prompt is sent merely by changing models.

History reads page across segments using stable event IDs. Original opening messages provide titles. Deletion removes all linked Spawnt chat rows; agent logs remain untouched. Routing remains local and excluded from metadata cloud sync.

Exact native resume uses official Codex, Claude Code and Gemini CLI resume commands, a detached hosted terminal, recorded workspace and strict native ID validation. Local and Remote control serialize resume/switch/prompt operations. Unsupported native resume remains read-only. Existing quota Resume Queue semantics remain separate.

Conversation headers show current-provider remaining limits and trust labels. Two-second foreground polling picks up newly ingested observations; older readings say Last seen, and elapsed resets wait for a new observation. Multiple model pools retain distinct labels. Provider data emission controls actual freshness; there is no subscription-token polling.

Model-change notices and their timestamps are centered with no side rules. Desktop and Remote header layouts wrap at phone widths.

## Verification

- Full Rust quality gate: formatting, clippy with warnings denied, workspace tests, cargo-deny and core purity.
- Web tests, TypeScript checking and production Next.js build.
- New store tests: multi-provider paging/deletion, database reopen and pending-context persistence, host replacement, v8 upgrade and idempotent migration.
- Native PTY tests: acknowledged model command frames, unsafe model rejection, exact native resume arguments without permission escalation.
- Daemon regression: original history/title survives provider change, private context remains absent from visible prompts, disconnected history exposes resume.
- Quota regression: provider isolation, changed readings, unknown values, observation age and elapsed resets.
- Browser desktop check: title/history/draft preserved, switch submits no prompt, resume after reload, live header reading changes, centered notices and 390px layout.
- Encrypted Remote browser acceptance uses synthetic history through the existing relay: saved-session resume, continued prompting, switch draft preservation, remembered-device reconnect and remaining-limit refresh.

## Cross-platform and privacy review

No dependencies added. Commands use separate arguments and existing platform executable resolution; no shell interpolation or permission escalation. Windows and Unix hosts use the same validated resume construction and existing detached launch paths. No agent-owned logs are modified, credentials persisted or conversation data added to unencrypted sync. Actual authenticated provider CLI acceptance, physical laptop reboot/sleep, physical phone and macOS/Linux execution were not performed; remain release acceptance checks. Provider-default selection may use a new backing session where a reliable automated native default selector is unavailable.

## Decisions

See ADR 0022 and SPEC sections 3.6, 3.7, 3.9 and 3.14. Preserve bounded local context rather than attempting to transplant provider-internal state. Keep reporting session identities separate from the conversation projection.
