# FleetHUD (fleethud.pro) — Product Specification & Architecture Document

## 1. Executive Summary

**FleetHUD** (`npx fleethud`) is an agent-agnostic, zero-friction local flight recorder and real-time Heads-Up Display (HUD) for AI coding agents. 

Modern developers do not use a single AI tool—they operate a fleet. At any given moment, a developer might run:
- **Claude Code** in terminal tabs running multi-turn loops.
- **Google Antigravity 2.0 / IDE / CLI (`agy`)** executing deep codebase changes and subagent missions.
- **Cursor / Composer** generating in-editor code refactors.
- **Cline / Roo Code** in VS Code doing autonomous tasks.
- **OpenAI Codex / ChatGPT Desktop** assisting in chat.

Existing tools are single-agent silos (`ccusage` for Claude only, `Cursor Cost Tracker` for Cursor only) or heavy enterprise APMs requiring SDK instrumentation and proxy routing. 

FleetHUD unifies the entire developer machine under a single, non-invasive system tray / menubar HUD and local ledger—tracking real-time agent activity, exact token metrics, prompt cache discounts, context bloat, and subscription quota health with **zero configuration required**.

---

## 2. Core Value Propositions

### For Subscription Users (Claude Pro/Max, Cursor Pro, ChatGPT Plus)
- **Rate-Limit & Lockout Prevention:** Tracks quota burn velocity and predicts lockout times before developers hit the dreaded 5-hour limit.
- **Context Window Health:** Measures exact context growth (`input_tokens`) and flags cognitive degradation/bloat before the agent gets dumb or slow.
- **Subscription ROI ("Plan Value"):** Computes equivalent compute generated (e.g., *"You extracted \$184 of compute this week on your \$20 plan"*).

### For Pay-As-You-Go API & Enterprise Users
- **Ground-Truth Cost:** Calculates exact costs down to the micro-cent by factoring in 90% prompt caching read/write discounts.
- **Runaway Loop Auto-Kill:** Halts runaway recursive loops before an agent burns hundreds of dollars unattended.
- **Team Ledger:** Aggregates spend across team members without storing proprietary code or secrets.

### For Workflow Efficiency (The Mission Control HUD)
- **Live Activity Feed:** Glanceable tray status showing who is running, what bash command or file is being touched right now, and who is waiting for permission.
- **"Bring to Front" Window Switcher:** One-click focus to jump directly to the specific terminal or editor window running a given agent.

---

## 3. Supported Agent Surfaces

| Agent / Surface | Ingestion Method | File / Data Path | Extracted Telemetry |
| :--- | :--- | :--- | :--- |
| **Claude Code (CLI & IDE)** | Log Harvester & Native OTel | `~/.claude/projects/**/*.jsonl` or OTLP (`localhost:4318`) | Exact input/output/cache tokens, model, tool durations, approval requests |
| **Antigravity 2.0 App & IDE** | Log Harvester | `<appDataDir>/brain/<id>/.system_generated/logs/transcript.jsonl` | Steps, `PLANNER_RESPONSE`, subagent invocations, tool arguments |
| **Antigravity CLI (`agy`)** | Log Harvester | `~/.gemini/antigravity-cli/` + session logs | CLI turns, slash commands, token burn |
| **Cursor / Composer** | Storage Reader & API | `Cursor/User/workspaceStorage/**/state.vscdb` + `/api/usage` | Fast request count remaining, cycle end date, model tags |
| **Cline / Roo Code** | Log Harvester | `Code/User/globalStorage/saoudrizwan.claude-dev/tasks/*.json` | Task status, tool executions, API costs |
| **Aider & Terminal CLI Agents**| Log Harvester | `.aider.chat.history.md` & tags cache | Commits, files touched, tokens |

---

## 4. The 3-Tap Zero-Friction Architecture

1. **Tap 1: The Local Log Harvester (Default / Zero Config)**
   - Monitors known agent data directories using native OS filesystem watchers (`fsnotify` / `inotify`).
   - Reads appended JSONL logs in real-time. Opens files with non-exclusive read permissions to avoid locking collisions on Windows.
   - Requires zero proxy configuration, zero environment variable changes, and zero agent modifications.

