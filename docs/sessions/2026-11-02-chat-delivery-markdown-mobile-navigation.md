# Chat delivery, Markdown and mobile navigation

## Goals and findings

Fix repeated mobile navigation disconnects, misleading pending responses, imported image metadata and agent Markdown. Keep the production URL configuration unchanged and leave the PR unmerged for Wahaj.

The browser could restore the first cached device credential instead of the selected desktop. Native history confirmed the reported Antigravity follow-up was not recorded; transport acknowledgment alone could leave an endless response spinner. Chat redaction flattened whitespace before Markdown rendering, and imported attachment wrappers were displayed verbatim.

## Changes and decisions

- Bookmark successful desktop selection; discover through the account directory when no bookmark exists. Remove arbitrary cached-device selection.
- Keep unmatched prompts in chronological order and visibly mark them unconfirmed after fifteen seconds or a newer native turn. Never resend automatically. Reject programmatic Antigravity paste while workspace trust awaits local confirmation.
- Preserve Markdown whitespace and redact secrets even inside formatting. Re-redact legacy reads. Use pinned markdown-it 15.0.3 for native and web presentation, including tables and disabled task checkboxes; escape HTML and keep remote images inert. Its browser bundle and dependency license notices ship with the installer.
- Strip native image transport wrappers while preserving the request and an explicit unavailable-preview label. Imported image previews remain unsupported; no arbitrary local paths are opened.
- Automatically repair existing local chat formatting once from available native sources. Maintenance marker 11 uses existing migration bookkeeping; no cloud migration is needed. Update only existing matching content, preserving deletion, retention, IDs, cursors, usage and checkpoints. Missing originals cannot recover lost formatting.
- Add a synthetic Codex image-wrapper fixture, processed through the repository anonymizer with authored transport markers retained for reproduction. No personal logs are committed.

## Verification

Full Rust quality gate passed: formatting, workspace clippy/tests, dependency policies and core purity. Focused attachment fixture, legacy repair and startup-trust tests passed. Web tests (20), production build, pinned vendor check and production dependency audit (zero findings) passed. Native chat browser checks verified Markdown at desktop and phone widths. Public-relay phone-viewport automation passed repeated Spawnt navigation and browser Back/Forward with stale cached credentials, two desktops, trust reload and bounded reconnect recovery.

Physical-phone acceptance remains open. No further paid model calls or injections into the user's active conversation were made. This phase does not change production deployment or URL configuration. The review installer is a separate development preview; the existing desktop installer remains untouched.
