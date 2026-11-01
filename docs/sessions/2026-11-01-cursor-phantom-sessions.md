# Cursor phantom sessions

Date: 2026-11-01

## Problem and root cause

The user saw many unknown Cursor rows marked Idle and Just now while Cursor was closed. The scanner considered JSON below Cursor's storage/configuration roots, and the adapter emitted an idle session even when a file had no conversation. Session/message timestamps defaulted to the current time. A real Cursor argv settings file, anonymized through the adapter fixture tool, reproduces the false session.

## Changes and decisions

Require a supported conversation array and a recorded timestamp on user/assistant messages. Skip unsupported settings and untimed records, retain recorded minimum/maximum timestamps, and classify exported transcripts as ended history rather than evidence of a live process. Missing model names remain unknown rather than receiving a made-up GPT-4o price. Preserve source logs.

At store open, remove only legacy idle Cursor placeholders lacking title/model/PID/parent, recorded chat/usage/activity, conversation membership, resume queue/alert/relay references and child sessions. Keep actual histories and referenced sessions. This is idempotent data repair, with no schema version change or removal of history-bearing data.

## Verification

Regression tests failed on the old settings/session/timestamp behavior and old placeholder retention, then passed after the fixes. The golden settings fixture contains a redacted crash-reporter identifier, no personal paths or contents. Focused adapter/store tests and the full Rust quality gate passed. The [native-title follow-up](2026-11-01-native-chat-titles.md) records the refreshed verified Desktop installer containing this repair. No user database was manually edited.
