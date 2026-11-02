# Gnomcode Phase 0: audit and delivery plan

Date: 2026-10-28. This is a planning document, not a claim that the target has shipped. The owner requested a review after each phase and a separate confirmation before Remote.

## Existing product and gaps

- The workspace declares 1.5.3; archived and website artifacts also show 1.5.x. The supplied v0.8.0 baseline is inconsistent with the repository. Rolling version numbers back to 0.x may break installer and updater ordering; design and test the migration before issuing a new release.
- Core, adapters, ingestion, store, platform, daemon, CLI, dashboard, guardrails, resume, pricing and quota code exist. M5 cloud code and migrations exist; completion and behavior need a feature-by-feature acceptance audit. Do not treat milestone labels as proof of all behavior.
- The shipped tray is a Rust controller with Windows scripts and browser launchers, not an existing Tauri application. ADR 0006 and 0008 record this distinction. Measure actual current functionality on each OS before defining parity.
- Existing `web/` is Next.js, with an embedded daemon console; do not replace it merely to use Astro. Local and hosted consoles have different transport and trust boundaries.
- The owner reserved `gnomcode@0.0.1` and created the `@gnomcode` npm organization. All seven scoped platform names returned 404 or were private in a registry check on 2026-10-28. Draft package manifests and shim are implemented but are not yet releasable.
- `crates/platform/src/lib.rs` reads the Claude credentials file for plan detection. The proposed no-token-reading rule requires a replacement path. Audit the separately specified opt-in usage endpoint and all credentials/network paths before claiming compliance; do not silently remove functionality. Cloud snapshots and account flows are not equivalent to end-to-end encrypted Remote. Agree a migration before retiring them.
- Local licensing checks no longer gate local APIs or automation. License UI and code remain pending a focused cleanup. There are no customers; Remote and Teams are the only planned paid features.
- ADRs 0011–0015 record the transition decisions and supersede conflicting decisions while retaining historical context. Remote ADR remains a design proposal until the separate M9 approval and review.

## Phase 1: rename, Electron and distribution

Estimated 4–7 part-time weeks, excluding account approval, signing procurement and release delays. Scope and exit checks:

1. Inventory identifiers, integrations, account and license behavior. Use a fresh Gnomcode home and 0.9.0 release line; there are no customers and no automatic migration. Do not delete the owner’s existing local data. Test fresh install, failure and rollback on three OSes.
2. Rename Rust crates, binary, CLI, environment variables, home, IPC, metadata, UI, package/app IDs and legal documentation in one coordinated pass. Use only the `gnomcode` executable and `GNOMCODE_HOME`. Centralize name, domain and base URLs in a canonical generated/config source consumed by Rust and UI without moving business rules into TypeScript. Search tracked files, scripts and built artifacts for old domains; retain only documented compatibility redirects.
3. Use the renamed private source repository and the newly created private releases repository. Update remotes, workflows, release assets, updater URLs, installer scripts, issue templates and documentation. Check release repository visibility and rights. GitHub redirects old repository links, but not actions hosted at the old repository name; never recreate the old names.
4. Preserve existing dashboard and tray behavior. Electron main process owns windows, tray and OS notifications; a sandboxed renderer loads only packaged local UI. Use context isolation, no Node integration, strict CSP, no remote content or release devtools. A narrow typed preload exposes validated, allowlisted IPC. Main process discovers and authenticates to the separate singleton Rust daemon; Rust owns all product logic. Daemon continues when windows close or Electron crashes; CLI may start/reuse it independently. Specify shutdown ownership and lock recovery, without terminating agent processes.
5. Build platform installers and updater, verifying signatures and manifests before replacement, atomic staging, rollback and release channel. Use OS signing and macOS notarization where credentials exist; explicitly disclose unsigned builds until available. Verify updater strategy against Electron packaging documentation before choosing a library. Test tray, notifications, focus and dashboard on native Windows, macOS, X11 and Wayland, documenting fallbacks.
6. Publish `gnomcode` with a bin shim and optional platform packages: `@gnomcode/cli-win32-x64`, `cli-win32-arm64`, `cli-darwin-x64`, `cli-darwin-arm64`, `cli-linux-x64`, `cli-linux-arm64`, `cli-linux-x64-musl`. Each binary package includes EULA and third-party notices; ship binaries only. Detect libc reliably, explain unsupported platforms and skipped optional dependencies, and provide a documented installer fallback. Build and test release tarballs in clean machines for `--version`, `doctor`, and `report`, including offline and `--no-optional`. Do not publish until binaries and legal documents exist.
7. Do not publish a deprecated compatibility package; the owner requested a sole product identity. npm direct-publish granular tokens are being phased out in January 2027; prefer trusted publishing where supported or a stage-only token and manual approval. If a token is necessary, store it as `NPM_TOKEN` in GitHub Actions secrets, never in files or chat. Provenance is unsupported for private source repositories: do not claim or enable it.
8. Reuse or adapt the existing site for static deployment after reviewing its account features and redirects. Host installers, policies, notices, documentation and a Remote/Teams waitlist at the new site; artifacts and `latest.json` live on GitHub Releases. Verify `gnomcode.vercel.app` availability in the owner's Vercel account. Vercel Hobby is personal/non-commercial only; choose a compliant plan or alternative for a commercial project. Until an owned domain exists, use a verified sender domain from an existing provider or the payment provider's hosted receipts; do not promise mail from `vercel.app`. Define waitlist consent, storage and deletion before enabling collection. Configure old-site redirects only with access to its host.
9. Check availability and ownership for Homebrew formula/cask, Scoop, winget and AUR before submitting; a search result is not a reservation. Prepare manifests against verified release URLs and checksums, and follow each upstream submission policy. Trademark risk exists around “Spawn” in software; request professional clearance, not a legal conclusion.
10. Measure idle memory on native Windows, macOS and Linux: fresh release builds, same hardware and workload, start daemon plus tray, close dashboard, wait five minutes, then sample five times at one-minute intervals. Record OS, architecture, build, process count, median and peak private memory and RSS/working set per process, and aggregate totals. Repeat with dashboard open and compare current baseline and Electron. Record measurement limitations and CPU; do not invent a target or numbers.

