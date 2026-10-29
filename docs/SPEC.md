# Spawnt — Specification and delivery status

| Field | Current value |
| :--- | :--- |
| Updated | 2026-10-29 |
| Status | Prelaunch; local orchestrator implemented, Remote production gates open |
| Version | 0.9.0 development builds; earlier unpublished 1.x artifacts retired |
| Owner | Wahaj (@mwahaj36) |
| Site | `https://spawntv.vercel.app` |
| License | Proprietary, closed source; all local features free |

This file owns product scope, architecture and delivery status. [PRODUCT](../PRODUCT.md) explains the product for discussion. [LAUNCH_CHECKLIST](LAUNCH_CHECKLIST.md) lists blockers and closure evidence. [Sessions](sessions/README.md) preserve engineering history. The [previous specification](history/SPEC-before-2026-10-29-refresh.md) is archived for detailed historical design, not normative status.

## 1. Product

### 1.1 One-liner

**Spawnt is the local-first runtime for AI coding agents:** launch official CLIs, work in one conversation workspace, switch models with a compact handoff and control supported sessions from a paired phone/browser.

### 1.2 Problem and audience

Developers run several agents across projects, lose continuity at limits and cannot direct sessions away from the desk. Spawnt coordinates those workflows and gives one view of state, usage, estimated API-equivalent cost and quota. Primary users already use Claude Code, Codex or Gemini CLI. Team aggregation is a later direction.

### 1.3 Positioning

Lead with orchestration, continuity and encrypted remote control. Monitoring and reporting support the runtime. Do not claim a universal takeover of editor agents, identical provider capabilities, lossless state transfer or production certification. Marketing may describe implemented outcomes optimistically; docs retain acceptance limits. No telemetry or invented customer/social proof.

### 1.4 Main workflows

- Launch an installed official supported CLI in a selected folder; use chat for prompts and Terminal stream for native interaction/login/approval.
- View imported history without assuming ownership of its process.
- Switch to a supported target model/provider by launching a fresh session with a locally compiled redacted briefing. Preserve source history.
- Pair a phone/browser and use encrypted history, prompting, supported CLI launch and bounded directory browsing.
- Read usage/quota with provenance; configure free local guardrails, alerts, resume and routing policy.

### 1.5 Commercial model

All local features are free, with no account/license requirement: CLI, daemon, Electron shell, chat, launch, handoff, guardrails, alerts, Resume Queue, analytics and exports. Remote and Teams are paid categories; prices/terms are not finalized. The Remote prototype currently checks a signed license or live validated Pro account. Retain existing optional metadata sync under its explicit separate privacy exception. Do not reinstate old paid-local gates. Teams is not a complete shipped service.

### 1.6 Boundaries

Spawnt is not an AI API proxy and does not supply models, vendor subscriptions or provider quota. Official CLIs own credentials and permissions. No permission-escalation flags. Read-only observation is default; only explicit integration edits agent config, with backup/removal. Spawnt-owned redacted local chat is permitted for continuity, while agent histories are preserved. Unrelated file contents and raw tool output must not become persisted monitoring data or unencrypted cloud chat.

### 1.7 Success and evidence

Measure install reliability, real launch/handoff completion, safe Remote reconnect, adapter quality and full-process resource usage. No analytics instrumentation. Windows/browser automated evidence is not physical-phone, cross-platform or live provider acceptance. Performance targets require measurements before being advertised.

## 2. Supported agents

Adapter inventory originally inspected on 2026-10-26. This table describes source coverage, not universal hosted-control or production acceptance. See the capability table below and launch gates.

