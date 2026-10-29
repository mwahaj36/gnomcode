## 2026-10-28 — web console refresh and trust states

- Goal: make manual cloud-view reload status truthful and prevent missing metrics from appearing as exact zero.
- Outcome: reload now reports success only when both cloud reads succeed, prevents duplicate manual refreshes, waits safely for an in-flight poll, and keeps the last successful refresh time on failures. Signed-out users see the control disabled with a sign-in explanation. Missing session token/cost/context values render UNKNOWN; relative times and rounded context percentages show estimated trust. Added keyboard support to expandable provider spend rows and focusable, named table scroll regions.
- Verification: Next.js production build and `git diff --check` passed. Chromium review confirmed the disconnected/signed-out state, disabled reload, unknown KPI state, table region names, and accessible session filters. A signed-in cloud fixture, narrow viewport, and keyboard-only interaction check remain pending.