2. **Tap 2: Embedded OpenTelemetry (OTel) Sink (Native High-Fidelity)**
   - Runs an embedded OTLP receiver on `localhost:4318` (HTTP) and `localhost:4317` (gRPC).
   - Ingests standard OpenTelemetry GenAI traces natively emitted by Claude Code and modern AI runtimes.

3. **Tap 3: Companion Model Context Protocol (MCP) Auditor (Optional)**
   - A universal MCP server that can be plugged into any MCP-compatible agent (`mcpServers` config) to audit tool calls directly.

---

## 5. Normalized Data Schema

```typescript
export interface FleetAgentEvent {
  id: string;
  agent: "claude-code" | "antigravity" | "cursor" | "cline" | "codex" | "other";
  surface: "cli" | "ide" | "desktop_app";
  sessionId: string;
  timestamp: string;

  project: {
    name: string;
    repoRoot: string;
    gitBranch?: string;
  };

  model: string;
  tokens: {
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
    total: number;
  };

  cost: {
    actualUsd: number;
    cacheSavingsUsd: number;
  };

  status: "thinking" | "executing_tool" | "waiting_for_approval" | "idle" | "error";
  currentAction?: {
    toolName: string;
    details: string; // e.g. "run_command: pytest tests/"
  };
}
```

---

## 6. Product Tiers & Monetization Strategy

### Tier 1: Community Core (100% Free & Open Source)
- Distributed via npm: `npx fleethud` or `npm install -g fleethud`.
- Embedded local SQLite database (`~/.fleethud/data.db`).
- Localhost web dashboard running on `http://localhost:3333`.
- Unlimited local tracking across all supported agents.
- Privacy-first: 100% of data stays on the user's laptop.

### Tier 2: FleetHUD Pro (\$19 One-Time Lifetime License)
- Native System Tray / Menubar HUD (macOS & Windows via lightweight Tauri shell).
- Glanceable tray pill with active agent count, live burn, and status animations.
- **Runaway Loop Auto-Kill:** User-defined threshold (e.g. *"Stop agent if spend > \$5 in 5 mins"*).
- **Quota & Reset Notifications:** Desktop sound/toast when Claude 5-hour limit resets or approval is needed.
- **Window Focus Switcher:** Single-click to jump to the terminal/IDE running any agent.
- Billed via **LemonSqueezy** / Stripe Payment Link (no merchant account friction).

### Tier 3: FleetHUD Teams (\$12 / seat / month)
- Centralized team dashboard for engineering managers.
- Cloud sync via lightweight Supabase backend.
- Team-level spend caps, budget alerts to Slack/Discord.
- Audit & compliance export (CSV/JSON) for AI-generated code reviews.

---

## 7. Zero-Dollar Bootstrapped Tech Stack

- **CLI & Core Daemon:** Node.js / TypeScript (or Go binary for single-file performance).
- **Tray Application:** Tauri v2 (Rust + native webview, < 25MB RAM).
- **Web UI:** React + Tailwind CSS + Tremor / Recharts.
- **Local Storage:** Embedded SQLite.
- **Distribution:** npm (`npm i -g fleethud`) + GitHub Releases.
- **Hosting / Domain:** Cloudflare Pages on `fleethud.pro` (\$0 hosting, free SSL).
- **Checkout:** LemonSqueezy / Stripe Payment Links.

---

## 8. Technical Milestones & Roadmap

- **Milestone 1: The Core Scraper & Adapter Engine**
  - Implement file watchers for `~/.claude/projects/` and `<appDataDir>/brain/`.
  - Normalize turn events into `FleetAgentEvent`.
  - Store turns in local SQLite with exact cache token pricing.
- **Milestone 2: The Local Web Dashboard (`localhost:3333`)**
  - Live WebSocket stream of active turns and cost counters.
  - Context size chart and rate limit countdown tracker.
- **Milestone 3: Native Tray App (Tauri v2)**
  - Menubar widget with live status pill and popover HUD.
  - Process tree inspector to bring terminal/IDE windows to the foreground.
- **Milestone 4: Public Launch on npm & ProductHunt**
  - Launch `npx fleethud` publicly on npm.
  - Landing page live on `fleethud.pro`.
