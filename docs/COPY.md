# spawnt — Official Copy Deck

Centralized copy guidelines and official product text for spawnt. Every UI component, documentation page, CLI output, and marketing asset must adhere to these standards.

---

## 1. Brand Identity & Casing

| Field | Canonical Value | Prohibited Variants |
| :--- | :--- | :--- |
| **Product Name** | `spawnt` (always lowercase) | `Spawnt`, `SPAWNT`, `FleetHUD` |
| **Domain** | `spawnt.xyz` | |
| **Tagline** | See what your coding agents are doing. | Live mission control, cockpit, AI flight recorder |
| **Subline** | Live status, cost and limits for your agents. Local-first. Nothing leaves your machine. | |
| **Environment Variable** | `SPAWNT_HOME` (with fallback to `FLEETHUD_HOME`) | |

---

## 2. Invariant Copy Rules

1. **No emojis anywhere.** Never use emojis in CLI terminal output, web UI, native tray menus, notifications, documentation, or commit messages. Use clean typography, geometric SVG indicators, or plain-text badges (`[WAIT]`, `[RUN]`, `[IDLE]`, `[EXACT]`, `[EST]`).
2. **Eliminate flight / space metaphors.** Replace all instances of "cockpit", "mission control", "flight deck", and "ground control" with clear engineering terminology: "console", "daemon", "live status", "session monitor".
3. **Declare trust levels explicitly.** Every token count or dollar figure must declare its trust rating:
   - `[EXACT]` — Derived from authoritative provider turn logs or response headers.
   - `[EST]` — Calculated from heuristic turn lengths or local character counters.
   - `[API]` — Verified against vendor quota endpoints.
   - `[UNKNOWN]` — Unattributed or ambiguous data.

---

## 3. Product Invariants (Verbatim)

Use these four guarantees word-for-word in footers, auth cards, docs, and privacy sections:

1. **Zero Content Storage:** Prompts, source code, diffs, and tool execution outputs NEVER touch disk or network. Only aggregate counters and scrubbed metadata are persisted.
2. **Local-First & Loopback Only:** Servers bind strictly to `127.0.0.1:41738` with bearer token authentication. All features work 100% offline.
3. **Fail-Open Architecture:** Background hooks and tailers run asynchronously with a 50 ms timeout. If spawnt is stopped or slow, your coding agent never blocks.
4. **Local HMAC-SHA256 Masking:** Directory paths and project identifiers are hashed with a local machine key before any cloud synchronization.

---

## 4. Status Badges

Format session and agent execution states using monospace square brackets:

- `[RUN]` — Agent is actively executing a tool or generating tokens.
- `[WAIT]` — Agent is blocked waiting for user prompt input or confirmation.
- `[IDLE]` — Agent process is alive but inactive (>15m without events).
- `[LIMIT]` — Agent has hit a rolling 5-hour or 7-day quota ceiling.
- `[ERR]` — Agent execution failed or encountered an unhandled error.

---

## 5. Pricing Tiers (SPEC 1.5)

- **Free / Community ($0 forever):**
  - "Everything you need to monitor and attribute your local AI coding agents on one workstation."
  - Unlimited local sessions, all local adapters, per-model 5h & 7d quota pools, background tray service.
- **Pro ($15/mo or $12/mo billed annually):**
  - "For engineers running agent fleets across multiple laptops and workstations."
  - Real-time cross-device cloud sync, Multi-Plan ROI calculator, custom pricing overrides, budget alerts.
- **Team ($35/seat/mo or $28/seat/mo billed annually):**
  - "Full visibility and spend governance across your entire engineering organization."
  - Org-wide dashboard, spend caps, HMAC project masking, role-based access, webhook notifications.
