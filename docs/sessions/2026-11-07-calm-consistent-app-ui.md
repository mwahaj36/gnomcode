# Calm, consistent application UI — 2026-11-07

## Goal and direction

Wahaj requested a detailed pass across every application page, using Apple Design and Impeccable, to remove congestion, repetitive EXACT/EST and uncertainty copy, and alignment defects. He explicitly selected clean platform system typography. Keep the dark/light palette and red accent; use shared controls, breathing room and progressive disclosure.

## Root causes and changes

- Broad text-input selectors made the subagent checkbox 255 by 44 pixels. Shared checkbox/radio rules now keep 16px geometry inside a 44px clickable label. Field labels are stacked and forms reflow at narrow widths.
- Explicit layout styles overrode the HTML hidden attribute, showing connection/status content outside its intended view. Shared application styles now honor hidden content, including flex layouts.
- Reporting repeated trust badges and inference warnings beside nearly every value. Short statuses, visibly estimated money, source tooltips and section-level disclosures preserve provenance without repeated badges. Unknown/stale values remain truthful.
- Activity interval grids, usage explanations and secondary histories are disclosed on demand. Empty tables omit meaningless headers. Provider rows use concise estimates with fuller partial-pricing explanations available in details/tooltips.
- Local and account consoles, Profile, account settings, notification setup, conversation filters, usage, projects and provider limits share system sans typography, control sizing and alignment. The web Automations page now offers the actual Workspace action instead of obsolete rotation guidance.

## Verification

The checkbox defect was reproduced in a browser against the baseline CSS before correction. Four new presentation regressions passed after their initial failures. A standalone browser control check uses static markup and no real agent data, testing three widths and both themes. Existing escaping, pricing, unknown-data, draft/history and notification regressions remain in the web suite. Local and account-console capture matrices cover eight views at 1440, 900 and 390px in both themes; account browser captures use synthetic data. Local read-only visual acceptance was explicitly authorized by the user. Screenshots remain in ignored local artifacts.

Release, installed acceptance and final gate results are recorded below when complete. Physical phone and native macOS/Linux validation are not claimed.

The full Rust quality gate passed (format, clippy, workspace tests, cargo-deny and core purity). Web checks passed 30 tests and typecheck; Electron checks passed 25 tests. Both capture matrices completed without document overflow; runtime-error checks were clean after correcting the capture harness's pre-DOM theme initialization. The control regression also verifies that hidden flex content stays hidden. Original local script/escaping/runtime-target checks passed. Cross-platform review: system fonts use local platform fallbacks, CSS has narrow/mobile and reduced-motion behavior, and no new OS process, permission, storage or notification behavior was introduced.

The final mobile check reproduced undersized form text after the shared styling change. The control regression now also requires 16px phone input text, and the corrected shared phone rule passed. Packaging was restarted so the installer includes the correction rather than the earlier capture build.

## Follow-up: Session tracking and UI navigation fixes

1. **Phantom Codex sessions:** Turn response items (`rs_`, `msg_`, `ctc_`, `fc_`, `ctco_`, `call_`) were previously treated as session IDs when emitted by Codex rollout logs. The Codex adapter now validates session ID prefixes and rejects sub-turn items. Existing phantom records are purged on store startup.
2. **Missing Antigravity ongoing sessions:** Background task completions in Antigravity contain `sender=<conv_id>/task-<id>`. The adapter was extracting the conversation ID as the sender, creating a circular reference where `parent_session_id = session_id`. Ongoing queries filter out child agents, hiding the active session. The adapter now validates `sender != conv_id` before assigning a parent, and cleans up any existing circular self-references.
3. **Workspace view persistence:** Active view selection is now saved to `sessionStorage` (`gnomcode-active-view`), and the desktop shell preserves URL hash parameters during reconnection so the interface does not unexpectedly jump back to the overview dashboard.
4. **Co-located sidebar toggle:** Added a dedicated collapse/expand button directly on the left side of the workspace pane when the sidebar is collapsed, eliminating the asymmetric navigation jump to the top right.
5. **Account sign-in visibility:** Unpaired desktop status cards remain visible even during daemon reconnection and offline states so the user can easily reconnect or pair without hunting through error states.
6. **Release and installer generation:** Rebuilt release binary (`target/release/gnomcode.exe`), packaged Windows desktop bundle, compiled installer (`Gnomcode-Setup.exe`), and updated distribution checksum manifests (`SHA256SUMS`, `SHA256SUMS-v0.9.3.txt`). All tests and quality gates passed.
