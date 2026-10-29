## 2026-10-28 — copyable command snippets

- Goal: make every executable command shown in the app easy to copy without selecting text manually.
- Outcome: added a shared accessible copy control to Windows/Linux install commands, verification and CLI commands in docs, and the download page. The existing standalone Daytona preview already has copy controls.
- Verification: Next.js production build and `git diff --check` passed. Desktop browser review confirmed the buttons and layout in docs/downloads, the accessibility tree names each copy action, and clicking a docs command displayed “Copied”. Narrow viewport and keyboard-only review remain pending.
