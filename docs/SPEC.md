# Gnomcode — Specification and delivery status

| Field | Current value |
| :--- | :--- |
| Updated | 2026-11-02 |
| Status | Prelaunch; local orchestrator implemented, Remote production gates open |
| Version | 0.9.0 development builds; earlier unpublished 1.x artifacts retired |
| Owner | Wahaj (@mwahaj36) |
| Site | `https://gnomcode.vercel.app` |
| License | Proprietary, closed source; all local features free |

This file owns product scope, architecture and delivery status. [PRODUCT](../PRODUCT.md) explains the product for discussion. [LAUNCH_CHECKLIST](LAUNCH_CHECKLIST.md) lists blockers and closure evidence. [Sessions](sessions/README.md) preserve engineering history. The [previous specification](history/SPEC-before-2026-10-29-refresh.md) is archived for detailed historical design, not normative status.

## 1. Product

### 1.1 One-liner

**Gnomcode is the local-first runtime for AI coding agents:** launch official CLIs, work in one conversation workspace, switch models with a compact handoff and control supported sessions from a paired phone/browser.

Current identity: **Gnomcode**, command/package `gnomcode`, working site `https://gnomcode.vercel.app`. See [ADR 0026](adr/0026-gnomcode-rename.md). Data-home precedence: `GNOMCODE_HOME`, legacy `SPAWNT_HOME`, existing `~/.spawnt`, then `~/.gnomcode`; no automatic history move/deletion. Deployed repository/relay/database identifiers and Remote cryptographic labels remain stable compatibility exceptions. Historical records keep their original identities.

### 1.2 Problem and audience

Developers run several agents across projects, lose continuity at limits and cannot direct sessions away from the desk. Gnomcode coordinates those workflows and gives one view of state, usage, estimated API-equivalent cost and quota. Primary users already use Claude Code, Codex or Gemini CLI. Team aggregation is a later direction.

### 1.3 Positioning

Lead with orchestration, continuity and encrypted remote control. Monitoring and reporting support the runtime. Do not claim a universal takeover of editor agents, identical provider capabilities, lossless state transfer or production certification. Marketing may describe implemented outcomes optimistically; docs retain acceptance limits. No telemetry or invented customer/social proof.

### 1.4 Main workflows

- Launch an installed official supported CLI in a selected folder; use chat for prompts and Terminal stream for native interaction/login/approval.
- View imported history without assuming ownership of its process.
- Change supported models in the existing official CLI session. Cross-provider changes keep the same Gnomcode conversation, title and history, with bounded local context sent privately with the next user message.
- Resume saved supported conversations after laptop sleep/restart from desktop or a paired phone while the laptop and Gnomcode are online.
- Pair a phone/browser and use encrypted history, prompting, supported CLI launch and bounded directory browsing.
- Read usage/quota with provenance; configure free local guardrails, alerts, resume and routing policy.

### 1.5 Commercial model

All local features are free, with no account/license requirement: CLI, daemon, Electron shell, chat, launch, handoff, guardrails, alerts, Resume Queue, analytics and exports. Remote and Teams are paid categories; prices/terms are not finalized. The Remote prototype currently checks a signed license or live validated Pro account. Retain existing optional metadata sync under its explicit separate privacy exception. Do not reinstate old paid-local gates. Teams is not a complete shipped service.

### 1.6 Boundaries

Gnomcode is not an AI API proxy and does not supply models, vendor subscriptions or provider quota. Official CLIs own credentials and permissions. No permission-escalation flags. Read-only observation is default; only explicit integration edits agent config, with backup/removal. Gnomcode-owned redacted local chat is permitted for continuity, while agent histories are preserved. Unrelated file contents and raw tool output must not become persisted monitoring data or unencrypted cloud chat.

### 1.7 Success and evidence

Measure install reliability, real launch/handoff completion, safe Remote reconnect, adapter quality and full-process resource usage. No analytics instrumentation. Windows/browser automated evidence is not physical-phone, cross-platform or live provider acceptance. Performance targets require measurements before being advertised.

## 2. Supported agents

Adapter inventory originally inspected on 2026-10-26. This table describes source coverage, not universal hosted-control or production acceptance. See the capability table below and launch gates.

