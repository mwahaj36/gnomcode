# 2026-11-13 - Scope 1.0 to three agents (E4, ADR 0058)

## Goals
- Delete Teams, the extra adapters and Gemini CLI from code, settings, docs, fixtures and tests. 1.0 supports exactly Claude Code, Codex and Antigravity. Keep account metadata sync and the Gemini MODELS Antigravity uses.

## Outcomes
- Core: removed `AgentId::GeminiCli`, the `teams_enabled` and `extra_adapters_enabled` toggles, the Gemini CLI display names and `Gemini CLI` pricing label. The unknown-agent display fallback is now "Unsupported agent".
- Adapters/ingest/platform/pty: deleted `gemini.rs`, its snapshot and `fixtures/gemini-cli/`; removed the `~/.gemini/tmp` root, Gemini install discovery and test, Gemini launch, resume (`--resume`), `/model set`, startup-screen matching and tests. `scan_log_files_with_extra` collapsed into `scan_log_files`.
- Daemon/CLI: removed the Gemini model catalog and auth probe, `gemini` from doctor probes, launch/resume/remote allow-lists, installed-agents list, `agent_for_model` inference, failover provider mapping and `is_teams_enabled`. Handoff-eval fixtures that named `gemini-cli` now use `antigravity` (`gemini-to-claude` renamed `antigravity-to-claude`).
- Web/desktop/scripts: removed Gemini CLI from launcher and filter selects, rule-agent select, provider logo map, tray names and `tray.ps1` focus map; rebuilt `console-react.js`. Unknown agent ids render "Unsupported agent".
- Docs: new ADR 0058; ADR 0049 marked partly superseded; ADRs 0002, 0003, 0013, 0014 and 0022 carry a supersession note; SPEC, PRODUCT, AGENTS, README, CHANGELOG, launch checklist, legal drafts and the M6-M9 addendum updated. Roadmap E4 removal item ticked.

## Decisions
- Old data is never deleted. `AgentId::parse` returns `None` for removed agents so typed queries skip them; the new store test `rows_from_removed_agents_stay_readable_and_never_crash_queries` stores a row, relabels it `gemini-cli`, and checks paginated lists, the usage report and row retention.
- Aggregate token SQL still lists `gemini-cli` next to `codex` so historical cache-token totals do not change.
- Licenses already signed with tier `team` must still verify (the signature covers the serialized payload), so `Tier::Team` stays as a legacy variant treated as Pro. No Teams feature exists.
- `supabase_cloud_sync_v1.sql` keeps its Teams tables as history; a comment marks them unused. Nothing is dropped.
- Left on purpose: `GEMINI_HOME` (Antigravity data root), Gemini model pricing and pool names, and `cursor`/`vscode` entrypoint detection in the Claude and Codex adapters (they label which editor hosts Claude Code or Codex, not a Cursor adapter).
- Marketing pages are out of scope here and handled by the marketing audit PR.

## Problems and root causes
- `native_compaction_clears_pending_relays_without_erasing_history` and the handoff-eval provider test counted a fourth agent; they now count three.
- Three `scripts/check-*.mjs` (`cloud-removal`, `native-navigation`, `settings-saves`) fail identically on untouched `main` (missing argument, removed function, missing module). Not caused by this change. The Playwright checks were not run (browsers are not installed here).

## Verification
- `CARGO_INCREMENTAL=0`: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all green. `Cargo.lock` unchanged, so `cargo deny` was not rerun.
- Web: `npm ci`, `npm test` (133 pass), `npm run typecheck`, `npm run build`; `scripts/build-local-console.mjs` regenerated the bundle. Desktop: `npm ci`, `npm test` (40 pass).
- Passing scripts: check-console, check-local-console, check-onboarding, check-release-versions, check-tour-density.
