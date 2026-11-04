# Desktop onboarding — 2026-11-04

## Goal and outcome

Add professional first-use guidance to the local desktop console, ending at the real session launcher. Four concise steps explain prerequisites, workspace selection, conversation controls, usage trust and optional Remote. Back, Skip, Escape, progress and Profile replay are available. Dismissal uses a local versioned preference and tolerates blocked storage. No account, provider process or configuration change is triggered by the guide.

## Decisions and root causes

The console previously opened directly into a dashboard without setup guidance. Reuse the existing dialog, button and theme conventions, native installed-agent guidance and session launcher rather than introduce a separate setup system or duplicate provider detection. Native dialog behavior contains focus. Narrow browser captures exposed scrolling that hid controls; a scrolling content region now keeps the header and actions visible. The workspace test also moves focus out of the expanding navigation rail before selecting a conversation. Agent authentication and approvals stay in the official CLI. Existing Explorer folder links remain intact.

## Verification

- `node scripts/check-onboarding.mjs`: first-run behavior, progression, Back, skip/Escape, launcher handoff and blocked storage; observed failure before implementation.
- `node scripts/check-local-console.mjs` and `node scripts/check-console.mjs`: markup targets and JavaScript compilation.
- Expanded `scripts/check-agent-ide.mjs`: browser onboarding, persistent dismissal, Profile replay, both narrow themes and existing workspace regression coverage.
- Full Rust quality gate passed: formatting, Clippy, workspace tests, cargo-deny and core purity. Existing duplicate-dependency warnings remain. Native navigation and shared console presentation checks passed.
- Cross-platform review: browser-native UI and local preference only; no OS-specific path, process, integration or permission behavior added. Physical macOS/Linux desktop acceptance remains untested.