| Agent | Tier | Source | Tokens | Model | Quota | Live state |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Claude Code** (CLI + IDE) | A (M1) | `~/.claude/projects/**/*.jsonl`, `~/.config/claude/projects`, `$CLAUDE_CONFIG_DIR` | exact (incl. 5m/1h cache-write split) | Yes | estimated or unknown; no subscription-token API | heuristic → exact with hooks |
| **Codex CLI + ChatGPT / Codex Desktop App** | A (M1) | `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-*.jsonl` (default `~/.codex`; `session_meta.payload.originator` distinguishes `"Codex Desktop"` vs CLI vs VS Code) | exact (`last_token_usage`) | Yes (`provenance.model`) | **exact** (`rate_limits` 5h + weekly per `limit_id`) | heuristic |
| **Antigravity** (app / IDE / CLI) | A (M1) | `~/.gemini/antigravity/brain/<id>/.system_generated/logs/transcript.jsonl` | exact per `PLANNER_RESPONSE` | Yes extracted from `<USER_SETTINGS_CHANGE>` tags in `USER_INPUT` steps | per-model pool (`limit_id` per model family: Claude, Gemini, Fable, etc.) | heuristic |
| **Gemini CLI** | A (M1) | `~/.gemini/tmp/<hash>/chats/` + `~/.gemini/` | exact | Yes | per-model pool | heuristic |
| **Cline / Roo Code** | B (M4) | `<editor>/User/globalStorage/{saoudrizwan.claude-dev,rooveterinaryinc.roo-cline}/tasks/<id>/ui_messages.json` | exact (`api_req_started`) | Yes | n/a (API) | heuristic |
| **Cursor** | C (experimental, opt-in) | `Cursor/User/globalStorage` / compatible transcript exports | partial | partial | unknown | heuristic where recorded |
| **Aider** | C (opt-in) | `--analytics-log <file>` | exact | Yes | n/a | No |

The A/B/C labels are historical adapter priorities, not release guarantees. Actual discovery and record coverage vary by installed version. No undocumented provider API is approved by this inventory.

Editor roots for Cline/Roo are probed across `Code`, `Code - Insiders`, `Cursor`, `Windsurf`, `VSCodium` under `%APPDATA%` (Win), `~/Library/Application Support` (mac), `~/.config` (Linux).

---

### 2.1 Hosted-control capability

| Agent | Monitor/import | Launch/prompt/handoff target |
| :--- | :--- | :--- |
| Claude Code | Adapter and history | Official CLI |
| Codex | CLI/desktop/IDE log adapter and history | Official CLI; no desktop app takeover |
| Gemini CLI | Adapter and history | Official CLI |
| Antigravity | Transcript adapter | Official `agy` CLI; live authenticated acceptance pending |
| Cline / Roo | Editor logs | No hosted editor control |
| Cursor | Local logs | No hosted editor control |
| Aider | Opt-in logs | No hosted launcher |

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

### 3.5 Ingestion

Watchers improve latency; reconciliation/checkpoints guarantee progress. Handle rotation, truncation, partial records, duplicates and malformed/unknown lines without killing ingestion. Windows reads share READ/WRITE/DELETE and do not hold idle source handles. Accept LF and CRLF. Respect home overrides and preserve agent logs. Codex harness/AGENTS/environment context is excluded from conversation display/ingestion; legacy filtering preserves row cursors.

### 3.6 Storage and local conversation

Default SQLite: `~/.spawnt/state.db`, or `SPAWNT_HOME/state.db`; CLI `--db` may select another database. Store sessions/projects, usage, quotas, source checkpoints, rules/queue/profile and redacted chat events. UTC storage, local conversion in presentation. Blocking SQLite runs off async execution paths. Migrations preserve existing data.

Chat rows remain local and are excluded from metadata sync. History supports paging and session deletion; retention primitives exist, but configuration/scheduling and backup/WAL deletion guarantees require launch verification. Do not equate row deletion with forensic erasure. Browser drafts and pending bubbles are memory-only. File/diff inspection is read-only and repository-contained; directory browsing is bounded and not persisted.

### 3.7 Session and conversation state

Internal states retain thinking/tool/approval/idle/error/ended distinctions. Public status badges use Running, Waiting, Idle and Error; explicit ended/disconnected information remains lifecycle feedback rather than false activity. State inference declares its limits. Pending local bubbles reconcile once with recorded turns, preserving identical repeated prompts and per-session drafts. Replies render safe Markdown; raw HTML/media remain inert. Enter sends; Shift+Enter adds a newline. Paging/scroll behavior preserves the reader's position.

### 3.8 Pricing and models

Rust computes estimated API-equivalent cost/cache savings from recorded token classes and available rates. Unknown prices remain unknown and partial coverage is labeled. Counts can be exact while money is estimated. Plan Value compares estimated API-equivalent work with a configured subscription fee; it is not invoice savings.

Model availability uses visible official Codex CLI cache records with a valid refresh timestamp within 24 hours. Hidden/stale/malformed/duplicate entries are excluded. This is availability metadata, not live pricing, quota or entitlement verification. Pricing catalog and model picker are separate. Claude/Gemini retain native selectors until reliable local discovery exists. Never read subscription tokens for discovery or plan detection.

### 3.9 Quotas

