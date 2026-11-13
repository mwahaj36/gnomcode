# 2026-11-13 - Check scripts triage (roadmap E4 last item)

## Goals
- Finish folding `scripts/check-*.mjs` into real tests now that the console is React: nothing that reads console sources, nothing that depends on the deleted `console-app.js`, no script that was already failing.

## Folded or deleted
| Script | Outcome |
| :--- | :--- |
| `check-console-presentation.mjs` | `web/test/console-presentation.test.mjs` (same assertions, shared presenter fixtures) |
| `check-tour-density.mjs` | `web/test/console-tour-density.test.mjs` |
| `check-settings-saves.mjs` (failing) | `web/test/settings-saves.test.mjs`; the fake-hook indexes and the missing `../lib/notifications` and `AppLockGate` mocks were stale |
| `check-console.mjs`, `check-local-console.mjs`, `check-onboarding.mjs`, `check-app-controls.mjs` | deleted; `web/e2e/shell`, `tour`, `account`, `approvals`, `phone` specs cover the behavior in a browser |
| `check-native-navigation.mjs` (failing: called a helper that no longer exists) | deleted; `web/e2e/shell.spec.ts` (aliases, token kept in the launch address, folder deep link, unreachable and unauthorized daemon) |
| `check-cloud-removal.mjs` (failing: needs a manually installed PGlite and a historical commit blob, so it never ran in CI) | deleted; the retired-schema migration it exercised has shipped |
| `check-agent-ide.mjs` | deleted (it loaded the retired `/console-app.js` and legacy ids); the composer (Enter, Shift+Enter, IME, failed send keeps the draft), model handoff (one switch, failure restores the model and draft), launcher (not-installed hint, folder kept, launch body, folder chooser) and tour/account/rail flows are in `web/e2e/workspace-chat.spec.ts` and the other specs |

## Kept on purpose
These drive a deployed site, a relay or a release binary, not the local console, and are run by hand against those targets: `check-account-ux`, `check-mobile-console`, `check-downloads-and-404`, `check-interface-screens`, `check-landing-hero`, `check-marketing-seo` (hosted web console and marketing pages), `check-remote-companion`, `check-live-relay` (Remote relay), `check-hosted-terminal` (release binary PTY smoke), `check-release-versions` (release packaging), and the installer and tray checks. None reads `console-app.js` or `console.html`.

## Tests
- 6 new browser tests (85 in total, counting the dispatch run); the shared conversation helpers moved to `web/e2e/workspace-helpers.ts`.

## Verification
- web `npm ci`, `npm test`, typecheck, build, `npm run test:e2e`; no Rust change in this PR.
