# 0022 - Continuous conversations and native resume

- **Status:** Accepted. Superseded in part by [0058](./0058-1.0-scope-three-agents.md) (Gemini CLI resume is removed from 1.0).
- **Date:** 2026-10-30

## Context

The existing switch implementation always launched a new session and submitted a task-oriented briefing. It split the visible chat, changed its title, exposed implementation details and triggered unsolicited workspace inspection. Native/host identity was also insufficient to reconnect an old conversation after process restart.

## Decision

Use a durable local conversation identity with ordered native/host members. Named-model changes in supported hosted CLIs use their official in-session command. Cross-provider changes add a member while retaining history, title, selection and drafts. The next user prompt includes bounded quoted redacted chat context, filtered from normal conversation presentation; switching itself starts no task.

Persist membership, requested model, launch metadata and pending-context state in SQLite schema v9. Exclude them from metadata cloud sync. Native histories remain agent-owned. Conversation paging spans members; deleting Spawnt chat removes every linked member's rows while retaining routing metadata.

Resume an exact saved native session with the official CLI in the recorded workspace. Replace the stopped host binding without creating another visible conversation. Expose the same bounded action to desktop and encrypted Remote. Keep the quota Resume Queue separate. Mobile requires an awake online laptop and running Spawnt.

## Consequences

Conversation continuity survives provider changes and daemon restarts. Cross-provider context remains bounded and lossy; provider-internal state is not portable. Model command delivery is not proof of model activation. Provider-default selection can use a new backing session when no automated native default selector is established. Unsupported native resume remains read-only. Login, trust and approvals remain native agent controls.

## Alternatives considered

- Always create a visible chat per agent session: rejected because users experience one continuing conversation.
- Rewrite or transplant provider log files: rejected because this risks agent-owned history and cannot transfer internal state reliably.
- Submit the handoff immediately: rejected because changing a model should not independently run a task or emit a reply.

## Command references

- [Codex developer commands](https://learn.chatgpt.com/docs/developer-commands?surface=cli) and [native inline model parser](https://github.com/openai/codex/blob/main/codex-rs/tui/src/chatwidget/model_command.rs).
- [Claude Code model configuration](https://code.claude.com/docs/en/model-config).
- [Gemini CLI commands](https://geminicli.com/docs/reference/commands/) and [session management](https://geminicli.com/docs/cli/session-management/).
