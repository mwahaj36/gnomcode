# Platform Support Matrix (Gnomcode 1.1)

This document records the exact verification status per operating system and architecture for Gnomcode 1.1. Marketing pages, download links, and release notes may only claim capabilities marked **Verified** below.

Last updated: 2026-11-26.

Current release evidence (v1.2.1): desktop/native packaging, packaged smoke and platform tests in run `45757249955`; Windows ARM64 and static-musl builds in `45757254066`; the Windows x64 installer, payload, updater and tray checks plus 204 console browser tests against the release binary ran locally. The npm win32-x64 tarballs were installed and smoke-tested locally; the all-target npm tarball smoke was last run for v1.1.0 (`44913532282`). These are CI/runtime checks, not physical desktop/provider acceptance. Older run numbers in the detailed sections below describe historical feature evidence.

The [v1.2.1 release](https://github.com/mwahaj36/gnomcode/releases/tag/v1.2.1) includes manual downloads with SHA-256 manifests. Windows automatic updates install only installers whose Ed25519 signature matches the key built into Gnomcode; OS code signing and notarization remain separate.

## Summary

| Platform | Daemon CLI | Desktop App Package | Live Runtime / Hardware Acceptance | Signing / Notarization |
| :--- | :--- | :--- | :--- | :--- |
| Windows x64 | Verified (CI + local) | Verified v1.2.1 setup and Electron payload | Partial (local smoke + console E2E; clean VM install/upgrade pending) | Unsigned (`NotSigned`) |
| Windows arm64 | Verified ARM64 CLI build and packed install | CLI only | Native Windows ARM64 CI smoke; interactive agent workflows pending | Unsigned |
| macOS arm64 (Apple Silicon) | Verified v1.2.1 CI tarball | Verified v1.2.1 CI `.zip` | Not verified on live device | Unsigned, not notarized |
| macOS x64 (Intel) | Verified v1.2.1 CI tarball | Verified v1.2.1 CI `.zip` | Not verified on live device | Unsigned, not notarized |
| Linux x64 (`ubuntu-22.04`) | Verified v1.2.1 CI tarball | Verified v1.2.1 CI `.deb` + `.zip` | Not verified on live desktop (X11/Wayland) | N/A |
| Linux arm64 | Verified v1.2.1 CI tarball | Not built | Not verified | N/A |
| Linux x64 musl | Verified static binary and Alpine npm smoke | CLI only | CI CLI checks; live desktop workflows pending | N/A |

## Windows (x64)

### Verified
- Rust workspace build, clippy (`-D warnings`), and unit/integration test suite (`cargo test --workspace`, CI run `43464307883` and local Windows runs).
- Automated smoke check (`scripts/smoke/run-packaged-smoke.mjs`) against `gnomcode.exe`: daemon startup on a free loopback port with isolated `GNOMCODE_HOME`, `/api/v1/doctor`, `/api/v1/providers/overview` (`antigravity`, `claude-code`, `codex`), `/api/v1/sessions`, console HTML `/`, and `formatTrayMenu` template generation.
- `apps/desktop` lint, typecheck, and 44 unit/Electron tests, plus Playwright local console E2E (`console e2e (windows-latest)` in run `43464307883`).
- Antigravity 2.0 app, Antigravity IDE 1.x, and `agy` CLI data/trajectory paths under `GEMINI_HOME` (default `~/.gemini`), plus Claude Code and Codex discovery (`docs/SPEC.md` §3.5).
- Process termination via `taskkill /F /T`.

### Known gaps
- Authenticode signing is not present (`NotSigned`); owner signing or explicit unsigned-beta approval is required before public 1.0 distribution (`docs/LAUNCH_CHECKLIST.md`).
- Clean install, upgrade, reboot persistence, and uninstall on a disposable Windows VM have not been executed (`docs/LAUNCH_CHECKLIST.md`).
- Win32 `SetThreadExecutionState` keep-awake is unit-tested via mock backend; real hardware sleep prevention is unverified.
- Windows ARM64 is CLI-only. Native packed-install/version/doctor/report checks pass in CI; the ARM64 desktop installer and full interactive-provider acceptance remain pending.

## macOS (arm64 and x64)

### Verified
- Release daemon binary compilation and tarball packaging (`native archive (aarch64-apple-darwin)` and `native archive (x86_64-apple-darwin)` in CI runs `43464307883` and `44131280761`).
- Desktop Electron app `.zip` packaging and CI `packaged smoke` (`macos-15` / arm64 and `macos-15-intel` / x64 in CI run `44131280761`).
- Rust workspace compilation, clippy, and unit tests on `macos-latest` (`clippy + test (macos-latest)` in CI run `43464307883`).

### Known gaps
- Code signing and Apple notarization are not configured; Gatekeeper will block unnotarized `.app` downloads on end-user Macs.
- Packaged `.app` launch, tray menu rendering, native notifications, `osascript` window focus, and `caffeinate -i -w` hardware sleep inhibition are unverified on a live macOS machine.
- Antigravity IDE 1.x and 2.0 app data/config directories on macOS (`~/Library/Application Support/Antigravity` vs `~/.gemini`) are unverified on-device (`docs/SPEC.md` §3.5).

## Linux (x64, `ubuntu-22.04` / `ubuntu-latest`)

### Verified
- Release daemon binary compilation and tarball packaging for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` in CI runs `43464307883` and `44131280761`.
- Desktop Electron app `.deb` and `.zip` packaging plus CI `packaged smoke` on `ubuntu-22.04` (`desktop package (ubuntu-22.04 / x64)` in CI run `44131280761`).
- Rust workspace compilation, clippy, and unit tests on `ubuntu-latest`, `apps/desktop` Electron tests under `xvfb-run`, and Playwright local console E2E (`console e2e (ubuntu-latest)`).

### Known gaps
- `.deb` installation/uninstallation and packaged Electron launch on a live Linux desktop (X11 and Wayland) are unverified.
- `systemd-inhibit` sleep inhibition, `wmctrl` window focus (not supported on pure Wayland compositors), desktop notifications, and system tray behavior across GNOME/KDE/Wayland are unverified on live Linux desktops.
- Antigravity IDE 1.x and 2.0 app data/config directories on Linux (`~/.config/Antigravity` vs `~/.gemini`) are unverified on-device (`docs/SPEC.md` §3.5).
- Linux `arm64` desktop Electron packaging and `linux-x64-musl` builds are not in the CI packaging matrix.

## WSL on Windows (ADR 0060)

### Planned (not yet verified)
- Discovery of running distros from `HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss`; stopped distros are never booted.
- Ingest of `\wsl.localhost\<distro>\home\<user>\` roots for `.claude`, `.codex` and `.gemini/antigravity*`, with slow-poll fallback when 9P delivers no events.
- Launch and resume through `wsl.exe -d <distro> --cd <linux path> -- <agent>`.
- Distro badge (`WSL: <distro>`) on sessions, provider sources and filters.

### Known gaps
- WSL 1 distros and distros without a `\wsl.localhost` share are unverified.
- The Claude hook inside a distro requires `gnomcode integrate claude` run there; without it, sessions still ingest (fail open).
