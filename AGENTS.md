# AGENTS.md — FleetHUD

Rules for any AI agent (or human) working in this repo. Read this first, every session.

## 1. What this project is

FleetHUD is live mission control for AI coding agents (Claude Code, Codex, Antigravity, Gemini CLI, Cline/Roo, Cursor) on Windows, macOS and Linux. It reads the agents' local logs, normalizes them, stores metadata in SQLite, and shows live state, cost and quota through a CLI, a local web dashboard and a Tauri tray app.

**Single source of truth: [`docs/SPEC.md`](docs/SPEC.md).** Before working on a feature, read the relevant SPEC section. If your change alters scope, architecture, the roadmap or deliverables, update SPEC.md **in the same change**. Never let code and SPEC drift apart.

Architectural decisions go in `docs/adr/NNNN-title.md` (template: [`docs/adr/0000-template.md`](docs/adr/0000-template.md)) and get a row in SPEC §9.

## 2. Non-negotiable product invariants

1. **Never store content.** No prompts, code, file contents or tool output go to disk or the network. Activity summaries go through `core::redact` *before* persistence.
2. **Read-only by default.** Only `fleethud integrate <agent>` may modify agent config, and it must back up first and support `--remove`.
3. **Fail open.** Hooks and integrations must never block an agent when FleetHUD is down, slow or erroring (hook IPC timeout: 50 ms).
4. **No telemetry.** No analytics, no phone-home. The only network traffic allowed is the opt-out update-manifest GET, the opt-in pricing refresh, opt-in vendor APIs, and license checks.
5. **Loopback only.** Servers bind to `127.0.0.1` and require the bearer token plus Host/Origin validation.
6. **Every number declares its trust level** (`exact | estimated | api | unknown`). Never present an estimate as exact.
7. **Never escalate agent permissions.** Auto-resume and guardrails must never pass `--dangerously-skip-permissions`, `--yolo` or similar flags.
8. **Protect the closed-source binary (SPEC §6.8).** Keep 100% of business logic (parsing, pricing, quota math, rules, licensing) in Rust, never in TypeScript. Wrap sensitive string literals (SQL, adapter path patterns, license checks) with `obfstr!`. Never enable sourcemaps or Tauri `devtools` in release builds, and never use `.exe` packers (UPX/VMProtect).
9. **No emojis anywhere.** Never use emojis in CLI output, UI components, tray labels, notifications, documentation, commit messages, or comments. Use clean typography, geometric SVG icons, or plain-text badges (`[WAIT]`, `[RUN]`, `EXACT`, `EST`).

## 3. Repo map

| Path | Contents |
| :--- | :--- |
| `crates/core` | Domain types, pricing, quota, rules, redactor. **Pure: no I/O, no async.** |
| `crates/adapters` | One module per agent. `parse(bytes) -> Vec<RawEvent>`, pure and deterministic. |
| `crates/ingest` | Watcher, reconciler, tailer, checkpoints. |
| `crates/store` | SQLite schema, migrations, queries, rollups. |
| `crates/platform` | OS-specific code: paths, process scan, focus, autostart. All `cfg(target_os)` code lives here. |
| `crates/daemon` | axum HTTP/WS API, IPC server, orchestration. |
| `crates/cli` | The `fleethud` binary. Thin: parse args, call into the other crates. |
| `apps/tray` | Tauri v2 tray (M3). |
| `web/` | React dashboard (M2). |
| `npm/` | npm shim and per-platform binary packages. |
| `fixtures/` | Anonymized real agent logs, organized as `fixtures/<agent>/<version-or-date>/`. |
| `docs/` | `SPEC.md`, `adr/`. |

**Dependency direction** (enforced in review): `core` ← `adapters`, `store`, `platform` ← `ingest` ← `daemon` ← `cli`. `core` depends on no internal crate. No cycles.

## 4. Commands

```sh
cargo fmt --all                                            # format
cargo clippy --workspace --all-targets -- -D warnings      # lint (CI gate)
cargo test --workspace                                     # tests (CI gate)
cargo deny check                                           # licenses/advisories (CI gate)
cargo run -p fleethud -- <subcommand>                      # run the CLI
```

To run the full local gate, use the `rust-quality-gate` skill (`.agents/skills/rust-quality-gate/`).

## 5. Rust standards

