# Antigravity surfaces: IDE, CLI and the 2.0 app (2026-11-13)

## Goal

Every Antigravity surface ingested, attributed and counted exactly once.

## What each surface is (evidence from this machine)

| Surface | Program | Data directory | Stored surface |
| :--- | :--- | :--- | :--- |
| Antigravity 2.0 | `Programs/Antigravity/Antigravity.exe`, Electron app `package.json` "Antigravity - Agentic Desktop Application" 2.31.2, hub updater, bundled `language_server.exe` | `~/.gemini/antigravity` (app code derives it from the lower-cased product name) | `desktop_app` |
| Antigravity IDE | `Programs/Antigravity IDE/Antigravity IDE.exe`, VS Code fork 1.107.0 | `~/.gemini/antigravity-ide` | `ide` |
| Antigravity CLI | `agy` 1.5.8 (`~/.gemini/bin/agy.version.json` says 1.5.5, `agy --version` says 1.5.8) | `~/.gemini/antigravity-cli` | `cli` |

The 2.0 app's `paths.js` names `~/.gemini/antigravity` as the "old IDE" directory, `~/.gemini/antigravity-ide` as the destination of its IDE-install copy and `~/.gemini/antigravity-backup` as the backup. So 2.0 is a separate product, not a newer IDE build: it reuses the old directory and holds byte copies of the 198 IDE conversations it migrated (`migrate_convos_into_projects: COMPLETED` in `antigravity_state.pbtxt`). The IDE does not mark editor versus Agent Manager conversations in storage (trajectory `source` is 1 for all of them), so both are `ide`.

Each directory has `brain/<id>/.system_generated/logs/transcript.jsonl` and `conversations/<id>.db`. The `.db` is a SQLite trajectory; `gen_metadata` has one protobuf row per model call. `trajectory_meta.source` is 1 (IDE and app), 17 (CLI) and 22 (one app conversation, meaning not identified). `.pb` conversations (87 here) are encrypted; 83 have no transcript either and stay unread. `antigravity-backup` is a frozen pre-migration copy and is not read.

## Bugs found

1. IDE 1.x tokens were never counted. IDE transcripts carry no token fields (19,993 `PLANNER_RESPONSE` steps, all zero), so 115 IDE sessions had no usage. The trajectories hold 18,454 calls: 210,012,876 input, 10,298,627 output, 1,947,436,786 cache-read tokens.
2. 2.0 app sessions were labeled `ide` (the adapter only knew `antigravity-cli` versus everything else).
3. Seven app conversations have transcripts without token fields; six of them have calls in the trajectory (13.1 M input tokens) that were missing. Where a transcript does carry tokens, the trajectory totals equal them exactly, per conversation and per token class (CLI 17 of 17, app 60 of 60).
4. A migrated copy shares its conversation id with the IDE original, so labeling by file path would flip the surface, and reading both would double count.
5. `find_repo_root_from_path` returned the drive root for a path outside any repository, and the last tool-call path overwrote the workspace the application recorded: 34 sessions landed in `unknown`, 21 of them with a recorded workspace.
6. The 2.0 app's last chosen model was used as the fallback model for CLI and IDE sessions.
7. Window focus for an IDE session targeted the 2.0 app's process (`Antigravity`), and install detection ignored `Antigravity IDE.exe`.

## Changes

- `adapters::antigravity`: surface by data directory, owner precedence (CLI, IDE, app), protobuf readers for `gen_metadata` and `steps.metadata`, usage records keyed `agy-gen:<id>:<row>`, a switch that leaves usage to the trajectory, workspace precedence, no-root-for-unrelated-paths.
- `store::trajectory`: read-only trajectory reads; one transaction inserts usage, deletes the same session's transcript-keyed usage and writes the checkpoint; surface repair; `surface_label` on session list items (`core::surface_label`).
- `ingest::antigravity`: catalog of ids across directories, owner surface applied to parsed batches and stored sessions, per-file checkpoint by size and mtime (including `-wal`), copies read only when they hold calls the owner lacks.
- `platform`: IDE and app focus targets and install detection.
- Fixtures `fixtures/antigravity/{cli-1.5.8,ide-1.107.0,app-2.31.2}`: transcripts anonymized with `anonymize-fixture.mjs`; `trajectory.json` blobs are minimal protobuf carrying only the fields read, with real token counts and synthetic ids.
- `scripts/verify-ingest.py` now takes Antigravity truth from the trajectory databases (`GEMINI_ROOT` override).

## Verification

Temp copy of the real data (`GEMINI_HOME` and `GNOMCODE_HOME` redirected, nothing under `~/.gemini` or `~/.spawnt` written):

| | Before | After |
| :--- | :--- | :--- |
| Sessions | cli 19, ide 182 (67 of them app), app 0 | cli 19, ide 115, desktop_app 67 |
| Usage rows | 17,035 | 36,936 |
| Input / output / cache-read tokens | 75.36 M / 8.14 M / 2.089 B | 298.43 M / 19.19 M / 4.206 B |
| IDE tokens | 0 | 210,012,876 / 10,298,627 / 1,947,436,786 |
| Sessions with usage | 77 | 192 |
| Sessions in `unknown` project | 34 | 13 (no workspace recorded anywhere) |

`verify-ingest.py` against the trajectory databases: 192 sessions, zero per-session differences, all three token classes OK. A second and third pass changed nothing. Unit tests per surface and an end-to-end ingest test (each surface once, migrated copy not recounted, copy continued in the app counted once, re-ingest idempotent) pass.

## Open

- A conversation continued in the 2.0 app only appears through its copy; the IDE copy stays the owner and keeps the `ide` surface.
- `.pb` conversations cannot be read; their sessions exist only when a transcript does.
- Trajectory `source` 22 is unidentified; it does not affect ownership.
- macOS and Linux paths for the IDE and app are unverified; Windows only.
