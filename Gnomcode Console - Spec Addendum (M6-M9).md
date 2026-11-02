# Gnomcode Console - Spec Addendum (M6-M9)

Draft for merging into SPEC.md. Owner: Wahaj. Written 2026-10-28. Items marked *(verify)* depend on third-party tools that change often; check them against current docs before building.

## 0. Decisions at a glance

| Question | Decision | Why |
| :-- | :-- | :-- |
| Native app or VS Code fork? | **Native Tauri v2 app**, not a fork | You already have the Tauri shell and React bundle. A fork means rebasing Code OSS every month, no official Marketplace, and competing with Cursor and Windsurf on the editor, which is not your edge. |
| Does it include a code editor? | **No editor. Read-only file viewer, diff review, terminal, and an Open in VS Code/Cursor button.** | Keeps scope small. Editing, LSP and extensions are years of work. |
| How do you drive agents? | **Official CLIs, through structured protocols (ACP first), with raw terminal mode as the fallback.** | Spawning the vendor binary keeps subscription login with the vendor, and the structured stream gives you a real chat UI. |
| Where does chat history live? | **In the agents' own logs. Gnomcode does not create a second copy.** | You already parse these logs. Replay is a re-read, which keeps your privacy promise intact. |
| Pricing | **Everything local is free. Paid: Remote and Teams.** | Remote and sync are the only features with a recurring server cost. |
| First thing to ship | **Launcher + Claude/Codex chat view, with failover and resume in the existing tray.** | Fastest path to a demo and to real feedback. |

## 1. Product direction

### 1.1 One-liner

**One console for every AI coding subscription. Chat with any agent, see your quotas, and never lose a session to a rate limit.**

### 1.2 What changes from SPEC v0.8

- Gnomcode stops being only an observer. It can also **launch and host** agent sessions.
- All local features become free: guardrails, Resume Queue, Context Relay, custom alerts, export, failover.
- Paid features are the ones that cost you server money or serve organizations: **Remote** (phone access) and **Teams**.
- The one-time $19 Pro license is replaced by a subscription for Remote. Keep an optional one-time supporter license if you want it (it costs nothing to offer).
- The privacy line changes from 'never see content' to 'content never leaves your machine unless you turn on end-to-end encrypted Remote'.

### 1.3 Revised tiers

|  | **Free** | **Remote - $4.99/mo or $39/yr** | **Teams - $9/seat/mo (min 2)** |
| :-- | :-- | :-- | :-- |
| Tray, dashboard, live state, quotas, notifications | Yes | Yes | Yes |
| Console: chat view, terminal, file viewer, diff review | Yes | Yes | Yes |
| Launcher, worktrees, model/agent picker | Yes | Yes | Yes |
| Auto-switch (same family and cross-vendor), Resume Queue, Context Relay | Yes | Yes | Yes |
| Guardrails, custom alerts, CSV/JSON export | Yes | Yes | Yes |
| Phone access (PWA): view, prompt, approve | - | Yes | Yes |
| Multi-machine quota mirror | - | Yes | Yes |
| Org rollups, budgets, Slack/Discord alerts, roles | - | - | Yes |

Suggested launch offers: 14-day Remote trial, first-6-months discount ($2.99), capped founding lifetime deal (first 100 buyers), student discount, regional pricing. Prices are suggestions, not advice; change them after you see conversion.

## 2. Native app vs VS Code fork

| Factor | Native Tauri app (recommended) | VS Code fork |
| :-- | :-- | :-- |
| Reuses what you have | Tray, daemon API, React bundle, adapters | Almost nothing; new TypeScript/Electron codebase |
| Maintenance | You own a small app | Rebase Code OSS regularly, patch conflicts |
| Extensions and Marketplace | Not needed | Forks cannot use the official Marketplace; Open VSX only |
| Memory footprint | Small (Tauri webview) | Electron + extension host, heavy |
| Rust-only logic (license, rules, routing) | Natural fit | Awkward, logic leaks into TypeScript |
| Competes with | Vendor desktop apps, JetBrains Air | Cursor, Windsurf, Copilot, Zed |
| Time to first demo | Weeks | Months |

**When a fork would make sense:** only if users clearly demand full editing inside Gnomcode, and you have a funded team. Not now.

**Cheap extra distribution later:** a small VS Code/Open VSX **companion extension** that shows Gnomcode state in the status bar and opens the console. It is days of work, not a fork.

