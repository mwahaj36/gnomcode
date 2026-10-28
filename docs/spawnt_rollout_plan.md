# spawnt redesign plan

Status: proposed implementation sequence, updated 2026-10-28 for the current repository and the reviewed landing preview. The user-supplied [original redesign plan](spawnt_redesign_plan.md) is preserved verbatim alongside this update. Use this rollout plan for implementation order, [DESIGN.md](../DESIGN.md) for current visual tokens, and [SPEC.md](SPEC.md) for product scope, security, and behavior. A phase that changes architecture or deliverables must update SPEC and, when architectural, add an ADR in the same PR.

## Current rollout state

The native GitHub stack #66 contains the redesign phases. Marketing, Docs, Auth and Settings use shared tokens and controls. Both consoles share neutral summary/activity/quota/session/project presenters and the same rail/content layout, with independent authenticated data adapters. The landing tour uses those completed presenters with explicit illustrative data. Rust provides daily/project/session totals, chart proportions and quota provenance/freshness.

Windows retains its script tray and authenticated browser app window. Linux/macOS use the same local console through browser launchers (ADR 0007); a Tauri migration remains separate. Canonical generated PNG/ICO/ICNS assets and self-hosted fonts are present. v1.5 Windows/Linux artifacts are rebuilt and verified. macOS native jobs cannot start because of GitHub account billing; merge and publication remain held. Browser visual QA remains unavailable because the tool reports disabled. Live cloud quota publication requires the user's deferred desktop re-pairing. Do not call pending visual/platform checks passed.

## Rollout order

Use one focused branch and PR per phase. Update `docs/SESSION_LOG.md` for each. Keep the landing preview available for comparison until the shared design system replaces its scoped tokens.

| Phase | Work | Acceptance |
| :--- | :--- | :--- |
| 0. Inventory and architecture | Map every current route, daemon endpoint, auth state, action, and tray entry. Write an ADR for shared console UI and delivery; update SPEC where it differs from code. | No existing console capability or security boundary is lost in the proposed map. |
| 1. Shared design foundation | Move the approved palette from DESIGN into shared tokens; self-host type and mark; define buttons, badges, tables, inputs, tabs, charts, panels, empty/error states, and themes. | Landing, component showcase, and both themes use the same tokens; contrast and reduced-motion checks pass. (Started on `feat/spawnt-foundation`.) |
| 2. Data contracts | Identify missing page data and add only necessary Rust store/daemon queries and endpoints. Reconcile SPEC's endpoint list with the actual `/api/v1` routes. | API tests cover filters, pagination, malformed input, trust levels, and token/Host/Origin protection. |
| 3. Local console | Build the new shell and pages against the daemon API; package its assets in the daemon binary. Replace `console.html` only after feature parity. (In progress: matching destinations and truthful local refresh states.) | Offline local console works with no remote font, script, or analytics requests and no prompt/code/tool-output persistence. |
| 4. Signed-in web console | Reuse the same visual components and page layouts, with a cloud/Supabase data adapter and account states. (In progress: seven destinations and dedicated Usage/Limits.) | Signed-in pages match the local console visually and only show data actually available in the cloud. |
| 5. Marketing and account pages | Migrate Pricing, Downloads, Docs, Auth, Settings, shared nav/footer, metadata, and responsive layouts. Replace illustrative product shots with captures from the real console. | Every public route follows DESIGN; old FleetHUD copy is removed from user-facing surfaces only when its replacement works. |
| 6. Native and distribution | Make the native tray/open-console experience use the same local console UI; update installer and app branding after parity. | Windows, macOS, and Linux fallbacks in SPEC still work; upgrades keep user data. |
| 7. Internal rename | Rename binary, package, env vars, and data dir separately with aliases and migration tests. | Existing installs, integrations, and scripts keep working through the documented transition. |

The landing and shared web shell are partial Phase 5 work; PRs #47 and #48 are stacked and unmerged. Pricing and Downloads are implemented on separate stacked branches. Do not infer that Phase 1, the other inner marketing routes, native surface, or installers are finished because the home route looks new.

## Console redesign

### Architecture and data boundary

1. ADR 0005 records the current framework-neutral token source and the decision to retain standalone embedded HTML for local delivery while matching layouts. Keep auth, transport, and data adapters separate. A shared React component package remains a possible later architecture change and requires a separate ADR before implementation.
2. The daemon remains the source of local sessions, redacted activity, cost, quota, guardrails, and settings. Every calculation and aggregation stays in Rust. The UI formats values and shows trust levels; it does not infer missing usage or reprice events.
3. The local console must work offline on `127.0.0.1` with the existing bearer token and Host/Origin checks. Do not weaken these to make a new frontend easier to serve.
4. The web console requires sign-in for its useful cloud views. It uses account-scoped Supabase data and must not claim that it has local-only project names or content. If a local daemon is unavailable, show a clear connection state rather than zero-filled figures.
5. Keep route/selection/filter state in the URL where a user benefits from deep linking. Preserve keyboard access and browser Back behavior. The local build can use hash routes if required by daemon serving; the web build can use Next routes, with the same view components.

