# Mobile reconnect recovery and purpose-built controls

Date: 2026-10-30

## Goals

Remove the reconnect dead end and generic Actions menu. Review mobile conversation navigation, composer, model selection, account connection, saved resume, pairing and folder picking for touch, keyboard and constrained viewports.

## Findings and changes

An open WebSocket only proved relay reachability. With missing desktop replies, no handshake/response timeout advanced recovery; foreground recovery also ignored sockets still reporting OPEN. Reconnect cleared visible history and selection, leaving a full-screen waiting state whose only escape forgot the desktop.

Add an authenticated-reply watchdog for remembered connections (15 seconds, checked every three seconds), foreground/network-return probes, automatic fresh encrypted reconnect and a manual Try again action. Preserve selected conversation, in-memory chat and draft. Gate prompting on authenticated state, retain navigation and disable delivery during recovery; never replay a prompt automatically. The initial code-verification step is exempt from the watchdog so users have time to enter the code. Connection epochs and native socket ownership still reject stale async work. A history request before renewed authentication previously created a false Chat connection lost alert; only authenticated connections now produce that feedback.

Replace generic Actions with direct conversation navigation, new chat and a labeled desktop icon. Desktop/account controls and model selection use separate bottom sheets with explicit close controls. Model selection stays beside the composer. Add search and a close control to the conversation drawer. Use 16px mobile form text to avoid browser zoom and preserve 44px touch targets. Retain the bounded folder dialog, safe-area composer, saved-session resume and unavailable-limit states.

## Verification

Web tests (18 passed), TypeScript checking, production build and the full Rust quality gate passed. Composer geometry assertions confirm multiline input ends above the toolbar; Send remains within a 350px reconnect viewport, with the document fixed against keyboard-induced page scrolling. Encrypted browser acceptance through the relay injects dropped incoming frames while the browser socket stays open: recovery becomes available, the draft survives, navigation stays available, Try again restores the authenticated conversation. Existing pairing, saved trust reload, background reconnect, saved-session resume, model-change draft, folder-picker, 320px/390px widths at 350px/600px/844px keyboard-height viewports and dark/light 390px/1440px layout checks remain covered. Rendered mobile chat and bottom-sheet captures inspected. Physical phone/network handoff remains unverified; the exact trigger of the user's intermittent disconnect is not available from screenshots alone.

## Decisions

Keep navigation and in-memory conversation available during network recovery; only explicit Forget this desktop removes local device trust. An online laptop does not guarantee an uninterrupted phone-to-relay-to-desktop connection. Composer rows use border-box sizing and separate flex rows so textarea and Send cannot intersect. Lock document scrolling only while the conversation workspace is active, preserve/restore prior page position, and track visual viewport height and offset during keyboard resize/panning. Dialogs are bounded to that visible viewport; history, drawers and sheets retain their own scrolling. No backend protocol, credentials or cloud persistence changed. SPEC and DESIGN updated.
