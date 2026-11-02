# 2026-11-02 — Gnomcode rename and PR 118 review

## Goals

Review PR 118, then rename the active product to Gnomcode with working site `https://gnomcode.vercel.app`, using ponytail ultra.

## Outcomes and decisions

- Updated UI, CLI/Rust crates, npm packages, desktop, installer source, CI/release scripts, legal copy, current documentation and named assets.
- Rebuilt the social preview with the Gnomcode wordmark.
- Added legacy data-home fallback without moving/deleting history; ADR 0026 documents compatibility exceptions.
- Kept real GitHub URLs, deployed Worker/database names, encryption labels and historical records stable. No production database changes.
- Existing untracked Spawnt release binaries are not relabeled or included.

## PR 118 review

What this change does: PR 118 restores mobile reconnect, renders safe Markdown, preserves formatting during redaction and repairs matching existing imported history. Review assumes one local desktop with multiple remembered account devices.

### Should fix

1. **A temporary read failure permanently skips Markdown repair** (`crates/ingest/src/lib.rs`, `read_from_offset` failure and unconditional `finish_chat_format_repair`).
   - What this is: the one-time repair rereads original agent logs and updates matching stored messages.
   - Problem: an unreadable source is skipped, but the global completion marker is still written. When the source becomes readable later, its old messages are never repaired.
   - Fix: retain pending repair status for skipped sources, while keeping normal ingestion running independently.
   - If skipped: affected old conversations keep flattened Markdown indefinitely. New ingestion continues normally.

Verdict: fix 1 before claiming complete automatic repair. No usage replay or deletion restoration was found in the repair path. Physical-phone/provider acceptance and non-Windows native behavior were not exercised.

## Problems and root causes

- Broad identity substitution initially touched applied Supabase migration identifiers. Restored those identifiers and corresponding clients before validation. Schema identity requires a coordinated migration, not branding replacement.
- Browser tooling was absent; installed Playwright in a temporary tools directory without adding a project dependency.

## Verification

- Full Windows Rust quality gate passed: rustfmt, clippy with denied warnings, workspace tests, cargo-deny and core purity.
- Web: 20 tests and production build passed. Browser checks passed for 19 public routes, metadata, links, schemas, sitemap, robots and responsive layouts; this includes the freshly built Gnomcode downloads.
- Desktop: all 19 tests passed, including a real Electron terminal roundtrip and history-home compatibility.
- npm shim: four tests passed. Relay: six tests passed. Browser/Rust key agreement remained compatible.
- Synthetic cross-platform packaging, native console/navigation, Windows tray and Explorer URL checks passed.
- Fresh Windows release packaging passed installer payload/version/notices/launcher checks and installer-option checks. SHA-256 manifests match all three Gnomcode artifacts. `target/release/gnomcode.exe --version` reports `gnomcode 0.9.0`.
- History fallback was tested red then green in both Rust and desktop.
- Cross-platform source review: Rust uses `PathBuf`; history remains in place; listeners/auth validation remain intact. Native macOS/Linux execution, physical-phone acceptance and a real install/uninstall were not performed.
- Live `gnomcode.vercel.app` returned HTTP 200 with the prior Spawnt title. Vercel identifies it as the existing `spawnt` project. No deployment, external repository rename or npm publication was performed; this PR is stacked on still-open PR 118.

## Final recheck before merge

The PR 118 repair finding is resolved in commit `5f7807b`, merged into this branch. A second regression reproduced older handoff context appearing in chat after the rename. Preserve the internal `<spawnt_context>` envelope as a compatibility identifier; observed the old-history regression fail, then pass after restoring the tag in the producer and reader. Both session-index entries are retained after the branch merge. Final combined Rust quality gate passed (format, clippy, workspace tests, dependency audit and core purity). Rechecked 20 web tests and production build, 19 desktop tests, four npm tests, six relay tests, synthetic cross-platform packaging, 19-route browser SEO/link/responsive checks and the native chat/workspace browser flow. Both regressions passed after observed failing runs. Existing React-hook lint warnings remain; native macOS/Linux and physical-phone acceptance were not run.
