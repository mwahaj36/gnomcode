# Worker A handoff

## 2026-11-13: ownership blocker before G5

- Branch: `docs/handoff-worker-A`.
- Handoff PR: https://github.com/mwahaj36/spawnt/pull/185.
- Starting commit: `fee3c8ee8529213c60d541e7e160594979d6ad44` (`origin/main`).
- Prerequisite: `gh pr list --state open --json number` returned `[]` before fetching and creating the isolated worktree. None of PRs #178 through #183 remained open. Trust: exact GitHub response.
- Scope: documentation only. No implementation, endpoint, JSON shape, setting or ADR was added. No feature is marked complete in SPEC or the roadmap.

### G5 phone interrupt: blocked

- Feature PR: not started.
- Gap evidence at the starting commit: `crates/daemon/src/remote.rs:626-627` dispatches Remote request types through a closed match. The authenticated E2EE receive loop calls this dispatcher at `remote.rs:601`. Unknown types reach `remote.rs:866`, returning `Unsupported remote command`. There is no interrupt branch.
- Ownership conflict: Worker A owns new daemon source files, but not the existing `crates/daemon/src/remote.rs`. Shared `api.rs` permissions allow only route and module lines. Adding a local route and a new handler alone cannot extend the E2EE dispatcher. Existing daemon root module declarations are in `crates/daemon/src/lib.rs:11-12`, also outside Worker A's ownership.
- Proposed next implementation, subject to ownership resolution: keep the handler in a new daemon file; add only minimal dispatch/module wiring in an explicitly assigned existing file; implement state-gated PTY Esc in the owned PTY crate. This is a plan, not a verified API or promise about native Esc behavior.
- Tests: none added; no production code written, so no red/green cycle started.
- Gate: `CARGO_INCREMENTAL=0 cargo fmt --all --check` passed. `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` not run; implementation stopped before code changes. `cargo deny check` not run; Cargo.lock unchanged. Web and desktop gates not run; neither directory touched. Documentation whitespace/encoding checks recorded below before publication.
- Verified live: GitHub prerequisite and repository source inspection only.
- Only in tests: nothing.
- Not verified: Esc cancellation for any provider, interrupt acknowledgment, state gating, pending-approval cancellation and E2EE delivery.
- Deviations: no implementation PR, no SPEC/roadmap completion amendment and no full Rust build gate for this blocker-only documentation PR. Required stop on an unowned file prevented starting G5; no product behavior changed. Existing unrelated `.claude/` in the original checkout was left untouched.
- Risks/questions: may Worker A add minimal request-dispatch wiring in existing `crates/daemon/src/remote.rs` and a module declaration in `crates/daemon/src/lib.rs`? Alternatively, which Worker B PR will provide those hooks and what interface should the new file implement? No interface has been invented.
- Worker B dependency: no PR number exists yet. G5 and all later E2EE API features require an assigned dispatcher integration point.

### Remaining ordered items

All entries below have no feature PR, changed files, endpoint, JSON shape, setting, ADR, test, gate or live verification yet. They are not started because the assignment requires G5 first and requires stopping for an unowned file.

| Item | Status | Known prerequisite or reviewer question |
| :--- | :--- | :--- |
| G6 agy native resume | Not started | Verify interactive `agy --conversation <id>` in an isolated workspace. |
| G7 agy limits `/usage` | Not started | Verify no model call; existing `vendor_limits.rs` is also unowned; ADR 0029 amendment requested by assignment. |
| G11 environment scrub | Not started | Gather exact variable-name evidence before choosing removals. |
| G12 Unix process-group stop | Not started | Preserve Windows behavior; cross-platform review required. |
| G13 Codex/Gemini discovery | Not started | Verify real install locations; use temporary fake executables. |
| C file listing and reading | Not started | ADR and confined workspace access; E2EE dispatcher wiring required. |
| C session changes/diff | Not started | ADR and E2EE dispatcher wiring required. |
| C last-turn artifacts | Not started | ADR and evidence of files created/edited required. |
| C attachments | Not started | ADR, size/type limits and E2EE dispatcher wiring required. |
| C quick actions | Not started | ADR, canned prompts only and E2EE dispatcher wiring required. |
| G8 Claude limits/API errors | Not started | Anonymized real fixtures before parser changes. |
| G3 terminal screen model | Not started | Recorded terminal evidence before matching changes. |
| G14 Enter recovery | Not started | G3 and proof text remains in composer; otherwise do not implement. |
| Codex app-server approvals | Not started | Verify live protocol; reuse ADR 0032; dispatcher/module ownership unresolved. |
| Antigravity ACP approvals | Not started | Verify live protocol; reuse ADR 0032; dispatcher/module ownership unresolved. |
| Gemini ACP approvals | Not started | Verify live protocol; reuse ADR 0032; dispatcher/module ownership unresolved. |
| D3 ingest watcher/back-off | Not started | Measure idle CPU before and after. |
| Antigravity exhausted cross-check | Not started | Verify against RESOURCE_EXHAUSTED evidence. |
| Failover demo | Not started | Automated test plus recorded manual Claude-to-Codex handoff. |
| E3 off-by-default settings | Not started | Preserve data and code; check ownership of all integration callers. |
| E2 api.rs domain splits | Not started | All Worker B PRs must be merged; current api.rs edit allowance prohibits pure moves. |
| E2 store/lib.rs domain splits | Not started | All Worker B PRs must be merged first. |

## Status at stop

- Last implementation commit: none. Last inspected main commit: `fee3c8ee8529213c60d541e7e160594979d6ad44`.
- Active reporting branch: `docs/handoff-worker-A`; no feature branch or partially written implementation exists.
- Half done: no implementation. Reporting PR #185 is open; this final reporting update completes publication.
- Resume: first resolve dispatcher/module file ownership. Then fetch, confirm prerequisites, create `feat/remote-interrupt` from fresh `origin/main` in a new worktree, inspect relevant SPEC/code again, verify native Esc behavior, write the failing test and implement G5 within the assigned files. Do not assume this report defines any protocol shape.
- Recommended merge order: handoff documentation PR only; no implementation PRs exist. Do not merge on Worker A's behalf automatically.

### Publication addendum

- Log PR and merge order: https://github.com/mwahaj36/spawnt/pull/185 only.
- Reporting snapshot commit before this final status update: `5f0f64a` on `docs/handoff-worker-A`. The final status update is the following commit in that PR's history.
- Documentation checks: `git diff --cached --check` passed after removing extra EOF blank lines; new files have LF and UTF-8 without BOM; staged diff contains zero `U+00C2` mojibake matches (exact check result).
- The reporting worktree is removed after pushing this status update; resume from the remote branch if further reporting edits are needed. No target or web/.next was generated in it.
