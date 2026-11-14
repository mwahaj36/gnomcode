# 2026-11-14 - Marketing site matches the redesigned console

## Goals

- Rebuild the landing hero and the "A closer look at the console" tour so they match the console after #268-#283.
- Use the console's provider logos everywhere on the marketing site.
- Audit marketing copy against SPEC, ROADMAP_1.0 and the 1.0.0 release notes.

## Outcomes

- The tour renders the real console presenters (`renderProviderCards`, `renderHeatmap`, `renderTodayTiles`, `renderHomeSessions`, `renderPeriodTiles`) with illustrative data, so it uses the console markup, tokens and CSS. Overview is the Home layout (full-width Limits, Activity and Sessions in 2fr beside Today in 1fr, via a container query). Sessions shows Needs you / Running / Recent, an expanded subagent group and a WSL badge. Limits shows one expanded card with Session and Weekly bars, a Low card and a multi-pool Antigravity card with one pool Out. Every screen is labelled ILLUSTRATIVE; the repeated in-screen title was removed.
- The hero is now a console window: the sidebar (Home, Chats, Usage, Settings, search, grouped sessions with a WSL badge) beside the handoff conversation. The sidebar hides below 600 px.
- `ProviderLogo` renders the same paths as `web/lib/provider-logos.js`. The supported-agents row uses it instead of the `/agents/*.svg` masks, which fixes the clipped Claude mark.

## Problems and root causes

- Home session rows wrapped the chevron onto a second line in the console too: #273 added a token badge (sixth grid child) without a sixth grid column. Fixed in `gnomcode-console.css` (`auto` token column at both breakpoints) and `homeRow` now always emits the token cell so the columns line up. `crates/daemon/src/console-react.js` was rebuilt.

## Copy fixes

- Phone approvals: Claude Code through its hook; Codex and Antigravity only with opt-in protocol mode (`gnomcode protocol-approvals enable codex|antigravity`, ADR 0057). Landing feature, landing FAQ, FAQ, pricing, Remote access and Prompt from phone pages.
- WSL discovery and launch (ADR 0060, #280-#282): supported-agents page, landing FAQ, OS FAQ.
- Downloads: Windows x64 only; removed "macOS and Linux are design targets" and the platform-tabs wording from Getting started.
- Chat/file previews: the in-flight chat rendering work was not on main when this finished, so it is not mentioned.

## Verification

- `npm ci`, typecheck, lint, `npm test`, `npm run build` in `web/`.
- Checked at 1440, 1024 and 390 px in light and dark: no horizontal page scroll (only the heatmap scrolls). New CSS adds no motion; existing reduced-motion rules still apply.
- Screenshots: `assets/2026-11-14-marketing/`.
