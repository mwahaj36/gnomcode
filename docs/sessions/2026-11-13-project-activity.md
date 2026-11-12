# 2026-11-13 - Project names and Project activity

## Goals

The web console Project activity list showed long runs of identical full-width bars named `c`, `c:`, `d:`, `a2`, `bas`, `cle`, `co`, `can`, `cli`, `daemon`, `desktop`, `account-retest-1793248674048`. Find where project identity goes wrong, fix it in Rust, and redesign the list.

## Root cause (read-only copy of the real database, 131 project rows for 345 sessions)

Project identity was the last folder segment of whatever folder an agent reported as the session cwd. Nothing checked that the folder was a project. Evidence, by name class:

- `c:` (10 sessions), `d:` (6), `ubuntu-22.04` (WSL share root): cwd was a drive or share root, so the "project" was the drive letter.
- `bas`, `c`, `can`, `cle`, `co`, `go-x20`, `i-x20`, `ngl`, ...: not truncated paths. They are real Codex per-chat scratch folders `Documents\Codex\<yyyy-mm-dd>\<slug>`, where the slug is the start of the first prompt. 26 sessions, most in single-session projects.
- `account-retest-1793248674048`, `workspace`, `gnomcode-live-audit-20261106`: test and temp directories (`D:\spawnt-ide-test-temp\...`, `AppData\Local\Temp`).
- `cli`, `daemon`, `desktop`, `web`, `src`, `core`, `store`, ...: subfolders of one repository, plus about two dozen `agent-*` folders from `.claude\worktrees`. The repository was never resolved, so one repo appeared as dozens of projects.
- Duplicates by spelling: `c://users//wahaj//...` (doubled backslashes from Antigravity workspace URIs) beside `c:/users/wahaj/...`; `infosec%20assignment1` and `new%20folder` (percent-encoded `file://` URIs); `wsl.localhost/...` with and without `//`; Codex keyed by Git remote while the other agents were keyed by path.
- Names were lowercased because they were taken from the lowercased key (`subtext`, `hAIst` shown as `haist`).

## Outcomes

- `crates/core/src/project_path.rs` (new, pure): normalizes the spelling and classifies a folder as Unassigned, Codex chat, temporary or project. `Project::from_origin` uses it; names keep their spelling. Existing ids for ordinary single-slash paths are unchanged.
- `crates/ingest`: `resolve_project` (nearest ancestor with `.git`, worktrees resolve to the main repository, remote key when present, folder name as display name), applied to every parsed batch, and `reproject_all` which recomputes every stored project at the start of each ingest pass and after the Antigravity reconcile. Derived fields only: sessions, usage and chat are never deleted.
- `crates/store`: `list_projects`, `reassign_project` (merge by canonical key in one transaction); the project report skips Unassigned and gains `repo_root` (empty for scratch buckets), `last_active` and `agents`; the Antigravity reconcile no longer re-points a session that already has a project.
- UI: one presenter (`mountProjectList` in `web/lib/console-presentation.js`) for the web Usage page and the local Usage > Projects. Left-aligned 44px rows with folder icon, name, shortened folder (local) or agents (web), agents, session count, last active. Newest first, ten rows, a filter and Show all beyond ten, one helper line. Recorded tokens and price coverage moved into the row tooltip. CSS is a labelled section at the end of `gnomcode-console.css`.

Real data after the change (temp copy, `GNOMCODE_HOME`): 131 project rows became 36; 345 sessions before and after; `c:`, `d:`, the Codex slugs, the `agent-*` and crate folders are gone; `fleethud` holds 76 sessions.

## Problems and decisions

- The user guessed truncated prefixes; the data says otherwise (real scratch folders). They now share one `Codex chats` row instead of being hidden, so spend is not lost.
- Review blocker: resume must not use a merged project's last observed folder. Sessions now record their own folder (`session.cwd`, additive column); ingest records it before resolution, and legacy rows are backfilled from their old one-folder project just before it merges. `get_session_focus_target` returns the session folder, else the project root for repository projects only; Codex chats, Temporary and Unassigned have no fallback and resume/launch is refused with a clear message. Recorded in SPEC section 3.6.
- A remote-keyed project whose folder is offline can be renamed to the remote's name and back when the drive returns; cosmetic.
- Sibling worktree folders outside the repository (`fleethud-wt/phase4`) cannot be resolved without the repository and stay separate.
- No agent logos: the monochrome marks exist only on the marketing site and the local console cannot serve them; agent names are text.

## Verification

- `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (CARGO_INCREMENTAL=0). New tests: core (one per name class), ingest (recompute per class, repository resolution with a temporary `.git`, remote key kept), store (rollup fields and Unassigned exclusion, reassign).
- Web: `npm ci`, `npm test` (85 pass, `project-activity.test.mjs` is new), `typecheck`, `build`.
- Browser with synthetic data at 390px and desktop, dark and light: 44px rows, no horizontal scroll, filter, Show all.

## Open items

- None blocking. Folder-name display for remote-keyed projects could use a stored label if flapping proves visible.
