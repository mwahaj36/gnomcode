# ADR 0055: Handoff quality evals (backend)

- **Status:** Accepted
- **Date:** 2026-11-13
- **Owners:** Worker A (core scorer, daemon runner, CLI). Fills in the contract of ADR 0038 and Worker B's PR #211.

## Context
Roadmap F3 asks for repeatable evals of what survives a handoff to another agent. ADR 0038 fixes the read-only `GET /api/v1/evals/handoff` shape the Usage view renders. AGENTS.md rule 8 keeps scoring in Rust, and rule 6 requires every number to declare its trust level.

## Decision
1. **Eval set.** Synthetic cases live in `fixtures/evals/handoff/*.json` (no real user data): a conversation (user, assistant and tool events, with `repeat` to model long sessions), the touched files, a length `budget_chars` (default 18000: the 16000-character history cap of `continuation_prompt` plus its framing) and what must survive (`goals`, `files`, `open_tasks`) or must not appear (`forbidden` secrets). The set is embedded in the binary; a test fails if a fixture file is not listed.
2. **What is scored.** The runner builds the text a new agent actually receives: the newest `HANDOFF_CONTEXT_EVENTS` (50) events, cleaned by the same read-time cleaners as production, through `continuation_prompt`. Pasted secrets are passed through `redact_chat_content`, as the store does on read. It also scores the zero-token packet summary (`handoff_summary` plus the file list) for the same conversation and reports it as `packet_score_pct`, which does not decide the outcome.
3. **Scorer** (`gnomcode_core::handoff_eval`, pure). Case-insensitive phrase match with collapsed whitespace and `\` read as `/`. Weights: goals 40, files 25, open tasks 25, within-budget 10. A case passes at 80 or more, inside its budget, with no forbidden string present. Empty categories count as covered.
4. **Trust.** Every figure is `estimated`: keyword coverage over synthetic cases is a proxy for quality, never exact. The UI renders it as EST.
5. **Report.** `GET /api/v1/evals/handoff` (bearer token, loopback) returns the ADR 0038 shape: `updated_at`, `summary {total_cases, passed_cases, pass_rate_pct, mean_score_pct, trust_level}`, `cases[] {id, title, source_agent, target_agent, repo_name, outcome ("passed"|"failed"), score_pct, trust_level, duration_ms, summary, details, packet_score_pct}`, plus `by_provider[] {target_agent, cases, passed_cases, pass_rate_pct, mean_score_pct}` (per receiving provider). It serves `GNOMCODE_HOME/evals/handoff-latest.json`; when none exists it runs the embedded set once and stores that. There is no 404 for a missing report.
6. **CLI.** `gnomcode evals handoff [--json] [--dir <folder>] [--no-save]` runs the set, prints a per-case, per-provider and total report and stores the embedded-set report. It always exits 0: it is a report.
7. **CI.** The manual CI workflow gains a `handoff-evals` job with `continue-on-error: true` that prints the report to the job summary and uploads the JSON. It is not a gate.

## Consequences
- The baseline set has one deliberate known failure, `long-session`: a goal stated far behind the newest 50 events is dropped from the handoff. It documents a real limit of the current handoff rather than hiding it.
- Scores move only when `continuation_prompt`, redaction, the cleaners or the fixtures change, so a drop in CI points at a handoff regression.
- No chat content is involved: fixtures are synthetic, the report holds no conversation text, and nothing leaves the machine.
- `HANDOFF_CONTEXT_EVENTS` is shared by delivery and the eval so they cannot drift.
