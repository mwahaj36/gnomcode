# Codex hook review and browser folder picker

Date: 2026-11-05

## Goal and outcome

Prevent chat prompts from operating Codex's startup hook-review screen and restore workspace folder selection in the browser-hosted Windows console.

## Findings and decisions

The Windows installer opens the local console in an Edge app window. The folder button was hidden unless Electron exposed its native picker. The local console now reuses the bounded directory-only Remote browser through an authenticated loopback endpoint. Electron continues to use its native picker. No file contents are read.

The Codex CLI received a long pasted prompt while its hook-review menu was active. The PTY write acknowledgement did not mean the CLI had accepted a turn, and the prompt was interpreted as menu input. Bracketed chat paste now returns a conflict while the hook review is present, leaving the draft for retry after the user resolves the dialog in Terminal. Detection depends on the observed CLI text and may need updating if Codex changes that screen.

## Verification

- Added failing-first tests for blocking Codex hook review and authenticated workspace folder listing; focused tests passed after implementation.
- Full Rust quality gate passed: formatting, workspace clippy/tests, cargo-deny and core purity.
- Browser launcher regression checked native and browser folder pickers, folder selection and responsive layout; inspected a picker screenshot.
- Built the Windows engine and installer; installer payload and option checks passed. Packaging initially failed attempting to overwrite the identical already-copied engine; rerunning the packager against the output copy generated the ZIP and checksums. The web production build passed with existing React hook warnings.
- No installed application was replaced and no live agent prompt or hook approval was submitted during verification. Non-Windows and physical-phone runtime acceptance remain unverified.
