# Contributing to Gnomcode

Gnomcode is licensed under the Apache License 2.0. Read [`AGENTS.md`](../AGENTS.md) first: it holds the invariants (no telemetry, fail open, loopback only, trust labels on every number, business logic in Rust).

## Workflow

1. Open an issue for anything beyond a small fix.
2. Branch from `main` (`feat/...`, `fix/...`, `docs/...`). Never commit to `main`.
3. Keep commits small. Use Conventional Commits: `feat(adapters): ...`.
4. Run the local gate before opening a pull request:
   - `cargo fmt --all -- --check`
   - `cargo clippy --workspace --all-targets -- -D warnings`
   - `cargo test --workspace`
   - `cargo deny check`
   - For `web/` and `apps/desktop/` changes: `npm ci`, typecheck, lint, test, build.

## Developer Certificate of Origin

Sign off every commit with `git commit -s`. This certifies the [DCO](https://developercertificate.org/): you wrote the change or have the right to submit it under Apache-2.0. Pull requests with unsigned commits are not merged.

## Style

No emojis anywhere (code, docs, UI, commits). Fixtures must be anonymized with the script named in [`AGENTS.md`](../AGENTS.md); never commit raw personal logs.

By contributing you agree to follow [`CODE_OF_CONDUCT.md`](../CODE_OF_CONDUCT.md).