| Agent | Tier | Source | Tokens | Model | Quota | Live state |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Claude Code** (CLI + IDE) | A (M1) | `~/.claude/projects/**/*.jsonl`, `~/.config/claude/projects`, `$CLAUDE_CONFIG_DIR` | recorded when present (incl. cache-write split) | Sometimes | unknown; adapter emits no quota snapshots | heuristic → exact with hooks |
| **Codex CLI + ChatGPT / Codex Desktop App** | A (M1) | `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (default `~/.codex`; `session_meta.payload.originator` distinguishes `"Codex Desktop"` vs CLI vs VS Code) | exact (`last_token_usage`) | Yes (`provenance.model`) | **exact** (`rate_limits` 5h + weekly per `limit_id`) | heuristic |
| **Antigravity** (app / IDE / CLI) | A (M1) | `~/.gemini/antigravity/brain/<id>/.system_generated/logs/transcript.jsonl` | recorded per `PLANNER_RESPONSE` when present | Sometimes, from settings/fallback | unknown; adapter emits no quota snapshots | heuristic |
| **Gemini CLI** | A (M1) | `~/.gemini/tmp/<hash>/chats/` + `~/.gemini/` | recorded when present | Sometimes | unknown; adapter emits no quota snapshots | heuristic |

The A/B/C labels are historical adapter priorities, not release guarantees. Actual discovery and record coverage vary by installed version. No undocumented provider API is approved by this inventory.

---

### 2.1 Hosted-control capability

| Agent | Monitor/import | Launch/prompt/handoff target |
| :--- | :--- | :--- |
| Claude Code | Adapter and history | Official CLI |
| Codex | CLI/desktop/IDE log adapter and history | Official CLI; no desktop app takeover |
| Gemini CLI | Adapter and history | Official CLI |
| Antigravity | Transcript adapter | Official `agy` CLI; live authenticated acceptance pending |

Remote commands additionally require paired authorization and a live uniquely bound supported agent host. Shells are local-only. Real model/provider compatibility remains a launch acceptance gate.

## 3. Architecture

### 3.1 Invariants

Rust owns parsing, pricing, quotas, rules and routing. `core` is pure synchronous domain logic; adapters are deterministic. No raw content/token logging. Redact summaries and permitted chat before persistence. All measurements carry `exact | estimated | api | unknown`. No analytics. Hooks fail open with a 50 ms IPC budget. Agent configs remain untouched except explicit integration. Sensitive SQL/path/license strings use `obfstr!`; release DevTools/sourcemaps and executable packers are prohibited.

### 3.2 Components and dependencies

`core` feeds adapters/store/platform; ingest coordinates log reading; daemon owns orchestration and authenticated HTTP/WS; CLI is a thin command client. `pty` provides independently owned terminal hosting; `tray` supplies native tray support. Electron is a thin shell with validated preload IPC. Next.js owns marketing and account/Remote web views. Business logic stays in Rust, including pricing and chart grouping.

### 3.3 Processes and terminal lifecycle

Official CLIs run in independent PTY owners, outside Electron. Windows uses ConPTY; Unix uses portable PTY support. Host prompt delivery uses bounded authenticated loopback input acknowledgements, bracketed paste and separately acknowledged Enter. Window resize is bounded. A local same-workspace xterm shell is distinct from the agent and unavailable over Remote. Cross-platform crash survival and reattachment require acceptance evidence.

### 3.4 Repository layout

See [AGENTS](../AGENTS.md) for crate responsibilities and engineering rules. `apps/desktop` is Electron; `web` is Next.js marketing/account/companion; `npm` stages platform shims; `fixtures` contains anonymized agent logs; `docs/adr` records decisions; `docs/sessions` has one file per engineering session.

Windows setup is per-user and waits for an explicit Install action. Choose a dedicated local installation folder, desktop shortcut, Start menu shortcut, launch at Windows sign-in (off by default), and Explorer folder/background integration. Explorer's Open with Gnomcode opens the local session launcher with the folder prefilled; the user still chooses and launches an agent. Classic Explorer verbs appear under Show more options on Windows 11. Uninstall uses its own installation directory and removes shipped files rather than recursively deleting a selected folder; local history under GNOMCODE_HOME remains separate. macOS/Linux packaging retains its existing platform launchers.

### 3.5 Ingestion

Cline/Roo, Cursor and Aider adapters were removed on 2026-11-12 to focus on agents Gnomcode can launch, resume and report limits for. Their roots are no longer discovered. Previously ingested rows remain in the local database and are never deleted or relabeled: typed lists, reports and queues skip unknown agent identifiers. Agent-owned logs are untouched.

Watchers improve latency; reconciliation/checkpoints guarantee progress. Handle rotation, truncation, partial records, duplicates and malformed/unknown lines without killing ingestion. Windows reads share READ/WRITE/DELETE and do not hold idle source handles. Accept LF and CRLF. Respect home overrides and preserve agent logs. Missing model identifiers remain unknown rather than being guessed. Cursor roots are automatically discovered when present. Codex harness/AGENTS/environment context is excluded from conversation display/ingestion; legacy filtering preserves row cursors.

Antigravity IDE and CLI transcripts share the existing parser but use separate `antigravity/brain`, legacy `antigravity-ide/brain` and `antigravity-cli/brain` roots under `GEMINI_HOME` (default `~/.gemini`). Transcripts are read only at `brain/<id>/.system_generated/logs/transcript.jsonl` (no recursive walk of artifact folders; `transcript_full.jsonl` duplicates the same steps). Antigravity's summary databases are authoritative for subagent parent links; a transcript names its parent only in its first system step. Codex discovery includes `CODEX_HOME/archived_sessions` and zstd-compressed `rollout-*.jsonl.zst` archives (decoded in memory, 256 MiB cap, fingerprint-checkpointed). A forked Codex rollout keeps the UUID in its filename even when it replays its parent's header, and Codex usage is keyed by its cumulative `total_token_usage` so re-emitted turn counts are not double-counted. Claude subagent transcripts (`isSidechain` with `agentId`) become `claude-code:agent-<agentId>` sessions linked to the parent `sessionId`; the `claude-desktop` entrypoint is the desktop surface. Claude Code is also resolved from the native installer and the copy bundled with the Claude desktop app, including the packaged (MSIX) app's `Packages/Claude_*/LocalCache/Local/Claude-Data` location. Processes started by the packaged Claude app see a virtualized AppData, so installs and diagnostics run from inside it do not reach the user's real AppData. Development Electron runs use the `app.gnomcode.desktop.dev` identity so they never claim the installed app's taskbar icon. `scripts/verify-ingest.py` independently compares raw logs with an ingested database. CLI sessions retain the CLI surface. Resolve workspace metadata read-only from the corresponding summary databases; preserve native logs and use stable conversation/step deduplication keys. Subagent sessions across adapters (Codex subagent rollouts, Antigravity child agents, Claude subagents, Gemini subagents) preserve their distinct IDs, parent session links (`parent_session_id`), and subagent identities/roles (e.g. `Goodall (control) · Subagent`) rather than colliding with or clobbering parent sessions and models. Session listings expose recorded token counts and subagent relationships.

### 3.6 Storage and local conversation

Conversation redaction preserves original whitespace so Markdown reaches presentation intact. A one-time local format repair (maintenance marker 11) reparses available native sources and updates only existing chat-event content; it does not insert deleted/expired chat, reset source checkpoints, change event IDs/cursors, replay activity or duplicate usage. Source files remain read-only. Failed source reads or checkpoint lookups leave repair pending for a later pass; normal checkpoint-based ingestion still runs once without recursive repair retries. Missing originals cannot recover formatting already lost. Imported attachment transport wrappers become the user request plus an explicit unavailable-preview label; no arbitrary local file path is opened or sent to Remote.

Assistant replies render CommonMark headings, emphasis, quotes, nested lists, fenced code and safe links with a pinned Markdown renderer; tables, strikethrough and disabled task-list checkboxes are also supported. Raw HTML stays escaped and image markup does not automatically fetch remote content. The same renderer and responsive table/code styles ship in native and web chat. User text remains literal. A transport acknowledgment is not proof that a CLI accepted a turn: unmatched prompts retain their text and become visibly unconfirmed after fifteen seconds or a newer native turn, instead of an endless response spinner. Antigravity workspace-trust and Codex hook-review screens reject programmatic pasted prompts until the user resolves startup in the local Terminal; the draft remains available for retry. The browser-hosted local console offers an authenticated read-only directory picker for launching a workspace; Electron retains its native picker.

Default SQLite: `~/.gnomcode/state.db`, or `GNOMCODE_HOME/state.db`; CLI `--db` may select another database. Store sessions/projects, usage, quotas, source checkpoints, rules/queue/profile and redacted chat events. UTC storage, local conversion in presentation. Blocking SQLite runs off async execution paths. Migrations preserve existing data.

Schema v9 adds local-only conversation membership, ordered native/host bindings, requested model and launch metadata, and pending-context delivery state. These mappings contain no credentials and are excluded from metadata sync. Retain them across restarts and chat deletion so routing does not resurrect deleted chat. Chat deletion removes rows for every linked native/host identity; retention applies to all chat rows under the existing timestamp policy. Agent-owned logs remain untouched.

Schema v10 records SHA-256 digests of deleted event IDs as local tombstones, atomically with manual deletion and retention pruning. Both direct chat insertion and source ingestion reject tombstoned IDs while accepting new IDs. Tombstones remain until the local database is reset, are excluded from account sync, and contain no raw identifier or chat content. Digests are unsalted and may reveal predictable identifiers; apply the same local access controls as the database. They protect stable event identities, not changed IDs, older backups, agent-owned history or a database reset. Migration preserves existing data and cannot reconstruct tombstones for past deletions. Older Gnomcode binaries ignore tombstones; restart all Gnomcode processes on the updated version.

Chat rows remain local and are excluded from metadata sync. History supports paging and session deletion; label metadata-only search and explain that deletion does not remove session titles, agent logs or backups and that changed source identifiers, restored backups or database resets can reimport deleted messages. Retention primitives compare timestamps as instants but are not automatically scheduled; configuration/scheduling and backup/WAL deletion guarantees require launch verification. Do not equate row deletion with forensic erasure. Browser drafts and pending bubbles are memory-only. File/diff inspection is read-only and repository-contained; Git diff and status output each have a 1 MiB limit and a ten-second timeout, with failures surfaced rather than empty success. Untracked files appear in status, not diff content. Directory browsing is bounded and not persisted.

### 3.7 Session and conversation state

Prefer the source application's saved chat label over prompt-derived labels. Refresh Codex's session_index.jsonl (relative to its discovered sessions root, respecting CODEX_HOME), Claude sessions-index.json plus custom-title records, and title/summary metadata in supported Gemini exports. Existing Antigravity summary reconciliation retains its title. Labels are bounded and redacted before local persistence; metadata-only renames do not advance activity or require transcript re-ingestion. Keep the original native source's name across provider switches. Sources without a reliable native label retain a cleaned user-request/project fallback; attachment-wrapper headings are not chat names. Sidecar reads use shared platform file reads, remain read-only and skip unavailable/oversized metadata files (8 MiB cap).

Web Remote entry points share the console navigation shell, including the unpaired Connect your desktop screen. Cloud Project activity lists project labels already present in the latest session snapshot and opens filtered session history; missing snapshots offer connection/reload actions. This view does not fabricate detailed local-only project rollups or add project data to cloud sync, and its list is not a timeline-filtered usage report.

Internal states retain thinking/tool/approval/idle/error/ended distinctions. Public status badges use Running, Waiting, Idle and Error; explicit ended/disconnected information remains lifecycle feedback rather than false activity. Dashboard activity inferred from logs is labeled as possible, and its count as an estimate, not a live process count. Pending local bubbles reconcile once with recorded turns, preserving identical repeated prompts and per-session drafts. Replies render safe Markdown; raw HTML/media remain inert. Enter sends; Shift+Enter adds a newline. Paging/scroll behavior preserves the reader's position. Conversation identity is independent of agent process identity. Order linked histories by segment then native turn, retaining event IDs as paging cursors. The original native source supplies its saved title across switches, falling back to the first meaningful opening request. Generated context and native commands are absent from normal chat; show only a brief model-change status. Raw Terminal remains an explicit separate view.

### 3.8 Pricing and models

Rust computes estimated API-equivalent cost/cache savings from recorded token classes and available rates. Unknown prices remain unknown and partial coverage is labeled. Counts can be exact while money is estimated. Plan Value compares estimated API-equivalent work with a configured subscription fee; it is not invoice savings.

Model selection uses a shared Rust catalog for the local console, CLI and encrypted Remote. Show provider/CLI groups and selectable model names; never require a manually typed model ID for changing models. Carry the explicit CLI identity with each choice, including account defaults, aliases and safe CLI-reported custom IDs. Known mismatched provider/model prefixes and option-like input remain rejected. Preserve conversation identity/history/drafts across switches.

On explicit catalog requests, check Codex's native login status and its fresh visible models cache (24-hour maximum); use Claude Code's native auth status and initialization models without sending a user prompt or persisting a session; use Gemini's selected auth method, public active-account metadata or configured API/Vertex environment presence and its supported `models list --output-format json` command. Gemini is labeled configured, not live-authenticated. Hidden/stale/malformed/duplicate Codex entries and malformed/option-like model IDs are excluded. Never forward account emails or raw status output, generate inference to discover models, or use pricing as an entitlement list. Provider credential files are read only by the opt-in limits sources in ADR 0029. Read-only metadata probes run in isolated temporary directories with bounded output and an eight-second deadline, concurrently across providers, outside database locks. CLI-version failures and missing PATH entries remain explicit unavailable/unknown states. Antigravity uses the native `agy models` tab-separated catalog, with validated IDs and bounded/redacted labels; it is labeled configured rather than live-authenticated. Resolve its official CLI installation directory when missing from PATH (Windows LocalAppData/agy/bin, macOS/Linux ~/.local/bin), using the same resolver for discovery and launches.

`/api/v1/model-catalog` requires the normal local authentication/Host/Origin checks. Encrypted Remote's optional `models` request returns only sanitized provider states and model choices; old desktops can return an error with refresh/update guidance. Catalog errors do not clear prompts or pending delivery. The legacy `/api/v1/models` pricing-enriched cache endpoint remains compatible. Catalog metadata is not a live quota or entitlement guarantee; native CLIs validate access when a model is used. No agent-owned configuration is modified.

An explicit provider switch remains available after a provider error, including exhausted credits; active generation, tool execution and permission waits still block a handoff. A switch preserves conversation identity/history and carries context on the next prompt. PTY input acknowledgments follow the actual write; Codex receives a non-text End key before Enter to flush Windows paste bursts without changing the prompt. Credit exhaustion must be reported from native evidence, never inferred from a missing quota reading.

### 3.9 Quotas

Track provider/window/model pools independently. Codex recorded `rate_limits` supplies exact source observations where available; this is not a fresh quota API request. Claude Code limits come from its opt-in status line or, with `gnomcode limits enable claude`, Anthropic's OAuth usage endpoint; Antigravity limits come from its running language server with `gnomcode limits enable antigravity` (ADR 0029). Other sources may only permit estimates or unknown. Preserve observation time and reset provenance; never manufacture limits. Conversation headers on desktop and Remote show remaining percentages for the current provider, with named windows/pools, trust and observation time. The existing two-second foreground refresh reads newly ingested observations without querying subscription credentials. Mark readings older than two minutes as last seen; after a recorded reset passes, await a new reading rather than inventing replenished capacity. Unknown or unavailable readings remain unknown, and provider/model changes replace the displayed provider limits. Context fill uses latest request context, not lifetime tokens, and can shrink after compaction. Rust derives a display state per window (`ok`, `low` below 20% left, `exhausted`, `reset_awaiting`, `unknown`) and, for exact or API readings after the first 5% of a window, a pace: the remaining percentage expected under even use, the difference, a verdict (`on_pace` within 5 points, `ahead`, `behind`) and a run-out time when it falls before the reset. Interfaces render these fields and never recompute them. `GET /api/v1/quota` adds `state`, `pace`, `stale`, `label`, `pool` and `window_label` to each window without renaming existing fields; `GET /api/v1/providers/overview` returns one card per provider (Claude Code, Codex and Antigravity always listed) with its windows, worst-window state, the tightest window (`headline_window`), plan, source label, estimated recorded spend and, when there are no readings, the enable hint and command.

### 3.10 Guardrails and local automation

Local rules, alerts and loop/budget controls are free. Hard-stop rules attempt to block only Claude Code tool calls through its installed hook; other agents receive alerts only. Hooks fail open and never bypass official permissions. Expose these limits and distinguish estimated spend from billing. Native automation acceptance remains provider/version-specific.

### 3.11 Local API and shell security

Bind `127.0.0.1` only; validate bearer token and Host/Origin for HTTP/WS. Electron uses context isolation, sandboxing, no Node integration, strict CSP and sender-validated narrow preload. Navigation stays within the authenticated daemon. Allow only its exact PTY WS resources. Remote web never receives a shell or arbitrary process/window control.

### 3.12 Platform scope

Windows, macOS and Linux are required design targets. Current recorded local packaged acceptance is Windows. macOS/Linux and architecture-specific release acceptance remain open. Use platform-resolved paths/env overrides; canonicalize ordinary Git roots as well as worktree roots so path aliases identify the same project. All OS-specific Rust code belongs in platform. Document fallback instead of silently omitting support.

### 3.13 Focus and desktop chrome

Use native focus integration where available and document Wayland/unsupported fallbacks. Electron removes its default menu, uses themed compact draggable chrome and retains OS window controls. Theme IPC accepts only fixed light/dark values from the authenticated main frame. Never expose focus/window control through Remote.

### 3.14 Resume and routing

Resume Queue and routing are free local features. Policies include `ask_first`, `automatic` and `deny_cross_provider`; legacy Context Relay has its own configuration. Context Relay bypasses auto-compacting environments (`surface: ide` and agents including Antigravity that manage compaction natively in-place), avoiding unnecessary context-reserve alerts or automated relay rotations. Require a known quota observation recorded at or after the eligibility time, with a valid percentage, before resuming; reject future observations. Respect native approval and unique host binding. Do not promise replay of arbitrary original prompt text: the legacy privacy transition stopped storing custom queue prompts. Verify real quota-reset/native resume behavior before claiming universal automatic continuation.

### 3.14.1 Handoff

Hosted Codex and Claude Code use `/model <model>` and Gemini CLI uses `/model set <model>` for named-model changes in place. Validate model tokens, deliver commands independently of chat prompts and require an idle agent. CLI versions must support the documented command; delivery acknowledgment is not activation confirmation. Provider-default selection or an agent without an automated selector may use a fresh backing session while retaining the conversation.

Cross-provider changes attach a new official CLI to the same local conversation. Preserve original sessions and logs. Switching alone does not submit a task or trigger an agent response. On the next user message Rust includes up to 50 recent user/assistant events, bounded to 16,000 characters total and 4,000 per event, as quoted redacted context. Exclude tool output; escape context delimiters within the JSON history. Hide the reserved `<spawnt_context>` envelope from chat after storage redaction, preserving the actual user message. Keep per-conversation drafts and selection. Requested models remain pending until observed from the agent. This transfers bounded context, not lossless provider-internal state; receiving agents still consume input tokens.

### 3.14.2 Reopen saved conversations

Desktop Resume conversation, local `POST /api/v1/sessions/{id}/continue` and encrypted Remote `resume_session` reuse a live hosted agent or reopen the exact recorded native ID in its recorded workspace: `codex resume <id>`, `claude --resume <id>` or `gemini --resume <id>`. Strictly validate native IDs, preserve CLI permission/login controls and do not inject a task until the user sends a message. Desktop Send can also resume before delivery. Serialize resume, switch and prompt control to avoid duplicate hosts during concurrent local/Remote actions. Keep the conversation ID/history stable and replace only the dead host binding. A provider switch with no native history yet can restart its saved launch and retain pending context. Unsupported native resume remains read-only; do not silently replace a saved native session with an unrelated fresh one.

The laptop must be awake, connected and running Gnomcode for mobile interaction. Remembered pairing resumes its encrypted channel under the existing trust policy; keep the conversation visible during device discovery and reconnection. Scheduled messages acknowledged by the laptop persist locally and run when it is awake with Gnomcode running; the phone may disconnect afterward. No cloud execution is implied while the laptop is offline. Native provider acceptance and macOS/Linux restart acceptance remain release checks.

### 3.14.3 Laptop-owned scheduled messages and automatic failover

Desktop and encrypted Remote may create, list and cancel scheduled conversation messages. Store UTC deadlines and redacted user text only in local SQLite, never metadata sync or the account directory. Show that redaction applies before scheduling. Retain pending messages until execution or cancellation; erase text immediately on cancellation, delivery or failure, and remove terminal records after 30 days. Deleting conversation history cancels its pending messages. Access uses existing loopback authentication or the authenticated encrypted Remote channel. A phone must wait for the laptop acknowledgment before claiming a schedule is saved.

Process overdue messages after restart. Busy/approval-blocked agents wait; do not inject into an active turn or bypass approval. Claim a message durably before delivery. A crash after claiming is reported as uncertain and never automatically replayed. Native delivery acknowledgment means accepted by the terminal, not completed by the model. Unsupported or unavailable sessions fail visibly. Scheduling does not wake a sleeping laptop.

Only conversations launched by Gnomcode expose sending, model switches, reconnection or schedules. Persist launch ownership when a host starts; keep imported provider sessions view-only. Default conversation lists to Gnomcode conversations, with an explicit imported-history filter and optional subagent inclusion. Legacy Codex tool-call identifiers are not sessions and must not enter lists or counts. Recover missing workspace and parent metadata from native logs without replaying usage or restoring deleted chat.

Providers own context compaction. Do not suggest or automatically execute context rotations based on estimated fill for current supported providers. Retire existing pending rotations and retain prior records behind collapsed history. Explicit model/provider switches remain available in owned conversations.

Completion alerts require a provider turn-completion signal; progress messages and token-count updates do not complete a turn. User-question and approval requests have their own attention state. Windows notifications are attributed to the packaged Electron app, which uses a single instance for tray clicks, shortcuts and notification activation. Pass validated original activation arguments through single-instance IPC because Chromium can reorder its command-line switches. Keep timed-out Windows toast objects alive for Notification Center and register the owned Start Menu shortcut with the application identity. Installers ship the Electron shell with the Rust engine; browser mode is a fallback when the shell is unavailable.

Background phone notifications require explicit browser permission and authenticated subscription registration. The native app must have account Remote access enabled and remain online. The daemon retries recent generic lifecycle event IDs until the account-authenticated push endpoint acknowledges them. The service sends fixed generic notices without chat text, tool output, paths or credentials. Destination endpoints are restricted to supported browser push vendors, stored under account-owner RLS, and validated again before sending. Event IDs deduplicate retries. Devices may revoke their subscription. This is opt-in Remote notification traffic, separate from metadata sync. Usage snapshots are stored per desktop in the existing owner-protected device directory; selecting a desktop selects its Rust-computed usage and ROI. No cross-machine overwrite or implicit aggregation.

Automatic routing remains opt-in under the existing routing policy. Only a recorded rate-limit event and fresh known quota can trigger failover, using a supported installed agent/model with positive headroom. Respect same-provider restrictions, idle/approval guards, bounded context and stable conversation identity. Do not replay a task after switching. Record one attempt per triggering event to prevent loops. Notify on switch requested or failed, without claiming observed activation. Desktop notifications contain generic lifecycle text, never message content; scheduling and switching status remain available in the UI when OS notifications are unavailable.

Reset reconnection is armed explicitly per conversation, or by automatic routing when no eligible alternative exists. Require fresh known exhaustion before arming and fresh positive quota after the recorded eligibility time across every known provider window. Reuse the live host or reopen the exact recorded native conversation without submitting a task. Cancel old reset entries on model changes. The daemon owns lifecycle and quota notification decisions; the desktop shell displays native notifications. The legacy tray does not add context-rotation warnings. Email signup uses Supabase confirmation when enabled, supports immediate sessions when confirmation is disabled, and offers resend/expired-link recovery. Public project settings were verified with email signup and confirmation enabled; mailbox delivery remains an external acceptance check.

### 3.15 Optional account sync and encrypted Remote

These are separate data channels. Existing opt-in Supabase snapshot sync is owner-scoped and not end-to-end encrypted. Current `build_remote_snapshot` includes reports/timelines, explicitly selected session fields and quota. Project names, session IDs and model/activity metadata can be sensitive. Session titles, profile, relay records and chat events are excluded. HMAC project-key helpers do not make this live payload hash-only. Publish the true session total while the account list is limited to 200; the web view must identify that limit and filter only those published records. Disclose the snapshot contents and lack of end-to-end encryption before account connection. Verify historical cloud copies, consent and two-account RLS before launch.

Remote uses the public Cloudflare Durable Object relay at `wss://spawnt-remote-relay.mwahaj25.workers.dev/relay`, overridable by `GNOMCODE_REMOTE_RELAY_URL`. Outbound TLS carries E2EE frames; the daemon still listens only on loopback. Linking a native app requires a confirmation modal that explicitly grants its available conversation history and supported controls to every device signed in to the same account, including existing and future logins. The native link API requires `remote_access_confirmed: true` before authentication or persistence. Both password sign-in and browser-account transfer use this modal. Cancel sends no credentials and changes no account state. Existing linked native apps default to disabled until this broader scope is confirmed.

