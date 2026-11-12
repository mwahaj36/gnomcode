# 2026-11-13 - Git viewer hardening (research gap G10)

## Goals
- Make the repository diff/status endpoint tolerate concurrent agent git use, non-ASCII file names and non-UTF-8 content.

## Outcomes
- `git_output` in `crates/daemon/src/api.rs` sets `GIT_OPTIONAL_LOCKS=0` and `-c core.quotepath=off`, and decodes lossily, returning a lossy flag instead of 422.
- `GET /api/v1/repos/diff` runs `status --porcelain=v1 -z` (NUL separators converted to newlines so the field stays line-oriented) and adds a `lossy` boolean.
- SPEC 3.6 amended. New test uses a temp repo with a non-ASCII name and a non-UTF-8 change; it returns early if git is missing.

## Problems encountered
- Gap verified in code: only `GIT_TERMINAL_PROMPT` was set, status was plain `--porcelain` and `String::from_utf8` failed with 422. No client in the repo consumes the `status` field, so the format change is safe.

## Decisions
- `repo_file_get_handler` (reads one file, 500 on non-UTF-8) is unchanged; it runs no git. Not in scope for the hardening.
- Names containing newlines split into several lines; documented as a shortcut in code.

## Verification
- New and existing git_output tests pass; full gate results in the PR.