## 3. Console architecture

### 3.1 Principles

1. **Official binaries only.** Gnomcode spawns `claude`, `codex`, `gemini` etc. The user logs in through the vendor. Gnomcode never extracts, stores or forwards subscription tokens.
2. **Two modes per session.** Chat mode (structured) and Terminal mode (raw PTY). Same session, switchable.
3. **Observability never depends on the driver.** Every session, however launched, is still ingested from logs. A session started in your own terminal shows up the same way.
4. **No second copy of conversations.** The chat view renders from the live event stream plus agent logs.
5. **Permissions are never auto-escalated.** Same rule as Resume Queue (SPEC section 3.14).
6. **Fail open.** If Gnomcode crashes, agent processes keep running and nothing is lost.

### 3.2 Dual-mode sessions

| Mode | How it works | Use when |
| :-- | :-- | :-- |
| Chat | Driver speaks a structured protocol to the agent; UI renders messages, tool cards, permission prompts | Agent supports it |
| Terminal | PTY runs the agent's own TUI in xterm.js | No structured protocol exists, or user wants full native fidelity |

Both appear in the same session list. Switching modes for the same session is a *resume* of the same native session ID where the agent supports it.

### 3.3 Agent drivers

| Agent | Chat driver | Fallback | Notes |
| :-- | :-- | :-- | :-- |
| Gemini CLI | ACP, built in *(verify flag name)* | Terminal | Google was the first external ACP integration. |
| Codex | `codex-acp` adapter (Zed, Apache-2.0); also look at Codex's own app-server JSON-RPC and `codex exec --json` *(verify)* | Terminal | Adapter supports ChatGPT subscription login per its README *(verify)*. |
| Claude Code | Zed's Claude Agent ACP adapter *(verify)*; alternative: `claude -p --output-format stream-json` headless mode *(verify)* | Terminal | Adapter is built on Anthropic's SDK. **Check Anthropic's current terms on subscription use through SDK-based tools before relying on it.** Terminal mode with the official TUI is the safest default. |
| Antigravity | None known | Observe only | IDE-hosted: notify and focus window (SPEC 3.13). |
| Cline/Roo, Cursor, Aider | None | Observe, or terminal for CLIs | Tier B/C as before. |

ACP (Agent Client Protocol) was created by Zed, is developed openly with JetBrains, and is supported by many editors and agents, so one driver implementation covers several agents. It does not carry quota data, so the existing log-based quota engine stays the source for limits.

### 3.4 New crates

```
crates/
  agents/      # spawn, supervise and talk to agent processes (ACP client, stream-json, PTY)
  convo/       # ConsoleEvent model, in-memory transcript, replay from adapters
  router/      # quota-aware routing, failover, family switching, handoff packet builder
  workspace/   # worktrees, git status/diff, file reads, path sandboxing
  term/        # portable-pty sessions, resize, scrollback ring buffer
  remote/      # relay client, pairing, E2EE (M9)
```

Dependency rule stays: `core` depends on nothing internal; `agents`, `router`, `workspace` depend on `core`; `daemon` wires them; UI talks only to the daemon API.

### 3.5 Canonical console event model

The UI and the phone render the same stream. Sketch:

```rust
enum ConsoleEvent {
    SessionStarted { session_id, agent, cwd, model },
    UserMessage { text_ref },            // content held in memory only
    AssistantDelta { text },
    ToolCall { id, kind, title, status, input_redacted },
    ToolResult { id, status, output_ref },
    FileEdit { path, diff_ref },
    PermissionRequest { id, tool, risk, options },
    PlanUpdate { items },
    ModelChanged { from, to, reason },
    UsageDelta { tokens, cost_usd, basis },
    StateChanged { state },              // thinking | tool | awaiting_approval | idle | error
    RateLimited { limit_id, resets_at },
    Handoff { from, to, packet_ref },
}
```

`*_ref` values point to in-memory or agent-log locations. The persistent store keeps only the metadata it keeps today.

### 3.6 Conversation storage and privacy

- Default: **no persistence of message text by Gnomcode.** History view = re-parse the agent's own log through the existing adapter.
- Optional setting: *Keep an encrypted local transcript cache* (off by default) for faster search.
- Update SPEC section 1.6 and 3.15.D: content never uploads unless Remote is on, and then only as ciphertext the relay cannot read (section 6).
- Keep `gnomcode doctor --network` and extend it to list the relay endpoint.

