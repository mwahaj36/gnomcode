# Windows downloads and local update manifest

Date: 2026-11-13. Owner: codex-release.

## Goals and outcomes

Keep downloads Windows-only and resolve artifact URLs from package version/branding. Add the previously missing checksum and latest.json allow-list entries. Scope copy and SEO changes to downloads plus its llms.txt description. Existing responsive tabs, keyboard handling, typography and colors are preserved.

Prepare a local manifest from the actual packaged installer and matching versioned checksums. No fake hash, signature, publication timestamp, non-Windows artifact or published-availability claim is added. Existing artifact signature format remains unchanged. Generated latest.json is ignored by Git; owner signing/publication stays separate.

## Problems, decisions and verification

Existing checksum links pointed to a route that rejected them. The updater's latest.json had no source-generation path. Generator regression failed before implementation, then passed for literal expected version/URL/target/hash; tampered input leaves previous output intact. The generator depends on existing packaging PE/CLI version verification and does not claim to attest a PE version itself.

Used Sites local-only workflow, apple-design and impeccable: keep actions explicit, preserve platform tab accessibility, disclose candidate/unsigned status and avoid unverified platform promises. No deployment or credential operation.

Full dedicated-worktree web npm ci/test/typecheck/build passed (127 tests); existing module/hook warnings and 6 moderate npm audit findings remain. CARGO_INCREMENTAL=0 with dev/test debug symbols disabled: fmt, workspace clippy -D warnings and 499 workspace tests passed. No Cargo.lock changes. The same downloads source files were copied byte-for-byte into the 1.0.0 integration build: 127 tests, typecheck/build, rendered version checker and local HTTP passed. Installer/ZIP/checksum/latest.json return exactly the configured v1.0.0 release-asset redirects; unknown asset returns 404. The generator also prepared a manifest from the real local Windows installer/checksums without fabricated values. Browser review found no horizontal overflow at a 390px viewport and macOS exposed no installer action. Desktop/mobile screenshots and accessibility state were inspected; no live deployment was changed. Package script parses without errors. Diff check passes; U+00C2 count is 0. Release metadata resolves to 1.0.0 after PR #243; this branch intentionally does not duplicate version edits. Signing procedure depends on PR #245. macOS/Linux and public URLs are unverified.

Final follow-up: rebased onto 3943dbb; fmt, full workspace clippy/499 tests and web 127 tests/typecheck/build passed again. The integrated Windows packaging workflow completed the new generator step successfully with real artifacts (PR #248); final manifest SHA-256 matches the retained installer. No live release or deployment exists for 1.0.0 yet.
