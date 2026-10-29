## 2026-10-28 — v1.4 platform packaging

- Assumption: use minor version 1.4.0 for the redesign. Keep binary/package/data identifiers and published domain/download paths compatible.
- Added native Intel/Apple Silicon macOS and x86_64/ARM64 Linux packaging, canonical app/launcher icons, font notices and cross-platform executable archive modes. Unix upgrades retain user data and verify the replacement before atomic installation. macOS browser-console app is unsigned; native Unix trays and notarization remain future work under ADR 0007.
- Windows setup uses absolute compiler inputs, version-checked embedded payloads, device-theme colors and canonical assets. Restricted upgrade/uninstall process cleanup to the installed executable/profile/tray, replacing broad process-name kills. Shortcuts display spawnt while installation/registry identities remain compatible.
- Added release architecture/permission/manifest regression checks and native CI packaging/updater verification on four Unix targets. Windows/Linux release builds passed; macOS CI is blocked before execution by account billing.