After native consent, account authentication authorizes access across all linked native apps. Browsers establish encrypted connections automatically without OTPs, phone approval or per-desktop device approval. The owner-only cloud directory is deliberately trusted for authorization; possession of the account can grant Remote control. This replaces the narrower independent per-host approval model of ADR 0025. Each connection still pins its enrollment public key and requires encrypted challenge-bound proof on reconnect. No private connection key or conversation content enters the directory.

Native consent persists outside installation directories under `GNOMCODE_HOME/remote-devices/<owner>/`; updates and inactivity do not expire it. The directory's `remote_enabled` switch separately controls availability. Devices can disable an entire native app for all signed-in devices. Heartbeats never re-enable a disabled app, including after restart. Native sign-out clears local consent and suspends all its Remote connections. Enabling again requires explicit native confirmation. Metadata sync remains a separate transport under the same account link.

Fresh phone keys and a desktop challenge bind directional encryption on reconnect. Serialized cancellation and generation checks prevent stale operations restoring access. The browser checks authenticated desktop reachability after 15 seconds and stops automatic retries after 30 seconds, preserving the selected conversation and draft with disabled sending and explicit Try again. Initial connection discovery is also bounded. No automatic replay of an unacknowledged prompt. Account-side disable is applied on the next successful desktop directory poll; local disable suspends connections immediately. Legacy code-based enrollment endpoints remain removed.

