# 2026-11-14 - Settings information architecture and Sign out

## Goals

The owner could not find Log out. Regroup Settings, put Account and Sign out first, add search and a command palette, keep every setting.

## Inventory (before -> after)

| Before (order) | Contents | After section |
| --- | --- | --- |
| Providers | limit sources, enable commands; live setup checklist (PR #288) | Providers |
| Appearance | theme cycle | Appearance |
| Devices and Remote | account access toggle, paired devices, remove device, device change confirmation | Remote and devices |
| Automations | reconnect after a limit, resume queue, usage rules, recent alerts, rotation history | Automations |
| Notifications | notification settings, pricing overrides, offline license | Notifications |
| Data | reload view, rescan local data | Privacy and data |
| Account (near bottom) | sync status, Manage account | Account (first): signed-in email, Manage account, Sign out |
| Desktop updates | update status, restart to update | Updates |
| Guide | Getting started, downloads, docs, pricing links, version | About |

No setting was removed. Sign out existed only as "Disconnect desktop" inside the Account view.

## Outcomes

- Settings: left section list on desktop (sticky, >= 1024 px), scrollable chip list on phone, search by name and keywords (`settings-index.ts`), "No settings match." empty state.
- Account first with the signed-in email and a danger-styled, confirmed Sign out.
- Shell: Sign out in the rail footer when connected (one click), Search entry and Ctrl/Cmd+K command palette (views, every Settings section, Sign out, Set up providers).
- One `signOutDesktop()` used by Settings, rail, palette and the Account view (renamed button to Sign out).

## Not done

- Save-on-change toasts for every existing control and a phone drill-in list: existing saves keep their current status lines; the phone uses a jump list.
- Before screenshots were not captured; after screenshots in `assets/settings-ia/`.

## Verification

web typecheck, lint, unit tests 140/140, build, full Playwright (new `e2e/settings-ia.spec.ts`: Sign out one click from the rail in light and dark, Account first, search finds Sign out, section jump, palette reaches Sign out and Notifications, phone two taps and no horizontal scroll).
