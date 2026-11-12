# 2026-11-12 - Home board group from session presence

## Goals
- Derive a Home board group (`needs_you`, `running`, `recent`) from `SessionPresence` in Rust, per SPEC section 8.3 of the console overhaul plan.
- Expose it on the session list payload without changing any existing field.

## Outcomes
- `HomeGroup` enum and `SessionPresence::home_group()` added in `crates/core/src/presence.rs`.
- `SessionListItem` (store) carries `home_group: Option<HomeGroup>`, set from presence, served by `/api/v1/sessions`.

## Problems
- `/api/v1/conversations` rows are built from state strings in `crates/daemon/src/remote.rs`, not `SessionPresence`, so they carry no `is_ongoing`; left unchanged to keep the change minimal.
- `HomeGroup` has no `Default`, so the serde-default field is an `Option` rather than a plain enum.

## Decisions
- The field is an `Option` with `#[serde(default)]` so payloads from older daemons still deserialize.
- The cloud sync payload in `crates/daemon/src/sync.rs` is not extended; Home grouping is local presentation only.

## Verification
- `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: `home_group_maps_every_presence` covers all five presence variants; the store project-filter test asserts the field matches presence.
