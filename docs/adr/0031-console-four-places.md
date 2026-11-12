# 0031 — Console: four places and a toggle-only rail

- **Status:** Accepted
- **Date:** 2026-11-13

## Context

The local console had seven places (Dashboard, Workspace, Usage, Limits, Projects, Automations, Profile). Limits and Projects were thin views of usage data, Automations was a settings table, and the rail expanded on hover and focus, covering content and shifting focus targets. On a phone the side rail left Home about 280px of width.

## Decision

The console has four places: Home, Workspace, Usage and Settings. Limits and Projects live in Usage; Automations, providers, pricing, data, account and the guide live in Settings. Old `#limits`, `#projects` and `#automations` links redirect through `VIEW_ALIASES`; internal view keys are unchanged.

The desktop rail is a grid column (64px, 208px expanded) driven by `body[data-rail]` and toggled only by its button, persisted in `localStorage` (`gnomcode-rail-expanded`). Hover and focus never change its width. Under 720px the rail becomes a top bar with the four icons. The web console rail is out of scope and keeps its current behavior until its own phase.

Workspace opens list-first: the conversation list is open by default, the launcher appears only from "New", and an empty state or the last conversation opened in the previous 24 hours fills the main area.

## Consequences

Fewer destinations and no overlay. Stored URLs and session storage keep working. Settings is a long page, mitigated by one column and collapsed pricing. Account sign-in is reached from Settings on phones, where the rail has no account entry.

## Alternatives considered

- Keep hover expansion with a delay — still covers content and is unusable on touch.
- Bottom tab bar on phones — conflicts with the chat composer and keyboard.
