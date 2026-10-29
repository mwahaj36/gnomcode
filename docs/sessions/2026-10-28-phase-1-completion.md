## 2026-10-28 — Phase 1 completion

Phase 1 final pass completed and PR created.

**Electron parity**: tray tooltip now exposes `awaiting_approval_count` from the `/api/v1/sessions` daemon response alongside `ongoing_count`. A `desktop:bring-to-front` IPC handler lets the renderer request window focus without exposing the OS call to the renderer context. All 14 desktop tests pass. Tray fallback dialog shows on systems without system tray support. Daemon lifecycle reconnection and restart are implemented. Security: context isolation, sandbox, no Node integration, CSP, validated IPC, allowlisted external URLs. Native Electron packaging via Forge is prepared with ASAR, disabled fuses (RunAsNode, NodeOptions, NodeCLIInspect), Squirrel lifecycle. Actual packaging requires staged binaries and approved EULA/notices which are still absent.

**npm distribution**: seven platform manifests in `npm/platform/`, shim in `npm/spawnt/`, staging (`stage-release.cjs`), published-style smoke (`smoke-release.cjs`), and release-input checker (`check-release.cjs`) are all present. Shim tests pass on the local Windows platform. The three-OS GitHub Actions smoke workflow (`npm-smoke.yml`) is prepared but has not been dispatched; release legal inputs (approved EULA) are absent. The skipped-optional-dependency error message and recovery instructions are tested.

**Package-manager manifests**: pre-release stubs for Homebrew (`pkg/homebrew/spawnt.rb`), Scoop (`pkg/scoop/spawnt.json`), winget (`pkg/winget/spawnt.yaml`), and AUR (`pkg/aur/PKGBUILD`) are added under `pkg/`. Name availability verified as of 2026-10-28: spawnt is not present in Homebrew, Scoop, winget, or AUR. Do not submit until real release artifacts exist and the releases repo is public.

**Rename**: all tracked source files, manifests, scripts, and docs use the Spawnt identity. The old product name does not appear in any tracked file (`git grep` confirms zero results). Old binary artifacts are preserved in `target/retired-downloads/` (ignored). The cloud sync table is `spawnt_sync_state`; SQL migration must be applied in the owner's Supabase project before the next cloud sync.

**Legal**: draft EULA, privacy policy, and refund policy remain in `docs/legal/` with DRAFT markers. They are not copied into any release package. Approved versions are required before any release or package-manager submission.

**Quality gates (Windows, 2026-10-28)**:
- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass (all tests; 20 daemon, 21 core, 11 adapters, 2 CLI smoke, plus store/platform/ingest/tray)
- `cargo deny check`: pass (duplicate-version warnings only, not errors)
- Core purity: pass
- Desktop lint (`eslint src`): pass (0 errors)
- Desktop typecheck (`tsc`): pass
- Desktop tests: 14 pass
- Web typecheck (`tsc --noEmit`): pass
- Web tests: pass
- Web lint: pass with 2 pre-existing React hook warnings (no errors)
- npm shim tests: 4 pass

**Idle memory (Windows, development Electron)**: previously measured — Electron median private 243,851,264 bytes (~233 MB), daemon median private 4,698,112 bytes (~4.5 MB). These are development-mode Windows figures, not packaged-release measurements. macOS and Linux idle memory not measured; native packaging not yet produced.

**Cross-platform verification**: Rust tests confirmed on Windows (local) and WSL Linux x86_64. macOS hardware unavailable. Linux ARM64 cross-compile not exercised in this session. All platform-specific code paths in `crates/platform` have `#[cfg(target_os)]` guards.

**Remaining external steps** (collected at PR):
- Apply `spawnt_sync_state` SQL migration in Supabase dashboard.
- Dispatch the `npm-smoke.yml` workflow once approved EULA is added.
- Produce and sign release artifacts before enabling downloads on the site.
- Make `mwahaj36/spawnt-releases` public at launch.
- Submit Homebrew, Scoop, winget, AUR manifests after real artifacts exist.