Successful browser connections bookmark the selected desktop for navigation and reload. Without a valid bookmark, discovery uses the authenticated account directory, newest available desktop first; it never restores an arbitrary cached device credential. In-app navigation and browser Back/Forward must reconnect without losing the conversation draft.

Directory rows contain owner/device IDs, generic labels, public keys, relay locators, availability/connection state and timestamps. Never publish conversation content, prompts, proof credentials or filesystem paths. Sessions stay on each online host and are discovered over authenticated E2EE. Directory failure prevents new enrollment and is visible in Devices. Restoring an existing connection retries transient discovery failures within the existing 30-second budget; revocation and identity mismatches do not bypass validation. The standalone migration `supabase/migrations/20261101160844_account_wide_remote_access.sql` supports both a fresh database and the earlier device directory; applying it does not enable existing native apps.

Paired commands include history paging, live subsequent responses, supported CLI launch, initial/follow-up prompts and bounded directory browsing. Imported history is labeled read-only. Prompting requires unique live ownership and observed idle/approval restrictions, including the first prompt before native history exists. No remote shell, credential access or permission escalation. Chat travels only through E2EE, never metadata sync.

Public-relay/browser automated acceptance exists. Physical-phone, independent security review and production approval remain open. Do not imply the relay hides connection metadata or is independently certified.

