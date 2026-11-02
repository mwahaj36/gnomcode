# 2026-11-02 — PR 118 repair retry

## Goal

Fix the review finding in PR 118, recheck PRs 118 and 119, and merge them in order.

## Root cause and fix

A source read failure was skipped before writing the global format-repair completion marker. Track failed reads/checkpoint lookups, keep the marker pending, and run normal ingestion through a bounded second pass with repair disabled. A shared file-pass function lets the regression reproduce a real file disappearing after discovery without OS-specific locking or mocked readers.

## Regression

Extended the existing isolated temporary-log test: a legacy log disappears after discovery, another log still imports, repair remains pending, and restoring the original log repairs Markdown while preserving its event ID, checkpoint and source bytes. Observed failure on the old completion behavior, then a passing run after the fix.

## Decisions and limits

No schema or agent-file changes. Already-missing originals cannot reconstruct lost formatting. The next scheduled pass retries transiently failed discovered sources. Windows/macOS/Linux share this path without new platform-specific behavior.

## Verification

Targeted regression passed after the expected failing run. Full Windows Rust gate passed (format, clippy, workspace tests, cargo-deny and core purity). Web: 20 tests and production build passed. Desktop: 18 tests including Electron terminal roundtrip passed. npm shim: four tests passed. Relay: six tests passed. Existing React hook lint warnings remain. Cross-platform source review found no new platform-specific behavior; native macOS/Linux and physical-phone acceptance were not run. PR 119 receives this fix before its final gate and both PRs merge in order.
