# 2026-11-14 Subagent sidebar fixes

## Goals
Include-subagents filter that works everywhere, real subagent names, parent state that reflects running children.

## Outcomes
- `include_subagents=false` now drops the nested `subagents` list as well as top-level child rows; the parent keeps a small "N running · M subagents" count. The choice persists in `localStorage` (`gnomcode.includeSubagents`) and applies to the sidebar and Home.
- Names come from Rust: `core::subagent::subagent_name`. Claude Code writes `agent-<id>.meta.json` beside each subagent transcript with the Task `description` and `agentType`; ingest reads it after each commit and sets the native title. Codex nickname/role already exist; Antigravity records no name. Everything else is `Subagent N` in start order.
- Parent state: `core::subagent::working_label` gives "Working · N subagents running". The conversations endpoint lifts the parent state and sets `state_label`; the store list sets `state_label` the same way. The sidebar and All chats show it.
- Remote `sessions` request now nests subagents too, so the phone gets the same counts.

## Problems and root causes
- Unchecked still showed nested rows because the server nested children before filtering and never removed them; the frontend always rendered whatever arrived.
- The parent said Idle because only its own (quiet) log set `state`; the Home lift changed the group but not the label.
- Every row said "Subagent" because the Claude adapter only reads a name from a transcript line that never carries one.

## Decisions
- Default is on. Owner decision: subagents collapsed by default, expandable inline, capped at 5. Off would leave nothing to expand. The group is collapsed either way, and the count-only form is one click away in Filters.
- With the filter on, a subagent is a top-level row only when its parent is not in the result, so the sidebar is not flooded with duplicates of nested rows.
- The Filters panel already gets the console form styling (global `.sp-native-console` input/select/label rules); the checkbox now shares the `sp-all-chats-toggle` class with All chats. There is no separate form-component library to adopt.

## Verification
Rust gate; web typecheck, lint, unit tests, build and full Playwright (new `subagents.spec.ts`); desktop gate.