### 3.15.1 Withdrawn cloud collectors

Hosted vendor-cloud-session collectors and legacy unencrypted cloud command intake are withdrawn. Existing migration/history records are retained. Do not reintroduce credential capture or remote commands through metadata sync.

### 3.16 Profile and navigation

Application UI uses platform system sans typography, consistent field/control sizing and a shared spacing scale across the local console, account console and Remote. Keep default views concise: short human-readable statuses, section/column-level provenance and accessible source details replace repeated EXACT/EST badges and per-row inference warnings. Estimated money remains visibly labeled as estimated; unknown and stale readings never become exact or zero. Reporting breakdowns, source explanations and secondary histories use keyboard-accessible disclosures. Empty tables omit column headers. Checkbox/radio geometry stays compact with at least 44px clickable labels; form labels sit above their controls and reflow at narrow widths.

The local console offers a four-step first-run guide covering native CLI installation/sign-in, project selection and launch, conversation controls, usage provenance and optional Remote. It is skippable, keyboard accessible and replayable from Profile. Completion or dismissal is stored only as a versioned local browser preference, with no analytics or account requirement; blocked storage does not prevent use. The final action opens the real launcher without starting a process. Existing folder-launch links retain their prefilled workspace.

Workspace is chat-first with independently collapsible conversation navigation and local terminal. Dashboard groups usage/reporting; Profile groups appearance/account/devices/exports. Desktop fine-pointer rail expands on hover/focus; touch uses an explicit toggle. Pairing disclosures/input survive polling. Phone uses visual-viewport-owned chat/composer and modal folder selection. Its toolbar exposes conversation navigation, new conversation and a labeled desktop-connection control rather than a generic Actions menu. Desktop/account navigation and forgetting a desktop live in a touch-friendly bottom sheet; model selection is directly beside the composer in a separate bottom sheet. Keep at least 44px touch controls, 16px form inputs and independent scrolling for sheets/drawers. Lock document scroll while chat is active, restoring it on exit; track visual viewport height and offset so keyboard resizing/panning moves neither the page nor composer controls. Textarea and toolbar occupy separate non-overlapping rows. All-time reporting uses at most 30 daily, then 30 ISO-weekly, then 30 monthly intervals before yearly grouping; omit unobserved intervals and preserve totals/trust. Token activity renders as a trailing-year grid of UTC days (53 ISO weeks, Monday first) with five Rust-computed intensity levels (square-root scaled to the busiest day), plus active days, current and longest streak and busiest day. Days before the first recording are not drawn; days without recordings read "no recorded activity", never zero tokens. The Today period uses a 24-hour strip in the same style. The daemon serves the grid at `GET /api/v1/activity/heatmap`; daily report rows carry the same `level`.

