$ErrorActionPreference = "Stop"

Write-Host "==> 1/5 cargo fmt --check"
cargo fmt --all --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> 2/5 cargo clippy"
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> 3/5 cargo test"
cargo test --workspace
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host "==> 4/5 cargo deny (if installed)"
if (Get-Command cargo-deny -ErrorAction SilentlyContinue) {
    cargo deny check
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} else {
    Write-Host "    (skipped: cargo-deny not installed; install with 'cargo install cargo-deny')"
}

Write-Host "==> 5/5 core purity check"
$coreLines = Get-Content "crates/core/Cargo.toml" | Where-Object { $_ -notmatch '^name\s*=' }
if ($coreLines -match "^\s*(spawnt-|tokio|rusqlite|notify|axum)\s*=") {
    Write-Error "crates/core/Cargo.toml must stay pure (no internal, I/O, or async crates)."
    exit 1
}

Write-Host "All quality gates passed."
