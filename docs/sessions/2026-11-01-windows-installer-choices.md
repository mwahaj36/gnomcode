# Windows installer choices and Explorer launcher

Date: 2026-11-01

## Goals

Build a fresh installer on the user's Desktop with an installation folder picker, Start menu and desktop shortcuts, optional Windows sign-in launch, and Explorer folder integration.

## Changes and decisions

Setup previously began on a timer without an options step. It now waits for Install, shows independent shortcut/startup/Explorer choices, defaults sign-in startup off, and remembers the registered installation location. Installation accepts an empty dedicated local folder or an existing Spawnt installation. Errors stop installation and return a failing exit status instead of silently reporting success.

Explorer folder and folder-background verbs invoke a quoted installed PowerShell launcher. It starts the local daemon and opens the session chooser with the selected folder prefilled through a URL fragment, without automatically choosing an agent. Paths and tokens are URL-encoded, the host remains loopback, and the folder fragment is consumed after use. Windows 11 uses Show more options; a packaged modern Explorer extension is outside this change.

Custom locations required fixing uninstall's assumption that every install lives under LocalAppData. It uses the installed uninstaller's directory, removes shipped payload files only, preserves unrelated files/subdirectories and local history, and removes Explorer entries. Tray/browser cleanup is scoped to the chosen installation. No provider configuration or permission flags change. Existing download artifacts are backed up while packaging and restored after the fresh installer is copied to Desktop; no binary is committed.

## Verification

The full Rust quality gate passed (format, clippy, workspace/unit/golden/doc tests, cargo-deny and core dependency purity). The optimized engine build and C# installer compilation passed. The final Desktop installer passed version, byte-for-byte embedded engine/notices and launcher-resource verification, setup/options/isolated registry checks, and payload cleanup checks preserving unrelated files/subdirectories. The production web build and native console/navigation/PowerShell URL checks passed. Tests first reproduced automatic installation before choice, the absent Explorer-to-launcher handoff and the missing separately testable non-recursive cleanup.

Inspected the rendered native setup window and Explorer workspace chooser. A headless Edge check using the actual console source with stubbed API responses confirmed the exact folder (including spaces, ampersand, apostrophe and fragment characters), preserved authorization and consumed folder fragment without automatically launching an agent. Registry checks use a unique disposable test subtree rather than the user's actual shell keys. Cross-platform review: this is Windows packaging/shell integration; existing macOS/Linux launchers are unchanged, and Windows paths use platform folder APIs and quoted literal paths. No integration into agent-owned configuration is added.

Copied the verified installer to `Desktop/Spawnt-Setup-2026-11-01.exe` and verified its SHA-256 against the compiled artifact. The aggregate packaging command failed after successful installer verification while overwriting the pre-existing release ZIP (Python OSError 22); a non-truncating write-open probe succeeded, so the precise filesystem trigger was not established. The requested installer is independently verified; no new ZIP/public release is claimed. The standalone web build passed afterward. Original download artifacts were restored from backup. Physical install/uninstall on the user's active installation and a real Explorer click remain untested.