## 4. Engineering standards

Follow [AGENTS](../AGENTS.md), installed engineering skills and relevant ADRs. Rust edition 2024, workspace MSRV/lints, typed library errors, no non-test unwrap/expect, Tokio only in approved orchestration crates, unsafe restricted to audited platform code. Tests use temp directories and anonymized fixtures, never actual agent homes. No emojis in product code, copy or documentation.

## 5. Roadmap and current delivery

| Area | Implemented | Remaining evidence/work |
| :--- | :--- | :--- |
| M0/M1 foundation, adapters, store and CLI | Present | Current real fixture/agent-version coverage |
| M2 daemon, reports and local console | Present | Platform/resource release acceptance |
| M3 desktop/tray/distribution | Electron shell and Windows development packages | Signed/approved public artifacts, macOS/Linux acceptance |
| M4 local integration/automation | Local access free; rules/relay, laptop-owned scheduled messages and quota-confirmed hosted reconnection | Real native quota/resume and permission acceptance |
| M5 account sync / Teams | Opt-in owner snapshot; schema foundations | Privacy minimization and complete team product |
| M6 launcher/terminal | Independent owner, official CLI launch and local xterm | Real provider and cross-platform lifecycle checks |
| M7 chat/file/diff | Redacted local history, safe chat and inspection | Retention/deletion acceptance |
| M8 routing/handoff | Continuous conversations, named-model controls, quota-based automatic failover, hidden cross-provider context and saved-session resume | Authenticated cross-model/provider outcome confirmation |
| M9 encrypted Remote | Account-wide consent, automatic signed-in access, durable trust and bounded commands | Physical phone, independent security and owner production approval |
| Marketing/documentation | Outcome-first positioning, guide/legal routes and SEO in this phase | Production crawl checks and legal approval |