### Information architecture

The console gets its own app shell: compact sidebar or rail, top bar with connection/source and time range, then a hairline-divided content grid. It does not reuse the floating marketing navbar. At smaller widths, collapse the sidebar to a rail and then an accessible mobile navigation pattern. Use one chart and one primary table per view before adding more widgets.

| View | Primary question | Core content |
| :--- | :--- | :--- |
| Overview | What needs attention now? | Attention row, active sessions, four trusted KPI cells, a seven-day usage chart, recent sessions. |
| Sessions | What is each agent doing? | Filterable, sortable session table; detail drawer with state timeline and redacted metadata. |
| Usage | Where did usage and cost go? | Daily chart, agent/model/project breakdown, export of Rust-produced data. |
| Limits | What is near a cap or reset? | Pool usage, basis/trust, reset time, and unknown-state treatment. |
| Projects | Which repos account for activity? | Per-project rollups and recent sessions, only where the data source can provide project identity. |
| Automations | What controls are armed? | Resume queue, guardrails, and relay controls already supported by SPEC; explicit state and confirmation for consequential actions. |
| Settings | How is this installation configured? | Theme, agents, notifications, storage, privacy, account/license, and diagnostics as supported by the local or web context. |

Start with Overview and Sessions because they establish the shell, data states, tables, and detail pattern. Then Usage and Limits, followed by Projects, Automations, and Settings. Map each old-console action to its new location before removing the old page. Do not add speculative controls merely because they appear in the external draft.

### Interaction and motion

- Live data should update in place without resetting scroll, selected rows, filters, or focus. Pause or slow polling when hidden and show the last update time when stale.
- On first viewport entry, chart bars may rise sequentially as on the landing preview. On subsequent live updates, animate only changed values with a short transition. Do not replay the whole dashboard every polling cycle.
- Tabs, drawers, filters, tooltips, and sidebar expansion use the timing in DESIGN. Avoid layout jumps. A pinned sidebar may shift content; a temporary hover expansion should overlay it.
- Every chart has a textual summary and a table or equivalent readable data view. Color is never the only series or status cue.
- Every page has real loading, empty, error, disconnected, and stale states. Unknown usage is displayed as unknown, never as zero.
- Respect reduced motion in both local and cloud builds. Test keyboard and touch navigation, not just pointer hover.

### Implementation slices

1. Build tokens, type, mark, and a component showcase in light and dark.
2. Build the shell with real navigation, theme persistence, connection indicator, and responsive behavior.
3. Implement Overview and Sessions using existing API data. Add Rust endpoints only where needed for a defined view.
4. Implement Usage and Limits; establish chart accessibility and trust badges.
5. Complete Projects, Automations, and Settings where backed by SPEC and existing functionality.
6. Wire the same view components to the signed-in web data adapter. Label missing cloud capabilities instead of silently approximating them.
7. Run old/new parity review: route, action, trust level, offline state, security, and responsive screenshots. Remove old console markup after parity.

## Design system rollout

- Put the approved tokens in one source used by the Next site and compiled local console. Add a small automated check to catch token drift. Use `#070708` for dark canvas, `#F6F7F8` for light, `#FF2544` for primary. Keep `#0C1119` for light-theme text/on-primary text.
- Migrate shared navbar/footer and the remaining site routes after the console foundation is stable. The marketing header keeps the Daytona-style scroll movement and `Log in` action; the theme control stays minimal in the footer and defaults to System.
- Self-host fonts and icons. Keep release builds without sourcemaps or devtools per AGENTS.md. Avoid external assets that would create phone-home requests.
- Preserve the current domain and download URLs until a separate distribution decision is made.

## Verification for every phase

- Run the applicable web build/typecheck and meaningful UI interaction checks. Capture desktop and narrow-screen views in light and dark.
- For Rust changes, run the repo Rust quality gate. For filesystem, process, tray, or focus changes, run cross-platform review and relevant OS CI.
- Check keyboard focus, reduced motion, contrast, table/chart semantics, signed-out and signed-in states, and local daemon disconnected behavior.
- Confirm every displayed number has a source and trust label. Confirm no prompt, code, file content, or tool output is stored or sent.
- Update DESIGN and SPEC when implementation changes either document. Record goals, outcomes, problems, and decisions in SESSION_LOG.
