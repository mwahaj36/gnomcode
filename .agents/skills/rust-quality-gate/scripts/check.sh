#!/usr/bin/env sh
set -eu

echo "==> 1/5 cargo fmt --check"
cargo fmt --all --check

echo "==> 2/5 cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> 3/5 cargo test"
cargo test --workspace

echo "==> 4/5 cargo deny (if installed)"
if command -v cargo-deny >/dev/null 2>&1; then
  cargo deny check
else
  echo "    (skipped: cargo-deny not installed; install with 'cargo install cargo-deny')"
fi

echo "==> 5/5 core purity check"
if grep -E "^[[:space:]]*(fleethud-|tokio|rusqlite|notify|axum)[[:space:]]*=" crates/core/Cargo.toml >/dev/null 2>&1; then
  echo "ERROR: crates/core/Cargo.toml must stay pure (no internal, I/O, or async crates)." >&2
  exit 1
fi

echo "All quality gates passed."