v1.0 criteria are not met. See [LAUNCH_CHECKLIST](LAUNCH_CHECKLIST.md) for blockers, causes, closure steps and evidence. Historical green checks are not new verification for this phase.

## 6. Deliverables and artifacts

### 6.1 Repositories

Source: `mwahaj36/gnomcode`; releases: `mwahaj36/gnomcode-releases`. Preserve privacy until owner authorizes public launch. Brand/config values originate in `web/branding.json` and generated outputs.

### 6.2 Release artifacts

CLI, Electron packages and notices/EULA/hashes per approved platform. Target npm shim plus seven platform packages: Windows/macOS x64+arm64, Linux x64+arm64 and Linux x64 musl. Targets are not claims of publication. Download links follow actual approved assets. Windows Electron installers and checksums are published to the public binary-only `mwahaj36/gnomcode-releases` repository; private source remains separate. Website download routes redirect only an explicit asset allow-list to the current release.

### 6.3 Versioning

CLI, desktop and npm use lockstep SemVer. Remain 0.x before v1.0. Current development version is 0.9.0. No legacy executable/home migration or compatibility alias; preserve old directories without deleting them.

### 6.4 Updates

Opt-out update-manifest check and approved signed/hash-verified release flow. Do not change public update channels when refreshing draft artifacts.

### 6.5 Website and docs

Next.js marketing presents launch, continuity, encrypted Remote and free-local value. The hero illustrates a concise handoff between agents rather than reporting metrics. The moving supported-agent strip is an independent full-width section; reduced motion renders it static. Log in/Console is the primary red navigation action on desktop and phone; Download is secondary on desktop and available in the phone menu. Every route exports unique metadata; indexable pages use canonical/OG/Twitter fields. Sitemap contains actual public content routes; console/auth/settings and the Remote application are excluded and noindexed; Remote documentation remains public. Shared docs guide pages answer the question first, cite vendor sources where needed and link related content. Privacy and Terms live in marketing navigation, not the console rail. No analytics. Product limitations belong in technical docs and launch gates, without invented marketing availability claims.

The downloads page offers only published artifacts, labels unsigned Windows development builds, and keeps legal/guide links in normal text flow. Missing website routes return HTTP 404 with a themed page and homepage, download and documentation recovery links.

### 6.6 Support

Developer contact: `mwahaj25@gmail.com` from bundled EULA. Keep actionable errors, source/trust disclosure and a documented offline path.

### 6.7 Payments and licensing

