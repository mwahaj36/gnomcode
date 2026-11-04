# Rebuild Windows installer and release artifacts

Date: 2026-11-04

## Goal and outcome

Following PR #135 (which resolved Codex and adapter subagent identity clobbering, added session recorded token counts to ongoing session cards, and bypassed context relay handoffs for auto-compacting IDE environments), the compiled Windows release distribution under `web/public/downloads` needed to be refreshed to match the updated source code.

## Actions taken

1. Recompiled release binary `gnomcode.exe` via `cargo build --locked -p gnomcode --release`.
2. Extracted and formatted complete third-party notices into `web/public/downloads/THIRD_PARTY_NOTICES.html`.
3. Recompiled `scripts/Setup.cs` using .NET Framework `csc.exe` (`/target:winexe /platform:x64 /optimize+`), embedding the updated `gnomcode.exe`, icons, scripts, and licenses into `web/public/downloads/Gnomcode-Setup.exe`.
4. Validated installer payload and options:
   - `scripts/check-installer-payload.cs`: verified embedded assembly version, binary hashes, and resource integrity.
   - `scripts/check-installer-options.cs`: verified explorer integration, UI options, and folder validation against legacy installs.
5. Packaged `Gnomcode-v0.9.0-windows-x64.zip` and generated updated SHA-256 manifests (`SHA256SUMS` and `SHA256SUMS-v0.9.0.txt`).
6. Built Next.js web application (`npm run build --prefix web`) with updated download assets.

## Verification

- `check-console.mjs`, `check-native-navigation.mjs`, and `check-tray.ps1` passed.
- `check-installer-payload.cs` and `check-installer-options.cs` passed.
- `check-open-folder.ps1` passed.
- `python scripts/package-artifacts.py` completed cleanly.
- Next.js static asset build completed with exit code 0.

```text
93fb8b81b170f7d0758d602744e53a7c6daf0dba0ea5f9ec10e911a83d35ac44  Gnomcode-Setup.exe
b14acfac14b2a9f9142c9870464aac3066d7ed0edbe91aed2b393ac5e7abf000  Gnomcode-v0.9.0-windows-x64.zip
63e8d3322012c73a696a49fe3d84685fc83ce54b4207e40f9a670a8e0c5a8fea  gnomcode.exe
```