### 3.7 Permissions

- Chat view shows every agent permission request as a card: tool, arguments (redacted), risk level, and Approve / Deny / Approve for session.
- Gnomcode never answers a permission request on its own. Resume and failover runs inherit the user's existing permission settings.
- Risk levels (computed locally): read-only, workspace write, shell, network, destructive (rm, force-push, outside workspace).
- **Phone rule (default):** the phone can approve read-only and workspace-write only. Shell/network/destructive require the desktop, or a second confirmation with device biometrics. Teams admins cannot loosen this remotely.

### 3.8 Router: auto-switch, family change, auto-resume

Inputs: quota pools per `(provider, limit_id)`, burn velocity, session state, user policy.

| Policy | Behavior | Default |
| :-- | :-- | :-- |
| `same_family_step_down` | Opus to Sonnet (or equivalent) inside the same vendor and session | `ask_first` |
| `same_vendor_account` | Not supported (account sharing risk) | Off |
| `cross_vendor_failover` | Claude to Codex to Gemini in a user-defined chain | `ask_first` |
| `wait_and_resume` | Wait for reset, then resume (Resume Queue) | On when armed |
| `disabled` | Manual |  |

Rules:

1. Trigger on **forecast** (pool at user threshold, default 90%) or on an actual `rate_limited` event.
2. Prefer in this order: wait for a near reset, same-family step-down, cross-vendor failover.
3. `ask_first` shows a card in the console, tray and phone with one tap to accept. `auto` is opt-in per project.
4. Every switch is logged as an `activity_event` and shown in the chat as a visible divider, so the user always knows which model wrote what.
5. Never switch mid tool call. Wait for the tool result, then rotate.

### 3.9 Handoff packet (zero extra tokens)

Built by Gnomcode, not by an LLM, from data it already has:

- Goal: the user's first prompt of the task and the last two prompts.
- Work done: ordered list of files changed (`git diff --stat`), commits made.
- Open items: latest plan/todo list from the event stream.
- State: branch, `git status`, last failing command and exit code.
- Instructions for the next agent: continue the exact next action; do not redo finished work; verify tests first.

Written to `.gnomcode/HANDOFF.md` in the worktree (existing Context Relay design) and passed as the first prompt to the new agent. Optional: at the safety-reserve threshold, ask the outgoing agent to refresh the file itself. Expect a quality dip and a cost spike on the first turn after a cross-vendor switch; say so in the UI.

## 4. Console UI spec

### 4.1 Layout

```
+----------------+-----------------------------+-------------------+
| Sidebar        | Center: tabs                | Right panel       |
| Projects       |  Chat | Terminal            |  Files | Diff     |
|  Sessions      |                             |  Plan | Context   |
|  state badges  |                             |                   |
| Quota pools    | Composer                    |                   |
+----------------+-----------------------------+-------------------+
| Bottom: user shell terminal (collapsible)                       |
+-----------------------------------------------------------------+
```

Top bar: project, branch, agent, model picker with quota badge, context meter, cost ticker.

### 4.2 Chat view (MVP list)

- Streaming markdown with syntax highlighting.
- Tool cards: collapsible; shell command with output, file read, search, web fetch.
- File-edit cards with inline diff and a link to the full diff view.
- Permission cards (section 3.7).
- Plan/todo panel updated live.
- Subagent tree (parent session with children).
- Model/agent picker showing quota headroom per pool; one-click switch with handoff.
- Composer: @file mentions, image paste, slash-command passthrough, message queue while the agent is busy, Stop button.
- Session divider cards for model switches, handoffs and auto-resumes.
- Cost and context meters per session; context bloat warning at 75% (existing).
- Command palette and keyboard shortcuts.

### 4.3 File viewer and diff review (built in, read-only)

- File tree scoped to the worktree; click to open.
- Syntax-highlighted viewer; jump from a tool card to the exact file and line.
- Diff view per session: all files changed since session start (computed by git against the base commit), hunk by hunk.
- Review actions use git: **Revert hunk / Revert file**, **Stage**, **Commit with message**. No in-app text editing.
- Open in VS Code / Cursor / default editor button on every file.
- Path sandbox: viewer only reads inside the session's worktree; symlinks resolved and checked.

### 4.4 Terminal panel

