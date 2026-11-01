# Native chat titles

Date: 2026-11-01

## Goals and root cause

The user requested the names saved by source apps instead of attachment text appearing as a title. All main adapters discarded native title metadata, the ingest loop ignored saved title indexes, and the conversation-label helper preferred opening prompt text over stored titles.

## Changes and decisions

Prefer a bounded redacted native title, retaining the original native source across conversation/provider switches. Import Codex's observed id/thread_name session index, Claude's session index/custom-title metadata, and supported Gemini/Cursor root title/summary fields. Keep existing Antigravity title reconciliation. Refresh indexes independently of transcript checkpoints so app renames take effect without pretending to be new activity. Read only known metadata files, with an 8 MiB cap, platform shared reads and graceful missing-file fallback. Reuse the existing workspace obfstr dependency in ingest for adapter filename patterns; no new external crate is introduced.

Keep the first meaningful user request as fallback. Attachment headings/paths are excluded only from fallback labels; conversation text and native agent files remain unchanged. Cline/Roo/Aider have no newly verified dedicated saved-name source, so they retain fallback naming. No new cloud table/column or chat upload is added; existing title metadata remains in its existing sync field.

## Verification

Adapter and daemon regressions failed before implementation and passed afterward. Tests cover native labels, custom-title precedence, malformed index lines, redaction, rename refresh without checkpoint/activity changes, native-name preference, fallback attachment cleanup and stable conversation source after switching. A real Codex session-index record was anonymized through the fixture tool before adding its golden snapshot; temporary raw fixture copies were removed.

The full Rust quality gate passed (including 28 adapter, 52 daemon, 4 ingest and 16 store tests), as did the optimized release build, C# compilation, native console/navigation/tray checks, embedded payload/version/notices checks, installer options/isolated registry/cleanup checks and PowerShell folder URL checks. Copied the hash-verified aggregate installer, including Cursor repair and current mobile UI assets, to `Desktop/Spawnt-Setup-Latest.exe` and reran payload/options checks on that copy. Build outputs stayed in `.tmp`; existing public download binaries were untouched. No installation was executed by the agent. Native source title appearance in the user's running app requires this update; physical-phone testing remains unverified.

Cross-platform review confirms discovery-based roots, no new cfg-specific paths/process behavior, shared file reads, and no agent-owned config mutation.
