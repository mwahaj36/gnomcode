# Quick prompts UI on local console and Remote

## Goals

Add one-tap canned quick prompts (`Continue`, `Summarize changes`, `Run tests`, `Explain error`) and a local customization sheet on both the local console Workspace and Remote phone companion, using only the existing prompt delivery path and never executing raw argv or shell commands.

## Outcomes

- Added shared quick-prompt normalization, merging, browser storage helpers, and strip rendering in `web/lib/console-presentation.js` and `web/lib/console-presentation.d.ts`.
- Wired `#chat-quick-actions` and `#quick-actions-dialog` into `crates/daemon/src/console.html` and `crates/daemon/src/console-app.js` using shared `sendPromptText` delivery to `POST /api/v1/sessions/{id}/prompt` while preserving any unsent composer draft.
- Wired `.sp-quick-actions` and the `Quick prompts` bottom sheet into `web/components/RemoteCompanion.tsx`, sharing `submitPromptText` delivery to `{ type: 'prompt', session_id, text }`.
- Styled `.sp-quick-actions` and `.sp-dialog` in `web/app/gnomcode-console.css` with `--sp-*` tokens, 44px mobile touch targets, 16px mobile inputs, and hiding on `max-height: 450px` so short keyboard viewports keep `Send` visible.

## Problems and root causes

- **Mobile quick-prompt pill specificity:** Generic button rules in `gnomcode-console.css` overrode `.sp-quick-action`, and `#chat-quick-actions` did not match `#chat-prompt-form`'s `900px` centered alignment. Fixed by qualifying `.sp-native-console .sp-quick-action,.sp-remote .sp-quick-action` and matching `#chat-quick-actions` width/alignment to `#chat-prompt-form`.
- **Safe Remote compatibility without Worker A backend:** Sending an unknown Remote request like `quick_prompts` to an un-updated daemon returns `{ type: 'error', message: 'Unsupported remote command' }`. Avoided speculative Remote RPC calls; quick prompts persist locally and dispatch solely via `{ type: 'prompt', session_id, text }`.

## Decisions

- Enforced prompt validation matching `remote.rs:803-809` (`trim`, max 4000 chars, no leading `/`, no disallowed control chars) and stripped any `argv`/`command`/`shell`/`args` keys in `normalizeQuickAction`.
- Sending a quick action preserves any unsent draft in the composer textarea.

## Verification

- `node web/test/quick-actions.test.mjs` (red -> green)
- `node scripts/check-local-console.mjs` (pass)
- `cd web && npm ci && npm test && npm run typecheck && npm run build` (104/104 tests pass, typecheck pass, Next build pass)
- Headless Chromium (`chrome-headless-shell.exe`) visual + bounding-box check on local console at `390x844`, `1440x900` (light & dark), and `390x350` keyboard viewport.
- Full Rust workspace gate run with `CARGO_INCREMENTAL=0` (`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`).