- xterm.js (WebGL renderer) connected to a Rust PTY through the daemon.
- Two uses: the agent's own TUI in Terminal mode, and a plain user shell.
- Windows: ConPTY; macOS/Linux: standard PTY. Test resize, unicode and long scrollback early.
- Scrollback ring buffer kept in memory only (size configurable).

### 4.5 Sidebar

Projects (grouped by canonical Git remote, as today), sessions with live state badges (\[RUN\], \[WAIT\], \[IDLE\], \[ERR\]), rate-limited sessions with a one-click Arm Resume, quota pools with reset timers.

### 4.6 Explicitly not building

Code editing, LSP, debugger, extension host, built-in git GUI beyond revert/stage/commit, notebooks, remote dev containers, a custom model API proxy.

## 5. Stack

| Layer | Choice | Notes |
| :-- | :-- | :-- |
| Shell | Tauri v2 (existing) | Lazy-create windows; keep the memory story in your risks table. |
| UI | React + Vite + Tailwind (existing), shadcn/ui, Zustand + TanStack Query | Pure view over daemon JSON, as SPEC 6.8 requires. |
| Markdown/code | react-markdown, Shiki | Shiki for highlighting in chat and viewer. |
| Viewer/diff | Monaco, lazy-loaded, read-only (diff editor included) | CodeMirror 6 merge view is the lighter alternative if size matters. |
| Terminal | xterm.js + WebGL addon |  |
| PTY | `portable-pty` crate | ConPTY on Windows. |
| Agent protocol | Rust ACP client (official SDK crate, or a thin JSON-RPC client you write) | Check the SDK's maturity *(verify)*. |
| Git | Shell out to the `git` CLI for worktree/diff/status; `gix` for fast reads | The CLI is the most reliable for worktrees. |
| FS watching | `notify` (existing) |  |
| Local API | axum + WS (existing) | Add `/v1/console/*` endpoints. |
| Remote relay | Cloudflare Workers + Durable Objects (WebSocket fan-in/out) | Cheap, no servers, ciphertext only. |
| Remote crypto | Noise protocol (`snow` crate) or X25519 + HKDF + ChaCha20-Poly1305 | Never roll your own primitives. |
| Auth, orgs, billing metadata | Supabase (existing plan) |  |
| Payments | Lemon Squeezy, Paddle as fallback (existing) | Subscriptions need webhook-driven entitlement. |
| Phone client | PWA first (React, same bundle) | No app store fees; installable; add native later if needed. |

## 6. Remote access (phone)

### 6.1 Architecture

```
Phone PWA <--E2EE--> Relay (Worker + Durable Object) <--E2EE--> Desktop daemon
```

The relay forwards sealed frames between two paired devices. It stores no message content.

### 6.2 Pairing

1. Desktop shows a QR code containing its public key and a one-time room ID.
2. Phone scans, generates its own keypair, and both derive session keys.
3. The user confirms a short verification code (6 digits) shown on both screens.
4. Keys are stored in the OS keystore (desktop) and IndexedDB with a non-extractable WebCrypto key where possible (phone).
5. Revoke from the desktop or the web dashboard at any time; revocation rotates the room.

### 6.3 What the phone can do

View sessions, state and quotas; read the chat stream; send a prompt; approve low-risk permission requests; accept a failover or resume card; arm or cancel Resume Queue entries. It cannot browse arbitrary files or run raw shell commands.

### 6.4 Threat model (write the real one before launch)

| Threat | Mitigation |
| :-- | :-- |
| Stolen unlocked phone | Biometric re-auth for approvals, short idle lock, one-tap revoke from desktop |
| Compromised relay | E2EE; relay sees only sizes, timing and room IDs |
| Malicious pairing attempt | One-time room, verification code, short expiry |
| Token theft | Short-lived signed tokens, device-bound keys |
| Prompt injection from repo content causing risky tool calls | Existing permission model; phone cannot approve dangerous tools |
| Replay of old commands | Per-message counters and nonces |

Get an independent review of the crypto and pairing design before charging for it. This is the one place where agentic coding output should be read line by line.

### 6.5 Entitlement

Remote is enabled by a signed entitlement from your license Worker (subscription active). Local features never check entitlement.

## 7. Teams

Keep the M5 schema (organizations, members, devices, hourly usage sync, quota sync, budget rules). Changes: price and tier per section 1.3; guardrails are now free locally, and Teams adds **org-wide budgets and hard-stop policy distribution**. Project names stay hashed by default. Remote content features are not available to admins; admins see aggregates only.

