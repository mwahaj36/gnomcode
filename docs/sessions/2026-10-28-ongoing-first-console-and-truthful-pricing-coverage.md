## 2026-10-28 — ongoing-first console and truthful pricing coverage

- Local API diagnostics confirmed the connected daemon had priced totals but aggregate unknown trust due to an unpriced model. Added Rust-priced/unpriced invocation counts at timeline, interval, provider, model, session and project levels; exclude unpriced records from displayed subtotals and label incomplete coverage explicitly.
- Overview leads with ongoing-session title/project, agent/model, inferred state, latest activity and context, independent of history filters/pagination. Limits occupy a separate desktop column and a collapsible region; history remains in Sessions. Both consoles share presenters and CSS.
- Today now uses actual local-midnight boundaries, Julian-day comparisons across RFC3339 offsets and hourly UTC buckets displayed as local time. Added a source-owned calendar-month report/snapshot and period labels. Missing hours remain unobserved, not fabricated zeros.
- Project cards navigate to a native full-query project filter before pagination. Web filtering uses only already-published session names; private project rollups remain local.
- Reduced repeated trust tags, added plain-language partial warnings, restructured Account controls, hid activation when licensed and kept explicit Replace/Remove controls. Aligned license/export actions and prevented Add-button wrapping.
- Regression coverage verifies mixed known/unknown costs, project coverage/filtering, mismatched timestamp offsets around local midnight, hourly buckets, month labels, shared presentation and ongoing identity. Windows Rust gate including cargo-deny and production web build passed; final source/package checks are run before push. Browser visual checks remain unavailable and macOS builds remain deferred.

