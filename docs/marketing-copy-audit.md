# Copy audit ledger

Source locations refer to this change. Rows pair removed/added wording in source order; where a section was restructured, read the file diff for exact placement. New guide/legal articles are original copy, listed separately below. Client page extraction preserves original console/auth/settings behavior.

| File:line | Original | Replacement |
| :--- | :--- | :--- |
| crates/daemon/src/console-app.js:232 | Exporting analytics requires a Pro license. Activate Pro in Automations. | Export is unavailable. Reopen the local console and try again. |
| crates/daemon/src/console-app.js:236 | Failed to export data. | Export failed. Reload the view and try again. |
| crates/daemon/src/console-app.js:249 | Export failed:  | Export failed. Reload the view and retry. Details:  |
| crates/daemon/src/console-app.js:396 | Context Relay configuration requires an active gnomcode Pro license. | Could not save handoff settings. Reopen the local console and try again. |
| crates/daemon/src/console-app.js:403 | Failed to save config:  | Settings could not be saved. Check the connection and retry. Details:  |
| crates/daemon/src/console-app.js:416 | Context Relay execution requires an active gnomcode Pro license. | Could not start the handoff. Reopen the local console and check the agent session. |
| crates/daemon/src/console-app.js:420 | Context Relay handoff executed successfully! A fresh session was launched with a repository handoff. | Handoff launched a fresh session. Check the target conversation and reported model. |
| crates/daemon/src/console-app.js:423 | Failed to execute relay handoff. | Handoff failed. Check the target CLI in Terminal stream and try again. |
| crates/daemon/src/console-app.js:977 |  [Failed to open terminal stream] |  [Terminal stream unavailable. Reopen the session and try again.] |
| crates/daemon/src/console-app.js:1026 | Could not load conversation history. | Could not load conversation history. Reload the view and check the desktop connection. |
| web/app/LandingClient.tsx:19 | See it live | Aider |
| web/app/LandingClient.tsx:28 | Every session, its status and which ones need you, in one list. | Hit a limit. Keep the task moving. |
| web/app/LandingClient.tsx:28 | Know the cost | Switch supported agents with a compact briefing of objectives and changed files. Keep the original history and continue with another model. |
| web/app/LandingClient.tsx:29 | Tokens and spend per agent, model and repo. Every number says how sure we are. | Away from your desk. Still in control. |
| web/app/LandingClient.tsx:29 | Stay under limits | Pair your phone, read the conversation and send the next instruction over an end-to-end encrypted channel. |
| web/app/LandingClient.tsx:30 | Track what is left in each plan and pick up where you stopped when it resets. | One fleet. One view of usage. |
| web/app/LandingClient.tsx:30 | Planned | Compare recorded tokens, estimated API-equivalent spend and quota across agents. Every measurement carries its trust level. |
| web/app/LandingClient.tsx:34 | paid feature | paid access |
| web/app/LandingClient.tsx:34 | Encrypted access planned | End-to-end encrypted access |
| web/app/LandingClient.tsx:35 | Planned | paid category |
| web/app/LandingClient.tsx:35 | paid feature | Shared visibility |
| web/app/LandingClient.tsx:35 | Team dashboard | Product direction in docs |
| web/app/LandingClient.tsx:35 | Shared budgets | Terms before purchase |
| web/app/LandingClient.tsx:141 | Team alerts | The runtime for your AI coding agents. |
| web/app/LandingClient.tsx:141 | See, stop, and control your agents. | Launch agents, switch models mid-task and send the next instruction from your phone over an encrypted connection. |
| web/app/LandingClient.tsx:141 | Know who is blocked on approval, halt runaway loops before they drain your API bill, and auto-resume at quota reset. | Download for Windows |
| web/app/LandingClient.tsx:141 | Download gnomcode | Windows, macOS and Linux. All local features free. |
| web/app/LandingClient.tsx:150 | Local-first. Cloud sync is optional. | Keep working across agents. |
| web/app/LandingClient.tsx:172 | One place to see it all. | Local by default. Encrypted when remote. |
| web/app/LandingClient.tsx:172 | Your logs stay on your machine. | Local history |
| web/app/LandingClient.tsx:172 | Local only | Keep redacted conversations on your desktop. |
| web/app/LandingClient.tsx:172 | Reads logs on your machine. | Encrypted Remote |
| web/app/LandingClient.tsx:172 | Metadata only | Conversations reach your paired device through E2EE. |
| web/app/LandingClient.tsx:172 | Activity metadata is redacted before storage. | No product analytics or tracking. |
| web/app/LandingClient.tsx:172 | No product telemetry. Optional services are disclosed. | Your choice |
| web/app/LandingClient.tsx:172 | Loopback | Account metadata sync is optional and separately disclosed. |
| web/app/LandingClient.tsx:174 | Only your machine can reach it. | Remote access and Teams are paid categories. Provider inference is billed separately. |
| web/app/LandingClient.tsx:176 | Remote and Teams are planned paid features. | Choose your platform. |
| web/app/LandingClient.tsx:178 | Start in a minute. | Questions before you start? |
| web/app/LandingClient.tsx:179 | Download gnomcode and open your first session. | Open your next session. |
| web/app/LandingClient.tsx:179 | Know what is running. | Bring your agents into one workspace. |
| web/app/LandingClient.tsx:183 | gnomcode reads local logs and stores redacted monitoring metadata. Local chat history is planned. | Keep your agents moving. |
| web/app/LandingClient.tsx:183 | (added) | Launch, switch and control AI coding agents. Local-first, with encrypted access from paired devices. |
| web/app/LandingClient.tsx:186 | (added) | Terms |
| web/app/components/ConsoleTourView.tsx:43 | RUN | Running |
| web/app/components/ConsoleTourView.tsx:43 | WAIT | Waiting |
| web/app/components/ConsoleTourView.tsx:43 | IDLE | Idle |
| web/app/docs/page.tsx:7 | Documentation — Setup, Agents & API Reference | Gnomcode Documentation |
| web/app/docs/page.tsx:7 | Complete technical guide for gnomcode: quickstart, agent support, CLI commands, and REST APIs. | Install Gnomcode, launch AI coding agents, switch models and pair your phone. Guides for agent support, cost, quota and privacy. |
| web/app/docs/page.tsx:26 | gnomcode is live mission control for AI coding agents. Current monitoring reads local agent activity and stores redacted metadata. Local conversation history is planned for model switching. | Getting started |
| web/app/docs/page.tsx:64 | gnomcode runs locally with no external runtime dependencies. Choose an available platform release below: | Gnomcode is a local-first agent orchestrator. Launch supported official CLIs, work in redacted local conversations, switch with a compact handoff and pair a device for encrypted Remote. |
| web/app/docs/page.tsx:69 | Current monitoring stores redacted activity and usage metadata. A future console will keep local conversation history for session continuity and model switching. Existing optional metadata sync is not end-to-end encrypted and must not carry chat content. | Gnomcode guides |
| web/app/docs/page.tsx:69 | Before account synchronization, local file paths are truncated to directory basenames, and full paths are irreversibly hashed using HMAC-SHA256 with a local machine key. | Choose a workflow, then check capability and release details. |
| web/app/docs/page.tsx:69 | (added) | Getting started |
| web/app/docs/page.tsx:69 | (added) | Model switching |
| web/app/docs/page.tsx:69 | (added) | Remote access |
| web/app/docs/page.tsx:69 | (added) | Supported agents |
| web/app/docs/page.tsx:69 | (added) | Quota and cost |
| web/app/docs/page.tsx:69 | (added) | Model pricing |
| web/app/docs/page.tsx:69 | (added) | Automate multiple agents |
| web/app/docs/page.tsx:69 | (added) | Prompt from phone |
| web/app/docs/page.tsx:69 | (added) | Quota limits explained |
| web/app/docs/page.tsx:69 | (added) | Privacy |
| web/app/docs/page.tsx:69 | (added) | Claude Code vs Codex |
| web/app/docs/page.tsx:75 | (added) | gnomcode runs locally with no external runtime dependencies. Check the platform guide for current release availability: |
| web/app/docs/page.tsx:171 | (added) | Gnomcode stores redacted local chat for continuity. Chat events are excluded from non-E2EE account metadata sync. Paired Remote carries conversations over a separate encrypted channel. |
| web/app/docs/page.tsx:178 | (added) | The live account snapshot includes project/session labels and automation metadata. HMAC rollup helpers do not make it hash-only. Read the privacy guide before enabling sync; field-level minimization remains a launch gate. |
| web/app/docs/page.tsx:366 | (added) | Legal |
| web/app/docs/page.tsx:366 | (added) | Privacy |
| web/app/docs/page.tsx:366 | (added) | Terms of Service |
| web/app/docs/page.tsx:366 | (added) | describe storage, optional services and licensing. |
| web/app/downloads/PlatformDownloads.tsx:15 | NOT PUBLISHED | Launch official coding agents, continue conversations and keep a separate workspace terminal close at hand. |
| web/app/downloads/PlatformDownloads.tsx:17 | Downloads will appear after release verification. | Read the installation guide for current release channels and verification steps. |
| web/app/downloads/PlatformDownloads.tsx:20 | NOT PUBLISHED | APPLE SILICON + INTEL |
| web/app/downloads/PlatformDownloads.tsx:21 | A verified macOS release is not available yet. We will publish an installer after native builds and signing are ready. | Bring your agent conversations, recorded usage and model-switch workflow into one local workspace. |
| web/app/downloads/PlatformDownloads.tsx:22 | No download path is enabled for macOS today. | Read the installation guide for platform requirements and release channels. |
| web/app/downloads/PlatformDownloads.tsx:26 | NOT PUBLISHED | Use the standalone CLI and local console to orchestrate agents and inspect usage on your own machine. |
| web/app/downloads/PlatformDownloads.tsx:28 | Downloads will appear after release verification. | Read the installation guide for current release channels and verification steps. |
| web/app/downloads/PlatformDownloads.tsx:93 | There is no verified installer to download yet. | Choose your installation path in the |
| web/app/downloads/PlatformDownloads.tsx:93 | (added) | getting started guide |
| web/app/downloads/page.tsx:7 | Gnomcode downloads are being prepared. | Download Gnomcode — Windows, macOS, Linux |
| web/app/downloads/page.tsx:7 | Gnomcode desktop and CLI builds are being verified. Your agent logs stay on your machine. | Download the Gnomcode AI agent orchestrator for your platform. All local features are free, with no account required. |
| web/app/downloads/page.tsx:15 | NEXT RELEASE · v | One place to launch, switch and control AI coding agents. Choose Windows, macOS or Linux. |
| web/app/downloads/page.tsx:21 | (added) | By downloading, you agree to the |
| web/app/downloads/page.tsx:21 | (added) | Terms of Service |
| web/app/downloads/page.tsx:21 | (added) | . Read our |
| web/app/downloads/page.tsx:21 | (added) | Privacy notice |
| web/app/layout.tsx:10 | gnomcode — Mission Control for AI Coding Agents | Gnomcode — The Runtime for AI Coding Agents |
| web/app/layout.tsx:11 | Live mission control, token cost attribution, per-model quota pools, and flight recorder for Claude Code, OpenAI Codex, and Google Antigravity across Windows, macOS, and Linux. | Launch, switch and remotely control AI coding agents. Work across Claude Code, Codex and Gemini CLI from one place. |
| web/app/layout.tsx:12 | AI coding agent monitoring | AI coding agents |
| web/app/layout.tsx:12 | Claude Code token tracker | Claude Code |
| web/app/layout.tsx:12 | OpenAI Codex rate limit monitor | OpenAI Codex |
| web/app/layout.tsx:12 | Google Antigravity mission control | Gemini CLI |
| web/app/layout.tsx:12 | LLM spend attribution | agent orchestrator |
| web/app/layout.tsx:12 | Prompt cache savings calculator | switch AI models |
| web/app/layout.tsx:12 | AI developer tooling | remote AI agent control |
| web/app/layout.tsx:12 | Local agent flight recorder | AI developer tools |
| web/app/layout.tsx:13 | gnomcode Team | Wahaj |
| web/app/layout.tsx:13 | gnomcode — Live Mission Control for AI Coding Agents | Wahaj |
| web/app/layout.tsx:13 | Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with local-first data handling. | Gnomcode |
| web/app/layout.tsx:51 | gnomcode — Live Mission Control for AI Coding Agents | (removed) |
| web/app/layout.tsx:53 | Monitor token usage, per-model quota pools, and live session states for Claude Code, Codex, and Antigravity with local-first data handling. | (removed) |
| web/app/layout.tsx:72 | Offer | (removed) |
| web/app/layout.tsx:77 | Live mission control, token spend attribution, and per-model quota monitoring for AI coding agents with local-first data handling. | (removed) |
| web/app/layout.tsx:80 | Organization | (removed) |
| web/app/layout.tsx:88 | Question | (removed) |
| web/app/layout.tsx:89 | Does gnomcode store my source code or prompt contents? | (removed) |
| web/app/layout.tsx:91 | Answer | (removed) |
| web/app/layout.tsx:92 | The current monitoring features store redacted metadata. Local conversation history is planned for model switching; existing optional metadata sync is not end-to-end encrypted. | (removed) |
| web/app/layout.tsx:96 | Question | (removed) |
| web/app/layout.tsx:97 | Which AI coding agents does gnomcode support? | (removed) |
| web/app/layout.tsx:99 | Answer | (removed) |
| web/app/layout.tsx:100 | gnomcode supports Claude Code, OpenAI Codex (CLI & Desktop App), Google Antigravity (IDE & Autonomous Agent), and Google Gemini CLI. | (removed) |
| web/app/page.tsx:6 | gnomcode — See what your coding agents are doing. | Gnomcode — The Runtime for AI Coding Agents |
| web/app/page.tsx:6 | Live status, cost and limits for your agents. Local by default, with optional paired desktop metadata snapshots. | Launch AI coding agents, switch models mid-task and control supported sessions from your phone. Local-first orchestration across providers. |
| web/app/page.tsx:9 | (added) | Gnomcode |
| web/app/page.tsx:9 | (added) | Person |
| web/app/page.tsx:9 | (added) | Wahaj |
| web/app/page.tsx:9 | (added) | Local-first orchestration for AI coding agents: launch, switch models and control supported sessions remotely. |
| web/app/page.tsx:9 | (added) | Offer |
| web/app/page.tsx:9 | (added) | All local features are free |
| web/app/page.tsx:10 | (added) | Question |
| web/app/page.tsx:10 | (added) | Answer |
| web/app/pricing/page.tsx:4 | All local features are free. Remote and Teams are planned paid features. | Gnomcode Pricing — Free Local, Paid Remote & Teams |
| web/app/pricing/page.tsx:4 | Planned | All local Gnomcode features are free. Remote access and Teams are paid categories. Your agent provider bills inference separately. |
| web/app/pricing/page.tsx:14 | paid feature | PAID ACCESS |
| web/app/pricing/page.tsx:14 | Encrypted remote access is planned | terms in documentation |
| web/app/pricing/page.tsx:16 | Remote is planned | End-to-end encrypted access |
| web/app/pricing/page.tsx:17 | Planned | Explore Remote |
| web/app/pricing/page.tsx:20 | paid feature | PAID CATEGORY |
| web/app/pricing/page.tsx:20 | Team dashboard and member rollups | product direction in docs |
| web/app/pricing/page.tsx:22 | Shared budgets and policies | Shared visibility is the Teams direction |
| web/app/pricing/page.tsx:22 | Role-based access | Scope and service terms in documentation |
| web/app/pricing/page.tsx:23 | Team notifications | Read the FAQ |
| web/app/pricing/page.tsx:30 | Teams is planned | Gnomcode stores redacted local history for continuity. Account metadata sync excludes chat events; paired Remote carries conversations through E2EE. |
| web/app/pricing/page.tsx:37 | Current monitoring stores redacted metadata. Local chat history is planned for session continuity and model switching. | Where can I read service terms? |
| web/app/pricing/page.tsx:38 | What will be paid? | Read the documentation for Remote and Teams service details. Your agent provider bills inference separately. |
| web/app/pricing/page.tsx:47 | Remote and Teams are planned paid features. Pricing has not been announced. | Use the CLI and local dashboard without an account. Remote access and Teams are paid categories. Provider inference is billed separately. |
| web/app/pricing/page.tsx:81 | Use the CLI and local dashboard without an account. Remote and Teams are planned paid features. | Redacted local history; optional metadata sync; encrypted Remote. |
| web/app/pricing/page.tsx:84 | Monitoring metadata is redacted. Local chat history is planned. | (removed) |
| web/app/remote/page.tsx:2 | (added) | Gnomcode Remote — Control Your Agents From Anywhere |
| web/app/remote/page.tsx:2 | (added) | Pair your phone or browser with Gnomcode to view conversations and send instructions to supported AI coding agents over an encrypted channel. |
| web/components/MarketingFooter.tsx:9 | Know what is running. | Keep your agents moving. |
| web/components/MarketingFooter.tsx:9 | gnomcode reads local logs and stores redacted monitoring metadata. Local chat history is planned. | Launch, switch and control AI coding agents. Local-first, with encrypted access from paired devices. |
| web/components/MarketingFooter.tsx:11 | (added) | Supported agents |
| web/components/MarketingFooter.tsx:12 | (added) | Terms |
| web/components/RemoteCompanion.tsx:157 | Session refresh failed. | Session refresh failed. Check the desktop connection and reconnect. |
| web/components/RemoteCompanion.tsx:204 | Remote request failed. | Remote request failed. Reconnect and try again. |
| web/components/RemoteCompanion.tsx:330 | (added) | Session ended |
| web/lib/console-presentation.js:1 | Provider measured | Running |
| web/lib/console-presentation.js:1 | Estimate | Running |
| web/lib/console-presentation.js:1 | Provider API | Running |
| web/lib/console-presentation.js:1 | Partial estimate; unpriced usage excluded | Waiting |
| web/lib/console-presentation.js:1 | Estimate | Waiting |
| web/lib/console-presentation.js:1 | Price unavailable | Idle |
| web/lib/console-presentation.js:1 | Recorded | Error |
| web/lib/console-presentation.js:1 | Unavailable | Idle |
| web/lib/console-presentation.js:6 | Price unavailable | EXACT · Provider measured |
| web/lib/console-presentation.js:6 | Price unavailable | API · Provider API |
| web/lib/console-presentation.js:6 | Recorded | UNKNOWN |
| web/lib/console-presentation.js:9 | Unavailable | EST · Partial estimate; unpriced usage excluded |
| web/lib/console-presentation.js:15 | Price unavailable | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:30 | Recorded | UNKNOWN |
| web/lib/console-presentation.js:31 | Unavailable | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:32 | Price unavailable | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:33 | Price unavailable | UNKNOWN |
| web/lib/console-presentation.js:39 | Recently idle | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:51 | Provider measured | EXACT |
| web/lib/console-presentation.js:51 | Provider API | UNKNOWN |
| web/lib/console-presentation.js:51 | Estimate | UNKNOWN |
| web/lib/console-presentation.js:51 | Unavailable | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:69 | Unavailable | Price unavailable · UNKNOWN |
| web/lib/console-presentation.js:77 | (added) | Idle |
| web/lib/console-presentation.js:132 | (added) | EXACT · Provider measured |
| web/lib/console-presentation.js:132 | (added) | API · Provider API |
| web/lib/console-presentation.js:132 | (added) | UNKNOWN |
| web/lib/console-presentation.js:132 | (added) | UNKNOWN |

## New content

Original: no page. Replacement: complete authored content in each file below (all paragraphs/headings are additions).

- web/app/docs/automate-multiple-agents/page.tsx:1
- web/app/docs/claude-code-vs-codex/page.tsx:1
- web/app/docs/faq/page.tsx:1
- web/app/docs/getting-started/page.tsx:1
- web/app/docs/model-pricing/page.tsx:1
- web/app/docs/model-switching/page.tsx:1
- web/app/docs/privacy/page.tsx:1
- web/app/docs/prompt-from-phone/page.tsx:1
- web/app/docs/quota-and-cost/page.tsx:1
- web/app/docs/quota-limits-explained/page.tsx:1
- web/app/docs/remote-access/page.tsx:1
- web/app/docs/supported-agents/page.tsx:1
- web/app/privacy/page.tsx:1
- web/app/terms/page.tsx:1

Shared new FAQ/landing FAQ wording: web/lib/faq.ts:1 and web/lib/landing-faq.ts:1. Shared documentation navigation: web/components/DocPage.tsx:1. Public EULA is byte-identical to docs/legal/EULA.txt. Canonical/social strings use web/lib/seo.ts.