Track provider/window/model pools independently. Codex recorded `rate_limits` supplies exact source observations where available; this is not a fresh quota API request. Other sources may only permit estimates or unknown. Preserve observation time and reset provenance; never manufacture limits. Context fill uses latest request context, not lifetime tokens, and can shrink after compaction.

### 3.10 Guardrails and local automation

Local rules, alerts and loop/budget controls are free. Hook actions must fail open and never bypass official permissions. Keep spend estimates distinct from billing. Native automation acceptance remains provider/version-specific.

### 3.11 Local API and shell security

Bind `127.0.0.1` only; validate bearer token and Host/Origin for HTTP/WS. Electron uses context isolation, sandboxing, no Node integration, strict CSP and sender-validated narrow preload. Navigation stays within the authenticated daemon. Allow only its exact PTY WS resources. Remote web never receives a shell or arbitrary process/window control.

### 3.12 Platform scope

Windows, macOS and Linux are required design targets. Current recorded local packaged acceptance is Windows. macOS/Linux and architecture-specific release acceptance remain open. Use platform-resolved paths/env overrides; all OS-specific Rust code belongs in platform. Document fallback instead of silently omitting support.

### 3.13 Focus and desktop chrome

Use native focus integration where available and document Wayland/unsupported fallbacks. Electron removes its default menu, uses themed compact draggable chrome and retains OS window controls. Theme IPC accepts only fixed light/dark values from the authenticated main frame. Never expose focus/window control through Remote.

### 3.14 Resume and routing

Resume Queue and routing are free local features. Policies include `ask_first`, `automatic` and `deny_cross_provider`; legacy Context Relay has its own configuration. Respect eligibility, native approval and unique host binding. Do not promise replay of arbitrary original prompt text: the legacy privacy transition stopped storing custom queue prompts. Verify real quota-reset/native resume behavior before claiming universal automatic continuation.

### 3.14.1 Handoff

Rust compiles redacted objectives, changed-file paths and recent activity, launches a compatible target session and submits a briefing. Non-Git workspaces provide an empty changed-files list; real Git failures remain errors. Requested models remain unconfirmed until observed from the agent. Preserve source session/history and carry drafts. “Zero-token” means no inference call to create a briefing; receiving models still tokenize and charge for it. This is not lossless provider-state transfer.

### 3.15 Optional account sync and encrypted Remote

These are separate data channels. Existing opt-in Supabase snapshot sync is owner-scoped and not end-to-end encrypted. Current `build_remote_snapshot` includes reports/timelines, profile, session records, quota and automation/relay metadata. Project names/stored labels and relay workspace metadata can be present. HMAC project-key helpers do not make this live payload hash-only. Chat events and derived opening-prompt titles must not enter metadata sync. A field-level allowlist/consent review and two-account RLS acceptance block launch.

Remote uses the public Cloudflare Durable Object relay at `wss://spawnt-remote-relay.mwahaj25.workers.dev/relay`, overridable by `SPAWNT_REMOTE_RELAY_URL`. Outbound TLS carries E2EE frames; the daemon still listens only on loopback. Pairing uses an owner-only five-minute offer, eight-digit discovery code and separate six-digit matching-code confirmation. Phone enters the discovery code while signed into the same account; desktop verifies the phone security code and both confirm.

Remembered trust (ADR 0021) uses owner-only desktop storage and a browser nonexportable IndexedDB HMAC proof key. Seven inactive days expire trust; polling does not renew it. Fresh phone keys and a desktop challenge bind directional encryption on reconnect. Serialized revocation and generation checks prevent stale writes restoring trust. Reload/background reconnect is implemented.

Paired commands include history paging, live subsequent responses, supported CLI launch, initial/follow-up prompts and bounded directory browsing. Imported history is labeled read-only. Prompting requires unique live ownership and observed idle/approval restrictions, including the first prompt before native history exists. No remote shell, credential access or permission escalation. Chat travels only through E2EE, never metadata sync.

Public-relay/browser automated acceptance exists. Physical-phone, independent security review and production approval remain open. Do not imply the relay hides connection metadata or is independently certified.

### 3.15.1 Withdrawn cloud collectors

Hosted vendor-cloud-session collectors and legacy unencrypted cloud command intake are withdrawn. Existing migration/history records are retained. Do not reintroduce credential capture or remote commands through metadata sync.

### 3.16 Profile and navigation

