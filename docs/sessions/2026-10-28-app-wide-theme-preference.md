## 2026-10-28 — app-wide theme preference

- Goal: make System, Light, and Dark preferences apply consistently to the landing page, marketing/account routes, signed-in console, and local daemon console.
- Outcome: centralized web preference read/apply/cycle behavior, applied the saved choice before paint, exposed the same control in web app navigation and the auth header, kept the landing/footer controls in sync, and added an origin-local control to the daemon console. Replaced hard-coded dark console colors and native-control color-scheme overrides with shared theme tokens.
- Decision: retain separate browser-local preferences for the signed-in site origin and loopback console origin; no preference or agent data leaves its origin.
- Verification: `npm run build --prefix web`, embedded local-console JavaScript parsing, `git diff --check`, pill-source search, and full Rust quality gate passed; cargo-deny is not installed. Chromium visual review confirmed theme cycling on the landing, auth, signed-in console, and local console in light/dark/system modes. Narrow viewport review remains pending; native tray palette remains later rollout work.
