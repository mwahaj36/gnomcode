---
name: cross-platform-review
description: >-
  Checklist to review any code touching filesystems, paths, processes, IPC,
  window focus, system tray, notifications, or autostart across Windows, macOS,
  and Linux (X11 + Wayland + WSL). Run before merging changes in crates/platform,
  crates/ingest, crates/daemon, or apps/desktop.
---

# Cross-Platform Review Checklist

Run through every item below before finishing any change that touches OS behavior (`crates/platform`, `crates/ingest`, `crates/daemon`, or `apps/desktop`).

## 1. Paths & Filesystem (`crates/platform`, `crates/ingest`)
- [ ] Zero hardcoded `/` or `\` separators; uses `Path`/`PathBuf` and `platform::paths`.
- [ ] Respects env overrides: `GNOMCODE_HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`.
- [ ] **Windows shared reads:** log files opened with `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`; file handles closed immediately after reading to EOF.
- [ ] **File identity:** `(dev, inode)` on Unix; volume serial + 64-bit file index on Windows.
- [ ] **Line endings:** parsers accept both `\n` and `\r\n`.
- [ ] **Linux inotify:** handles `IN_Q_OVERFLOW` and `max_user_watches` exhaustion by falling back to the reconciler/poller and surfacing a warning in `gnomcode doctor`.
- [ ] **WSL:** Windows daemon discovers `\\wsl$\<distro>\home\*` roots and polls them (since `ReadDirectoryChangesW` does not cross 9P reliably).

## 2. Processes, IPC & Security (`crates/platform`, `crates/daemon`)
- [ ] IPC uses Named Pipes (`\\.\pipe\gnomcode-<user>`) on Windows and Unix Domain Sockets (`~/.gnomcode/ipc.sock`, mode `0600`) on macOS/Linux.
- [ ] `daemon.json` bearer token file sets `0600` on Unix and user-only ACL on Windows.
- [ ] HTTP/WS binds strictly to `127.0.0.1` and validates `Host` and `Origin`.
- [ ] Process signals: uses `SIGINT` on Unix; on Windows never calls `TerminateProcess` without explicit user confirmation, and never kills IDE host processes (`Code.exe`, `Cursor.exe`, `Antigravity.exe`).

## 3. Tray, Notifications & Window Focus (`crates/platform`, `apps/desktop`)
- [ ] **Linux tray (SNI/AppIndicator):** does not rely on left-click popover events (unsupported on AppIndicator); provides a context menu item ("Open Mission Control") and fallback mini-window.
- [ ] **Wayland window focus:** implements compositor-specific paths (KWin D-Bus, `swaymsg`, `hyprctl`) and degrades gracefully to a "Copy repo path" toast on unsupported compositors (vanilla GNOME Wayland).
- [ ] **`unsafe` blocks:** confined to `crates/platform` with `#![allow(unsafe_code)]` at module scope and a `// SAFETY:` comment on every block.