Local operations never check a paid license. Remote retains existing prototype entitlement checks. Final prices, refund/payment terms, legal jurisdiction and commercial hosting eligibility require owner approval before selling.

### 6.8 Binary and UI protection

Business logic in Rust. Obfuscate sensitive literals. Strip release symbols as configured; disable release DevTools and sourcemaps; validate Electron security fuses. Never use executable packers. Bundle third-party notices and verify exact packaged payloads.

## 7. Deployment constraints

Keep local listeners loopback-only. Allow only disclosed update, opt-in pricing/provider operations, account services and encrypted Remote network traffic. No analytics/crash telemetry. Verify actual endpoint inventory and commercial hosting plan. Relay operating limits and abuse controls require review. Signing/notarization and platform acceptance belong to launch evidence, not inferred from build success.

## 8. Risks

Provider log/CLI drift, incomplete quota visibility, handoff context loss, sensitive stored content, overbroad account snapshot, remote authorization/reconnect flaws, unsigned artifacts and missing native platform acceptance. Mitigate with fixtures, trust labels, bounded commands, independent security review, explicit consent/minimization and release matrices. No product claim may substitute for evidence.

## 9. Decision log

| ADR | Decision | Status |
| :--- | :--- | :--- |
| [0001](./adr/0001-rust-core.md) | Rust for core, CLI, daemon and hooks | Accepted |
| [0002](./adr/0002-closed-source-free-tray.md) | Closed source; free tray; Pro = guardrails/rules/Resume Queue/analytics/sync via signed license file | Accepted |
| [0003](./adr/0003-distribution-and-deployment.md) | Private source repo, public binaries-only releases repo, npm + OS package managers, Tauri tray, Cloudflare web | Accepted |
| [0004](./adr/0004-cloud-session-collectors.md) | Portable opt-in cloud collectors, measured usage imports, isolated cloud presence rows | Accepted |
| [0006](./adr/0006-shared-spawnt-console-ui.md) | Shared token source and matched local/cloud presentation; preserve separate data adapters and existing native runtime pending a separate migration | Accepted |
| [0007](./adr/0007-shared-console-renderers.md) | Shared console presenters and navigation layout, Rust daily measurements, source-generated freshness and authenticated sync status | Accepted |
| [0008](./adr/0008-platform-release-fallbacks.md) | Versioned native archives and browser-console launchers; retain existing Windows tray and unsigned cross-platform fallbacks | Accepted |
| [0009](./adr/0009-console-pricing-coverage-and-periods.md) | Rust pricing coverage, local-midnight hourly Today, source calendar month, ongoing-first Overview and project filtering | Accepted |
| [0010](./adr/0010-session-presence-and-focus.md) | Shared two-minute presence rules, truthful window focus and separate app/tray icon scales | Accepted |
| [0016](./adr/0016-pty-launcher.md) | Independent PTY owner, ConPTY and POSIX terminal sessions, loopback API, and authenticated reattachment | Accepted |
| [0017](./adr/0017-structured-chat-and-history.md) | Structured drivers, local redacted conversation history, retention, hard deletion, and sandboxed file/diff inspection | Accepted |
| [0018](./adr/0018-router-and-zero-token-handoff.md) | Model router, zero-token handoff compilation, switch/resume, and CLI policy controls | Accepted |
| [0019](./adr/0019-remote-e2ee-relay.md) | End-to-end encrypted remote access, zero-knowledge Cloudflare relay room, ephemeral SAS verification, and mobile companion | Accepted |
| [0020](./adr/0020-account-pairing-discovery.md) | Owner-only discovery of temporary public Remote offers; two-device verification retained | Accepted |
| [0021](./adr/0021-remembered-remote-device-trust.md) | Remembered Remote trust, challenge-bound reconnect and seven-day activity expiry | Accepted |
| [0025](./adr/0025-account-device-approval.md) | Account desktop discovery and durable trust; account-wide native consent supersedes per-host browser approval | Accepted |
| [0022](./adr/0022-continuous-conversations-and-native-resume.md) | Stable local conversation identity, native model controls, hidden context and exact native resume | Accepted |
| [0023](./adr/0023-deleted-chat-replay-markers.md) | Local hashed markers prevent stable-ID replay after chat deletion or retention | Accepted |
| [0004](./adr/0004-cloud-session-collectors.md) | Portable opt-in cloud collectors, measured usage imports, isolated cloud presence rows | Withdrawn in v1.4.1 |
| [0005](./adr/0005-browser-device-pairing.md) | Independent browser login, encrypted one-time transfer, private credential persistence | Pairing withdrawn in v1.4.1; generic credential protection retained |

| [0026](./adr/0026-gnomcode-rename.md) | Gnomcode identity, canonical site and non-destructive legacy history compatibility | Accepted |

| [0027](./adr/0027-laptop-owned-scheduling-and-quota-recovery.md) | Laptop-owned scheduling, conservative automatic failover and quota-confirmed hosted reconnection | Accepted |
| [0028](./adr/0028-managed-conversations-and-device-notifications.md) | Managed chat ownership, native compaction, per-desktop snapshots and opt-in generic background notifications | Accepted |
| [0029](./adr/0029-api-limits-from-agent-sign-in.md) | Opt-in exact Claude and Antigravity limits from provider APIs using the agent's own local sign-in | Accepted |

## 10. Open decisions

- Final Remote/Teams prices, payment/refund process and governing jurisdiction.
- Field-level metadata sync consent/minimization and migration approach.
- Launch platform list and signing/notarization policy.
- Security reviewer and production Remote approval.
- Exact Teams scope and official local model catalog sources beyond Codex.
- Retention configuration/scheduling and deletion/backups contract.

Changes and verification are recorded per session under [docs/sessions](sessions/README.md), rather than appended to this specification.
