# Console activity clarity

## Goal

Investigate misleading running sessions and workspace usability reports.

## Findings and changes

- Session presence is inferred from recorded events. An unmatched execution can remain classified as running for 24 hours without process evidence. The ongoing list now describes this uncertainty, labels execution and approval states as possible, and identifies the count as inferred. Unknown titles and projects use readable fallbacks.
- The native workspace reopen control was aligned to the right of the header, away from the conversation pane it controls. It now sits at the left.
- Remote reconnection briefly set the initial connecting phase during device discovery, hiding the existing workspace and draft until that lookup completed. Preserve the reconnecting phase when the device has a saved proof key.
- Signed-out native users now have a labeled dashboard action and a direct navigation entry to account sign-in. Account status is checked at startup, while account access consent remains unchanged.
- During reconnect on very short viewports, hide the unavailable automation controls to leave room for the composer.

## Verification

- `npm test` in `web`: 21 tests passed, including the shared presentation checks.
- `node scripts/check-native-navigation.mjs`: passed.
- Installed Playwright in ignored local test dependencies. `node scripts/check-agent-ide.mjs`: passed, including direct sign-in and the left-side conversation control; inspected the 1440px workspace screenshot.
- `node scripts/check-remote-companion.mjs` with a local synthetic relay: passed the complete encrypted flow, including delayed discovery, stalled socket recovery, a preserved draft and the short-screen composer. The delayed discovery regression failed before the fix. Real-device connectivity and the underlying cause of any particular network drop remain unverified.
- `npm run typecheck` in `web`: passed. The Windows Rust quality gate passed, including workspace tests and cargo-deny; cargo-deny reported duplicate dependency warnings.
