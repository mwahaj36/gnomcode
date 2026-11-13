# 2026-11-13 - Host sweep fails open

## Goals
- Make the exited-host sweep from #229 (`end_exited_hosted_sessions`) end sessions only when the host listing succeeded and agrees with a fresh process check. An error or a suspicious empty list must end nothing.

## Problem and root cause
- The automation tick built its live-host list from `list_active_sessions`, which hides every failure: a failed `read_dir`, an unreadable or torn record (Windows share violation, partial write) and a record it could not parse all produce a shorter or empty list, and it also deletes records of hosts it thinks are dead. The store then ended every bound busy session whose host was missing from that list, so one transient read error ended live sessions. A host that started between the listing and the database read had the same effect.

## Outcomes
- `pty`: `try_list_active_sessions` returns an error instead of a partial list and never deletes; `host_is_running` is a fresh per-host process check that returns `Ok(false)` only for a missing record or dead process and an error when the record is unreadable.
- `store`: `unlisted_bound_hosts` lists the sweep candidates; `end_exited_hosted_sessions(live, known_alive)` ends nothing when any host the process check found alive is missing from `live`.
- `daemon`: `end_exited_hosts` returns on a listing error; `sweep_exited_hosts` rechecks each unlisted bound host and returns on any unknown result or any host still running before ending anything.
- SPEC 3.7 text and the roadmap updated.

## Verification
- Tests: pty `checked_listing_reports_unreadable_records_instead_of_a_short_list` and `fresh_host_check_distinguishes_gone_from_unknown`; store `sweep_ends_nothing_when_a_host_known_alive_is_missing_from_the_listing` and `unlisted_bound_hosts_are_the_sweep_candidates`; daemon `host_sweep_ends_nothing_unless_the_listing_is_complete` and `host_sweep_keeps_a_listed_host`.
- `CARGO_INCREMENTAL=0`: fmt, clippy `-D warnings`, `cargo test --workspace`.
