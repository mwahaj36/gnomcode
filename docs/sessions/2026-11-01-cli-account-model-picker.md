# CLI account model picker

Date: 2026-11-01

## Request and coordination

Replace typed model IDs with choices across signed-in coding-agent CLI providers. The user explicitly scoped this to Codex, Claude Code, Gemini CLI and Antigravity, and asked that another agent's uncommitted `/workspace/spawnt-ui-ux` work on `feat/ui-ux-polish` remain untouched. This work stays on `fix/remote-model-catalog`, in the Windows checkout. No other worktree/branch is edited. UI changes are limited to the model sheet, a separate presentation component, and local catalog wiring; global styling is preserved.

## Root causes and changes

Remote had only a free-text model input and no catalog command. The existing local model list only read Codex's cache, and Remote switches omitted the explicit CLI identity needed by aliases such as sonnet and by account defaults. Add a shared Rust account/catalog service, authenticated local endpoint and encrypted optional models command. Render choices grouped by provider, with refresh, explicit missing-source/account states, retry, and draft-preserving errors. Local model selection and `spawnt models` use the same grouped source. Existing model-cache API compatibility is retained.

Codex uses its native login status and fresh visible cache. Claude uses native auth status plus the official initialization model list, without user messages, session persistence, hooks or MCP tools. Gemini uses noncredential account/settings metadata or API/Vertex environment presence and its native JSON models-list command; it is labeled configured rather than live-authenticated. Native metadata is sanitized in Rust; no account email, credential file or raw process output is exposed or persisted. Probes run concurrently in isolated temporary directories, with bounded output and time, outside database locks. Reuse the already-installed tempfile crate as a runtime dependency for probe isolation.

Antigravity currently has no verified supported read-only model-catalog integration. The picker explicitly reports this instead of manufacturing a catalog. Missing PATH entries do not prove that a CLI is uninstalled. Catalogs are availability observations, with final access enforced by each CLI; no provider tokens, pricing registry, browser login or guessed subscription entitlement is used.

## Verification

Native alias/custom-model routing regressions failed before implementation. Focused Rust catalog/routing tests, TypeScript, 18 web tests, production build and native navigation checks pass so far. A real local `spawnt models` probe confirmed the signed-in Codex account and its fresh model choices, while other CLIs were not on this process PATH. The encrypted headless Edge model-picker check passes with controlled catalogs for Codex/Claude/Gemini, explicit missing Antigravity catalog, error/retry, explicit Claude switch identity and preserved draft. Rendered 390px model sheet inspected. Claude/Gemini live probes and physical phone acceptance remain unverified.

The broader encrypted browser suite initially failed its short reconnect-viewport Send geometry assertion both before and after this change. The test measured before the visual-viewport resize had settled. Waiting for the bounded Remote viewport height before making the unchanged geometry assertion resolved the race; the full encrypted pairing/reconnect/resume suite now passes. A focused models-only mode also exercises this feature independently. No layout/CSS changes were made.

The full Rust gate exposed an old CLI test tied to the previous Codex-only empty message; it now verifies unavailable diagnostics for all four CLIs and no invented choices with an isolated PATH. A subsequent positive regression found that native PTY switching rejected the documented opus[1m] alias. The alias test failed, then bracket characters were added to the existing safe model-character allowlist; whitespace, flags and control characters remain rejected. The full Rust gate passed. After merging PR #111, the production web build, typecheck, 18 tests and desktop browser interaction check passed again. The native Windows installer was rebuilt with the integrated console and copied to `C:\Users\Wahaj\Desktop\Spawnt-Setup-Latest.exe`; its SHA-256 matches the build output (`E2DBB153DA372306D6142088968980D73ACA39A496171FA768D694129189D425`). Payload/version/notices and installer-option checks pass on the desktop copy. This is a local installer refresh, not a published release or a repackaged Electron distribution.

## PR #111 integration

At the user's explicit request, reviewed PR #111 with ponytail ultra and merged its exact reviewed head. Resolved the model-sheet/test/session-index overlap on this branch, preserving dialog accessible names, embedded heading levels, drawer focus restoration, navigation fixes and both session records. Updated the native desktop browser fixture for the shared catalog endpoint and provider-qualified selection values. The other agent's branch and worktree were not edited. The review is recorded separately in [PR #111 review](2026-11-01-pr111-ponytail-review.md).

Final integrated verification: full Rust gate passed again, including Clippy/workspace tests/deny/core purity. The encrypted headless Edge suite passed against a temporary loopback TLS relay, including PR #111 drawer focus assertions, pairing, reconnection, history, account model selection and draft preservation. Only the local test copy added the browser certificate exception for the temporary self-signed relay; production code and certificate validation were unchanged. The local relay cannot establish public-relay or physical-phone reliability.

Cross-platform review: new process creation flags are isolated in platform; CLI execution uses existing executable resolution without shell interpolation. Probe work uses independent temporary directories, bounded reads/timeouts and owned child cleanup. Gemini metadata reads use shared-file handling. No agent config, credential file, permission flags or original history is modified. macOS/Linux live probes remain unverified on this Windows host.
