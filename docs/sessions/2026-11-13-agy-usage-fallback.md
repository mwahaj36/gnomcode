# G7 verified native usage fallback

## Goal and gap

Baseline `0d439e6`: `crates/daemon/src/vendor_limits.rs:303-363` requires live language-server descriptors and otherwise returns unavailable; its check dispatcher at :381 has no CLI fallback. Add only the already-authorized fallback wiring there, keeping the handler/probe in new agy_usage.rs and pure parsing in the owned adapter.

## Native evidence before implementation

Actual installed agy 1.5.7: hidden shell-free `--output-format text -p=/usage` in a disposable workspace exited zero in 4,750 ms with 339 stdout bytes, no stderr and five rows (exact command outcomes). Fifteen actual tabs, no literal backslash-t separators. Native diagnostic logs contain retrieveUserQuotaSummary and no cascade/generation markers; a controlled earlier marker generation contains cascade/streamGenerateContent as positive control. No credentials entered or raw diagnostic logs checked in. Independent wire-level proof is not claimed.

Converted safe quota metadata into JSONL fields and ran the mandatory anonymize-fixture.mjs, inspected it, then reconstructed the parser's TSV fixture from those anonymized fields. This avoids placing raw agent logs in the repository. Initial PowerShell conversion coerced RFC3339 strings to locale dates, caught by the strict parser. Corrected fixture preparation with JSON string access; reconstructed TSV now hashes identically to the captured safe metadata bytes. Parser validation was not relaxed. The adapter-format skill reference is outside ownership; canonical format/source details are recorded in SPEC, ADR and the fixture README instead. No new log-root/SourceAdapter/usage-token ingestion was introduced.

## Implementation and policy

Existing server readings win. Existing opt-in and polling cadence stay unchanged. Fallback uses the official hidden CLI in a temporary workspace with closed stdin, discarded stderr, 64 KiB output caps and one eight-second timeout for version plus usage. Exact version 1.5.7 is required before issuing usage; unknown versions never receive the slash command. No prompt/model/bypass arguments, new credentials/config reads or output persistence. Pure parser emits canonical API-basis pools with validated windows, percentages and resets; malformed/unknown/duplicate rows are skipped.

Owner separately authorized minimal vendor_limits.rs wiring and the existing API source-label expectation/copy change to `Native limits`. The short neutral label covers both sources; no frontend/layout/theme/control changes. Apple-design writing/label guidance and Impeccable clarify/craft-floor were applied to copy, preserving the existing system typography and source disclosure. Impeccable context reported pre-existing product schema/design-sidecar drift; those files are outside scope and were not repaired.

## Verification

- Fresh parser red: empty scaffolding returned zero rows instead of five. Golden and malformed-input tests pass after implementation; snapshot inspected for all groups, API trust, remaining-to-used conversion and UTC resets. CRLF and duplicate parity checked.
- Fresh process red: fallback scaffolding rejected the preferred server reading. Controlled compiled fake CLI tests now pass for server preference/no CLI, exact argv, verified version only, failure, malformed output, oversize and hung-process termination. All executable/config/workspace state is temporary; fake CLI never reads native agent homes.
- Source-label expectation freshly failed Language server versus Native limits before changing the production label.
- Initial complete Rust gate passed before the label change. Final complete Rust quality-gate script PASS after the label change, with CARGO_INCREMENTAL=0 and isolated GNOMCODE_HOME/CLAUDE_CONFIG_DIR/CODEX_HOME/GEMINI_HOME: fmt, workspace clippy -D warnings, workspace tests, cargo-deny and core purity. Golden snapshot was generated once, inspected, then verified normally by the full gate without snapshot auto-update.
- Final web npm ci/test/typecheck/build PASS with NEXT_TELEMETRY_DISABLED=1; no tracked web files changed. Browser visual/physical-device acceptance not run; the only displayed-copy change is the shorter source-label text with existing markup/styles untouched.
- No new endpoint/JSON shape/setting/dependency. Existing source_label now says Native limits. Desktop not touched; native macOS/Linux, physical-phone and browser visual acceptance unverified.

## Dependencies and limits

Depends on #189; independent of #187 and #193. ADR 0029 addendum documents the new source/version gate. Actual command and no-generation native trace verified live; production fallback failure paths tested with a fake CLI. Did not shut down the user's shared language server to simulate absence, did not probe other versions and did not run an independent network audit. No suppressed check or broader permission granted.
