# 2026-11-13 - Marketing audit for 1.0 (E4)

## Goals
- Make every public page, metadata block and machine-readable file match 1.0 exactly: Claude Code, Codex and Antigravity only, no Teams, only features that exist, no unverified numbers.

## Outcomes
- Removed Gemini CLI from the landing agent marquee, FAQ data, landing FAQ, SEO JSON-LD, root metadata and keywords, manifest, `llms.txt`, `llms-full.txt`, docs index, getting started, supported agents (section and official-source links for Gemini CLI, Cline, Roo Code, Cursor and Aider), model switching and quota limits (Antigravity limits section replaces the Gemini CLI one).
- Removed Teams: landing and pricing plan cards (now two plans, grids set to two columns), pricing FAQ, terms, EULA text, landing and site FAQ. A new FAQ entry states that Gemini CLI, Cursor, Aider, Cline and Teams are not supported.
- Removed unverified claims and numbers: JSON-LD `$20` Pro offer, `softwareVersion` 0.9.0 (now the package version), JSON-LD iOS/Android/macOS/Linux operating systems (now Windows), "free forever", download-page `glibc 2.35+`, `Apple Silicon / Intel`, `ARM64`, `Windows 10 / 11`, docs `Linux x86_64 and ARM64, WSL2` dependency, "15 minutes" idle claim, "2-second" snapshot claim, "100% locally", "7-day exact rate limit headers", the downloads title naming macOS and Linux, and the claim that the Linux install script verifies downloads.
- Fixed claims that contradicted shipped behavior: Claude credentials are read only by the opt-in usage-limit source (docs index and quota limits page); imported history is read-only until Continue here; phone approvals for Claude Code and opt-in protocol mode for Codex and Antigravity; phone interrupt, changes, files, artifacts and attachments.
- Tour sample data now shows Antigravity instead of Gemini (still labeled illustrative). OG image and its generator and spec now list the three agents; the PNG was re-rendered (headless Edge, same template as `scripts/generate-social-image.mjs`).

## Decisions
- Kept `GEMINI_HOME`, `~/.gemini/antigravity` and Gemini pricing references because Antigravity uses them.
- Kept the hedged "prototype" and "pricing not finalized" language for Remote; no price is stated anywhere.
- Left `docs/marketing-copy-audit.md` untouched (historical ledger).

## Rendered review
- Landing, pricing, downloads and supported agents checked at 1440 dark and 390 light in the in-app browser: no horizontal overflow, two-plan grids lay out correctly, no Critical or High findings. Low, not fixed: the landing "Compare plans" link wraps its arrow onto a second line at narrow widths (pre-existing).

## Verification
- `npm ci`, `npm test` (133 pass), `npm run typecheck`, `npm run build` in `web/`. Meta descriptions checked at 150 characters or fewer; `<title>` values remain unique.
- `scripts/check-marketing-seo.mjs` and the other Playwright checks were not run (Playwright and its browsers are not installed here).
