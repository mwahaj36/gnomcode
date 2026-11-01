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

The full Rust gate exposed an old CLI test tied to the previous Codex-only empty message; it now verifies unavailable diagnostics for all four CLIs and no invented choices with an isolated PATH. A subsequent positive regression found that native PTY switching rejected the documented opus[1m] alias. The alias test failed, then bracket characters were added to the existing safe model-character allowlist; whitespace, flags and control characters remain rejected. The final gate and refreshed installer are pending.