- **Edition 2024**, MSRV is set in the root `Cargo.toml` (`rust-version`). Lints are configured once in `[workspace.lints]`, and every crate has `[lints] workspace = true`.
- **Errors:** library crates use `thiserror` enums, one `Error` per crate. `anyhow` is allowed **only** in `crates/cli` and in tests. Never `unwrap()`/`expect()` in non-test code (clippy denies it). Add context when converting errors.
- **No panics on bad input.** Adapters must tolerate malformed, truncated, unknown-field and unknown-type lines: skip them, count them, move on. One bad line must never kill ingestion.
- **`unsafe`** is denied workspace-wide. It's allowed only in `crates/platform`, under a module-level `#[allow(unsafe_code)]`, with a `// SAFETY:` comment on every block.
- **Async:** tokio only, and only in `ingest`, `daemon` and `cli`. `core`, `adapters` and `store` stay sync. Run blocking SQLite work via `spawn_blocking` or a dedicated thread.
- **Logging:** `tracing`. Never log content fields or tokens. Levels: `error` = needs user action, `warn` = degraded, `info` = lifecycle, `debug`/`trace` = dev.
- **Dependencies:** prefer well-maintained crates with permissive licenses (allow-list in `deny.toml`). Add them to `[workspace.dependencies]` first, then reference with `{ workspace = true }`. Justify each new dependency in the PR description.
- **Public API:** every `pub` item in library crates gets a doc comment. Keep things `pub(crate)` unless another crate needs them.
- **Paths:** never hardcode `/` or `\`. Use `Path`/`PathBuf` and resolve home and app dirs through `platform::paths`. Respect env overrides (`FLEETHUD_HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`).
- **Time:** store UTC (`time` or `chrono`, one only, chosen in M1). Convert to local time only in the UI.

## 6. Testing standards

- **Adapters:** golden tests against `fixtures/`. Parse the fixture and compare with a checked-in snapshot (`insta`). Every bug report about parsing becomes a new fixture first.
- **Fixtures must be anonymized** with `.agents/skills/add-agent-adapter/scripts/anonymize-fixture.mjs`. Never commit raw personal logs.
- **Ingest:** tests for truncation, rotation, partial lines, crash between read and commit (no double counting), and duplicate lines across resumed sessions.
- **Store:** migration tests start from an empty DB and from every previous schema version.
- **Platform code:** unit-test the pure parts; mark OS-only tests with `#[cfg(target_os = "...")]`. CI runs on all three OSes.
- Tests must not touch the real `~/.claude`, `~/.codex` etc. Use temp dirs (`tempfile`) and env overrides.

## 7. Cross-platform rules

- Every feature is designed for **Windows, macOS and Linux**. If a platform can't support it, implement a documented fallback (SPEC §3.12–3.13) rather than skipping it silently.
- Windows file reads use share mode `READ | WRITE | DELETE`. Never hold agent log files open while idle.
- Line endings: the repo is LF (`.gitattributes`). Parsers must accept both `\n` and `\r\n`.
- Before merging platform-touching code, run the `cross-platform-review` skill.

## 8. Workflow

- **Conventional Commits:** `feat(adapters): …`, `fix(ingest): …`, `docs(spec): …`, `chore(ci): …`. Scope = crate or area.
- **Branch & PR workflow:** Never commit directly to `main`. Create a focused feature branch (`feat/...`, `fix/...`, `docs/...`), keep each commit small and atomic, verify locally with `.agents/skills/rust-quality-gate/scripts/check.ps1` (`check.sh` on Unix), and merge to `main` via a GitHub Pull Request (`gh pr create` + `gh pr merge`).
- **Session log (`docs/SESSION_LOG.md`):** Update [`docs/SESSION_LOG.md`](docs/SESSION_LOG.md) in every phase/PR with goals, outcomes, problems encountered, root-cause fixes, and decisions made.
- Keep each change small and focused. When a roadmap item finishes, update its status in SPEC §5.
- New agent support → the `add-agent-adapter` skill. Releases → the `cut-release` skill.
- **Installed engineering skills (`.agents/skills/`):**
  - `ponytail` (`ponytail`, `ponytail-review`, `ponytail-audit`, `ponytail-debt`, `ponytail-gain`): minimalist filter ladder — write the smallest code that works, reuse stdlib/existing crates, avoid speculative abstractions.
  - `test-driven-development`, `systematic-debugging`, `verification-before-completion` (`obra/superpowers`): strict red-green-refactor TDD, root-cause debugging, and evidence-backed completion checks.
  - `rust-best-practices`, `tauri-v2`, `supabase-postgres-best-practices`: domain guides for idiomatic Rust, Tauri v2 tray/IPC, and Supabase RLS/schema design.
  - `rust-quality-gate`, `add-agent-adapter`, `cross-platform-review`, `cut-release`: FleetHUD-specific runbooks.
- Don't add features that aren't in SPEC. Propose them in SPEC (and an ADR if architectural) first.
- Don't rewrite or delete existing comments and docs that are unrelated to your change.
