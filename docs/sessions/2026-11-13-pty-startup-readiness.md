# 2026-11-13 - PTY startup readiness for Claude Code and Gemini CLI

## Goals

Close gap G2 (and G3 for matching) from `docs/research/2026-11-13-dialpad-cao-comparison.md`. A prompt sent from the console or phone must never be pasted into a Claude Code or Gemini CLI startup or trust dialog, where the following Enter could accept workspace trust.

## Outcomes

- `host_input_handler` now gates pasted prompts and `/model` for `claude`, `claude-code`, `gemini` and `gemini-cli` as well as agy and Codex. A held prompt returns 409 (`NotReady`), the existing "needs setup in the terminal" path; no keystrokes are sent.
- Per-agent blocker and ready markers live in `startup_needs_setup`. Claude Code and Gemini CLI also require a ready marker while the startup frames are still in the 256 KiB ring.
- Matching reads `screen_text`: text after the last clear-screen or alternate-screen switch, at most 8192 characters, OSC removed, cursor-forward turned into a space and other cursor moves into a newline. This applies to all agents, including the Codex `? for shortcuts` fallback.
- Fixtures: `fixtures/claude-code/v2.1.322/trust-dialog.pty` and `ready-composer.pty`, captured by launching Claude Code through ConPTY and killing it without sending input. Paths, user name and OS temp directory were replaced by hand because `anonymize-fixture.mjs` handles JSON only.

## Problems and root causes

- Claude Code paints with `ESC[1C` cursor-forward between words. The old CSI stripper removed it and glued words together ("Yes,Itrustthisfolder"), so a naive port of the agy string match would never fire.
- In v2.1.322 the trust dialog selects "No, exit" by default and the ready composer shows no `? for shortcuts` footer in auto mode, so the ready marker is the composer box (a rule of at least 10 box-drawing dashes followed by `>`), with `? for shortcuts` as a second marker.
- Claude Code enters the alternate screen after the trust dialog and leaves it for views such as the transcript; treating `?1049l` as a clear would hide the composer, so only `?1049h`, `2J`, `3J` and `ESC c` cut.

## Decisions

- No terminal-emulator crate (ponytail). `screen_text` is a flat approximation; the shortcut and upgrade trigger are in its doc comment. Full screen model stays with research item PR 5.
- Ready is required only while the ring has not wrapped (`bytes.len() < SCROLLBACK_BYTES`); after wrap the startup frames are gone and only blockers are judged, so a long session cannot lock Remote prompts out.
- G14 (resend Enter when the submitted prompt stays in the composer) is skipped: it needs reliable composer-content detection and a wrong resend could submit a draft twice.

## Verification

- `cargo test -p gnomcode-pty`: 24 passed, including per-agent hold, release, stale text above a ready prompt, ANSI-split words, wrapped ring and handler-level 409 with nothing sent to the PTY.
- Claude Code strings: verified live on v2.1.322 (Windows). Gemini CLI is not installed here; its strings come from the `google-gemini/gemini-cli` source (`FolderTrustDialog.tsx`, `AuthDialog.tsx`, `InputPrompt.tsx`) and are unverified live.
- Workspace gate: see PR.
