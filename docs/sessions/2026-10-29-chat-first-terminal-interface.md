## 2026-10-29 - Chat-first terminal interface

- Goal: make the hosted agent feel like a conversation while its official CLI continues running in the independent terminal owner.
- Root causes: the shared renderer escaped Markdown as plain text, both roles used full-width cards, and successful agent launch unconditionally switched to the raw terminal.
- Added right-aligned user bubbles, left-aligned agent replies, readable typography and safe formatting for headings, emphasis, code, nested lists, quotes, links and tables. User messages stay literal; HTML and embedded media remain inert.
- Agent launches stay in chat; shell launches retain the terminal. Enter submits, Shift+Enter adds a newline and IME composition is respected. Failed delivery preserves the draft. Session changes cannot let an old send clear the current draft. Unowned-session delivery errors explain the need for a Spawnt-hosted session.
- Conversation updates follow the bottom only when already near it; loading older history preserves the reader's position. Existing authenticated prompt delivery, native-agent history ingestion, permissions and local redaction are reused.
- Verification: all seven web tests, TypeScript checking, JavaScript syntax checking and the full Rust quality gate pass. Cargo deny reports existing duplicate-version warnings but passes advisories, bans, licenses and sources. Browser acceptance uses installed Edge because bundled Playwright Chromium is absent; verifies role alignment, rendered Markdown, keyboard/multiline submission, failed-send draft retention, launcher defaults, model handoff, scroll behavior and no page overflow at 1440/390 px. Rendered desktop and phone captures were inspected.
- Cross-platform review: these are browser presentation changes; no paths, process ownership, IPC, permission flags or focus implementation changes. No new dependency. Live provider-response validation and installing a rebuilt desktop app are not claimed by this change.

