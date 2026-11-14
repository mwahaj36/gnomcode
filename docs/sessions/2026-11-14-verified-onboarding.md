# 2026-11-14 - Verified provider onboarding

## Goals

Replace the four-step tour (#183) with one verified onboarding: pick providers, run a live checklist per provider (install, sign in, integrate or protocol approvals, Claude limits, first data), optional phone pairing, summary. Reuse the same checklist in Settings > Providers and from the doctor.

## Outcomes

- PR A (Rust): `GET /api/v1/onboarding/status?provider=&distro=` in `crates/daemon/src/onboarding.rs` (ADR 0061). Doctor CLI points to Settings > Providers.
- PR B (UI): `ProviderChecklist` component (onboarding, Settings > Providers, `#settings/providers` link from the doctor), `Onboarding` dialog with resume state, polling every 2 s only while visible.

## Problems and root-cause fixes

- Two-second polling would spawn `<cli> --version` each time: added a six-second probe cache.
- WSL checks must not boot a distro: a stopped distro returns `unknown` steps.

## Decisions

- Sign-in is a file-presence check; token values are never read. Antigravity sign-in is `estimated`.
- "Run for me" integrate deferred; copyable commands only.

## Verification

Rust: fmt, clippy -D warnings, `cargo test --workspace` (CARGO_INCREMENTAL=0), cargo deny. Web gates and Playwright listed in PR B.

## UI (PR B)

- `web/local-console/ProviderSetup.tsx`: `ProviderChecklist` (one component for onboarding, Settings > Providers, doctor link), `CopySnippet`, `ProviderSetupPanel`.
- `web/local-console/Onboarding.tsx`: Providers -> Set up (per provider, WSL "Run in") -> Phone (pairing polled from `/api/v1/remote/devices`, notification permission) -> Done. Replaces the four-step tour; old dismiss flag counts as finished.
- Primary buttons in onboarding use `--sp-cta-bg` (red stays for danger, owner decision).
- Screenshots: `assets/onboarding/{welcome,checklist,settings}-{light,dark}-{1440,390}.png`.
- Verification: web typecheck, lint, 140 unit tests, build, full Playwright (105 incl. 9 new onboarding specs with a mocked status endpoint), desktop build and 41 tests.