## M6–M9 implementation estimates

Estimates are uncertain, part-time, and assume existing working code is reused; platform testing and external review may extend them.

| Phase | Work and exit criteria | Estimate |
| --- | --- | --- |
| 2 / M6 | Launch, stop and focus official Claude, Codex and Gemini clients in repos or worktrees; quota-aware picker; Rust PTY and xterm; CLI launch and console. Exercise resize, Unicode and all platforms. Spike PTY ownership, crash survival and reconnection first: ordinary PTY closure can terminate a child. | 3–5 weeks |
| 3 / M7 | New agents crate with ACP-first verified drivers and terminal fallback; canonical ephemeral events; permission and tool cards; bounded, symlink-safe file viewer; git diff, revert, stage and explicit commit. Gnomcode-owned local conversation history for model switching, with retention, deletion and access controls designed before implementation; adapter fixtures and permission tests. | 5–8 weeks |
| 4 / M8 | Same-family step-down, explicit cross-vendor handoff file, console resume and visible dividers; ask-first default, no permission escalation. User-approved handoff content on disk conflicts with the broad content-persistence rule: define its explicit exception, access control, lifetime and deletion before implementation. | 3–5 weeks |
| 5 / M9 | Do not start without separate owner confirmation. Threat model, pairing, revocation, replay protection and ciphertext-only relay, phone client and subscription entitlement; independent security review before launch. Define Teams metadata exception or E2EE design before any network claims. | 8–12 weeks plus independent review |

Do a driver spike before selecting versions or flags. Codex documents app-server for persistent sessions and approvals; Claude documents print/stream modes. Neither fact proves an ACP adapter supports the user's subscription and permission requirements. Verify Gemini's current official ACP mode, Claude's route and every adapter version and license, plus vendor terms. Use terminal fallback when unverified. No product reads or forwards subscription tokens.

## Verification and review

Each implementation phase runs `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, plus TypeScript lint, typecheck and tests. Fix the existing interactive Next lint setup before using it as a gate. Native three-OS builds, adapter golden fixtures, daemon/agent crash isolation, security boundaries and release smoke tests apply to their respective changes. Phase 0 changes documentation only; no feature code or release was authorized in this phase. Stop for review after each phase.

## References

- npm trusted publishing and private-repository provenance: https://docs.npmjs.com/trusted-publishers/
- npm token transition: https://docs.npmjs.com/about-access-tokens/
- npm scoped public packages: https://docs.npmjs.com/creating-and-publishing-scoped-public-packages/
- GitHub repository rename: https://docs.github.com/en/repositories/creating-and-managing-repositories/renaming-a-repository
- Vercel fair use: https://vercel.com/docs/limits/fair-use-guidelines
- Codex integration choices: https://developers.openai.com/blog/codex-as-a-platform
- Claude CLI: https://docs.anthropic.com/en/docs/claude-code/cli-usage
