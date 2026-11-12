# 2026-11-13 - Console four places and toggle rail

## Goals

Phase 6 of `docs/design/console-overhaul-plan.md`: seven places become four, the rail becomes a toggle-only grid column, Workspace opens list-first, and the rail becomes a top bar under 720px (addendum 15.3). Also fix the Phase 5 review finding that Home got about 280px on a phone because of the side rail.

## Outcomes

- Rail: Home, Workspace, Usage, Settings. A 64px/208px grid column driven by `body[data-rail]`, toggled only by `#navigation-toggle`, persisted in `gnomcode-rail-expanded` (try/catch). All native hover/focus/`:has` width rules deleted. The web console rail keeps its hover rules (its own phase).
- Under 720px the rail is a top bar with four icons; Home, Usage and Settings use the full 390px.
- `VIEW_ALIASES` redirects `limits`, `projects`, `automations`; the hash is rewritten in place with `replaceState`.
- Usage now holds Limits (provider cards), Projects and the period selector with Export. Settings is one 720px column: Providers (new, from `/api/v1/providers/overview`), Appearance, Devices and Remote, Automations, Pricing, Data, Account, Guide. `#view-projects` and `#view-automations` removed.
- Workspace: list open by default (collapsed state persisted in `gnomcode-chat-list-collapsed`), launcher hidden until "New", empty state in the main area, last conversation within 24 hours reopened once per visit, list grouped by the Rust `home_group` (Needs you, Running, Recent) when anything needs attention.
- Tests: `web/test/nav.test.mjs`. ADR 0031. SPEC 3.16 and section 9 updated.

## Problems and root causes

- Seven layers of rail CSS (hover, pinned, `!important` overrides) fought each other. Root fix: delete the native-console hover rules and add one Phase 6 block at the end of the stylesheet rather than another override layer.
- The conversations endpoint (`remote.rs`, out of scope) has no `home_group`. The console joins it by session id from the sessions fetch it already makes; no new request.
- Phone Workspace: the fixed list toggle and the "Conversations" button overlapped once the rail moved to the top. The in-page button is hidden on phones.

## Decisions and deviations

- Both the empty state ("New session") and the sidebar "New" would be brand red. Only the empty-state button is primary; "New" is neutral to keep one red element per view.
- Conversation list headings are Needs you, Running, Recent (Rust group), not Today and Earlier, to avoid date logic in JS.
- The chat header compact limits line was not restyled; `renderConversationLimits` is unchanged.
- Remaining inline styles in the Pricing and Offline license details were not touched (not moved content).
- Phone top bar has no account entry; sign-in is reached from Settings, Account.

## Verification

- Local console from this worktree, temp `GNOMCODE_HOME`, port 41987. Home, Workspace, Usage, Settings checked at 1440 dark and 390 (dark and light); rail expansion pushes content (content left 64 to 208); no horizontal scroll at either width; `#limits` and `#automations` redirect to `#usage` and `#settings`.
- Gate: cargo fmt, clippy -D warnings, test --workspace; web `npm test`, `typecheck`, `build`.