## 8. Legal and terms guardrails

- Re-read each vendor's current terms for third-party tools that use subscription login *(verify, they change)*. Design rule: Gnomcode launches the official client and never handles subscription tokens.
- Your existing opt-in feature that reads Claude OAuth credentials for a usage endpoint (SPEC section 3.9) carries the same risk. Keep it opt-in and labeled experimental, or remove it.
- Remote controls the user's own machine and own session; do not build account sharing or pooling features.
- Have the EULA, privacy policy and refund policy reviewed before charging (open question 3 in SPEC).

## 9. Roadmap (rough, part-time with coding agents)

Durations are guesses to help you plan, not commitments.

| Milestone | Scope | Exit criteria | Est. |
| :-- | :-- | :-- | :-- |
| **M5.5 Launch the free tool now** | Ship v0.8.x publicly, demo GIF, waitlist for Remote/Teams, update pricing in SPEC | First 100 installs, 20 feedback conversations | 1-2 wks |
| **M6 Launcher + terminal** | Start agents from tray/dashboard in a repo or worktree; embedded xterm terminal mode; model/agent picker with quota badges | Start, stop and focus Claude/Codex/Gemini sessions on all 3 OSes | 3-4 wks |
| **M7 Chat view** | ACP driver (Gemini, Codex), then Claude; tool cards, permission cards, file viewer, diff review | Full chat session with approvals on Windows and one Unix OS; no content persisted | 4-6 wks |
| **M8 Router** | Same-family step-down, cross-vendor failover with handoff packet, console-hosted resume | Rate-limited session continues in another agent with a correct handoff, no permission escalation | 3-4 wks |
| **M9 Remote** | Relay, pairing, E2EE, PWA, low-risk approvals, Lemon Squeezy subscriptions | External crypto review done; paid beta with 20 users | 6-8 wks |
| **M10 Teams** | Finish M5 items, org budgets, Slack/Discord | 3 paying teams | after traction |

**Checkpoint after M7 (about 3 months):** if free-tier weekly active installs are low and nobody asks for chat or remote, stop adding scope and move to maintenance. Cheap test before M9: measure waitlist signups.

## 10. Growth built into the product

- Shareable **Plan Value card** (ROI image with footer) and a monthly recap.
- A **Saved by Gnomcode** notification with a one-click share when failover or resume rescues a session.
- `npx gnomcode` works without an account; winget, Scoop, Homebrew and AUR listings.
- Site pages answering real queries: how to check Claude Code usage, Codex 5-hour vs weekly limits, how to resume after a rate limit.
- Weekly routine (3-4 hours): one release post, replies in two or three communities, fast issue replies, one metric (weekly active installs).

## 11. SPEC.md edit checklist

1. Section 1.5 tiers and licensing model.
2. Section 1.6 non-goals (content rule) and 3.15.D privacy boundary.
3. Section 3.3 add console processes; 3.4 add crates.
4. Section 3.14 and 3.14.1: now free; add router behavior.
5. Section 6.7 and 6.8: license checks only gate Remote entitlement; remove the distributed Pro-feature checks.
6. Section 5: add M6-M10, mark M5 scope.
7. New ADRs: 0005 native console over VS Code fork; 0006 structured agent drivers (ACP) with terminal fallback; 0007 E2EE relay design; 0008 free local core with paid Remote and Teams.

## 12. Risks

| Risk | Mitigation |
| :-- | :-- |
| Vendor terms change for subscription use in third-party tools | Official binaries only, terminal-mode fallback, no token handling |
| ACP adapters break or lag behind agent releases | Pin versions, golden session fixtures, terminal fallback |
| Remote security incident | E2EE, independent review, strict phone permission limits |
| Vendors ship their own failover and cross-agent consoles | Stay neutral, lead on quota-aware routing and Windows/Linux polish |
| Scope creep into an IDE | Section 4.6 is the rule; revisit only with clear user demand |
| Solo bandwidth and burnout | Checkpoint after M7, fixed weekly marketing routine |

## 13. Open questions

1. Exact ACP flags and adapter versions for Claude, Codex and Gemini today.
2. Whether Anthropic permits subscription login through SDK-based adapters, or only the official client.
3. Monaco versus CodeMirror for the viewer, based on measured memory on Windows.
4. Whether to start Remote as a Cloudflare-only relay or add optional direct peer-to-peer.
5. Final prices after the first conversion data.