Workspace is chat-first with independently collapsible conversation navigation and local terminal. Dashboard groups usage/reporting; Profile groups appearance/account/devices/exports. Desktop fine-pointer rail expands on hover/focus; touch uses an explicit toggle. Pairing disclosures/input survive polling. Phone uses visual-viewport-owned chat/composer and modal folder selection. All-time reporting uses at most 30 daily, then 30 ISO-weekly, then 30 monthly intervals before yearly grouping; omit unobserved intervals and preserve totals/trust.

## 4. Engineering standards

Follow [AGENTS](../AGENTS.md), installed engineering skills and relevant ADRs. Rust edition 2024, workspace MSRV/lints, typed library errors, no non-test unwrap/expect, Tokio only in approved orchestration crates, unsafe restricted to audited platform code. Tests use temp directories and anonymized fixtures, never actual agent homes. No emojis in product code, copy or documentation.

## 5. Roadmap and current delivery

| Area | Implemented | Remaining evidence/work |
| :--- | :--- | :--- |
| M0/M1 foundation, adapters, store and CLI | Present | Current real fixture/agent-version coverage |
| M2 daemon, reports and local console | Present | Platform/resource release acceptance |
| M3 desktop/tray/distribution | Electron shell and Windows development packages | Signed/approved public artifacts, macOS/Linux acceptance |
| M4 local integration/automation | Local access free; rules/queue/relay present | Real native quota/resume and permission acceptance |
| M5 account sync / Teams | Opt-in owner snapshot; schema foundations | Privacy minimization and complete team product |
| M6 launcher/terminal | Independent owner, official CLI launch and local xterm | Real provider and cross-platform lifecycle checks |
| M7 chat/file/diff | Redacted local history, safe chat and inspection | Retention/deletion acceptance |
| M8 routing/handoff | New-session briefing path and policy controls | Authenticated cross-model/provider outcome confirmation |
| M9 encrypted Remote | Relay, pairing, remembered trust and bounded commands | Physical phone, independent security and owner production approval |
| Marketing/documentation | Outcome-first positioning, guide/legal routes and SEO in this phase | Production crawl checks and legal approval |

v1.0 criteria are not met. See [LAUNCH_CHECKLIST](LAUNCH_CHECKLIST.md) for blockers, causes, closure steps and evidence. Historical green checks are not new verification for this phase.

## 6. Deliverables and artifacts

### 6.1 Repositories

Source: `mwahaj36/spawnt`; releases: `mwahaj36/spawnt-releases`. Preserve privacy until owner authorizes public launch. Brand/config values originate in `web/branding.json` and generated outputs.

### 6.2 Release artifacts

CLI, Electron packages and notices/EULA/hashes per approved platform. Target npm shim plus seven platform packages: Windows/macOS x64+arm64, Linux x64+arm64 and Linux x64 musl. Targets are not claims of publication. Download links follow actual approved assets.

### 6.3 Versioning

CLI, desktop and npm use lockstep SemVer. Remain 0.x before v1.0. Current development version is 0.9.0. No legacy executable/home migration or compatibility alias; preserve old directories without deleting them.

### 6.4 Updates

Opt-out update-manifest check and approved signed/hash-verified release flow. Do not change public update channels when refreshing draft artifacts.

### 6.5 Website and docs

Next.js marketing presents launch, continuity, encrypted Remote and free-local value. Every route exports unique metadata; indexable pages use canonical/OG/Twitter fields. Sitemap contains actual public content routes; console/auth/settings are excluded and noindexed. Shared docs guide pages answer the question first, cite vendor sources where needed and link related content. Privacy and Terms live in marketing navigation, not the console rail. No analytics. Product limitations belong in technical docs and launch gates, without invented marketing availability claims.

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
| [0004](./adr/0004-cloud-session-collectors.md) | Portable opt-in cloud collectors, measured usage imports, isolated cloud presence rows | Withdrawn in v1.4.1 |
| [0005](./adr/0005-browser-device-pairing.md) | Independent browser login, encrypted one-time transfer, private credential persistence | Pairing withdrawn in v1.4.1; generic credential protection retained |

## 10. Open decisions

- Final Remote/Teams prices, payment/refund process and governing jurisdiction.
- Field-level metadata sync consent/minimization and migration approach.
- Launch platform list and signing/notarization policy.
- Security reviewer and production Remote approval.
- Exact Teams scope and official local model catalog sources beyond Codex.
- Retention configuration/scheduling and deletion/backups contract.

Changes and verification are recorded per session under [docs/sessions](sessions/README.md), rather than appended to this specification.
