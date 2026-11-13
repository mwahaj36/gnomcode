# Release 1.0.0 version preparation

Date: 2026-11-13. Owner: codex-release.

## Goals and outcomes

Prepare lockstep candidate metadata and user-facing changes since 0.9.4 without publishing artifacts. Ran the cut-release bumper, including all seven npm platform packages. Branding check passes without regeneration.

## Problems, root causes and decisions

The desktop lockfile still reported 0.9.3, and the bumper did not update Cargo.lock. A failing isolated-fixture regression reproduced the desktop mismatch before the bumper was fixed; the test now checks both lockfiles. No dependency resolution changed.

The source v0.9.4 tag is absent locally and on origin. Wahaj explicitly approved commit 7cec4e6 as the baseline. Release notes cover its descendants through 496d740, grouped by user-facing behavior. Claude must decide any tag correction at release time; no tag was created.

Shared docs receive append-only completion records. Existing launch gates remain open. No forbidden console files were changed.

## Verification

- Bumper regression: failed first (0.9.3 versus 1.0.0), then passed.
- scripts/sync-branding.mjs --check: passed.
- CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0: cargo fmt --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace: passed. Debug symbols disabled to preserve disk space.
- cargo deny check: passed (duplicate-dependency warnings only); Cargo.lock changes only local package versions.
- Desktop npm ci/lint/test/typecheck/build: passed; 40 tests. npm audit reported 12 moderate vulnerabilities.
- Web npm ci/test/typecheck/build: passed; 133 tests. Existing React hook/module-type warnings; npm audit reported 6 moderate vulnerabilities.
- scripts/check-release-versions.mjs after web build: passed for 1.0.0.
- npm/check-release.cjs: failed because release binaries/EULA/notices are not staged in the npm platform packages. This is artifact readiness, not version consistency; npm publication is not verified.
- git diff --check passed; diff U+00C2 marker count is 0.
- No installed-app mutation, package upload, key creation, tag, release or deployment occurred.
