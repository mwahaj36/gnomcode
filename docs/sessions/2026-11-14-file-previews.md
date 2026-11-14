# 2026-11-14 - File and artifact previews

## Goals
- Previews in Files, Artifacts and chat for images, SVG, text, code, Markdown, PDF, JSON and CSV; name, size and download for binaries; capped previews for large files.
- Same behavior on desktop, web console/PWA and Remote over the E2EE relay.

## Outcomes
- New pure `core::file_preview` (type detection, 512 KiB text cap, 1 MiB inline cap, base64) with unit tests.
- `workspace_files::preview_workspace_file` replaces the UTF-8-only reader for Remote `repo_file`, and the local `GET /api/v1/repos/file` now uses the same confined reader, so it also rejects `.git` paths (it previously had its own looser guard).
- `renderFileViewer` renders each kind; `hydrateAttachmentThumbs` lazily loads chat attachment thumbnails (local API or E2EE Remote), cached per path.

## Problems and root causes
- Oversized or binary files used to fail with 413/422 and showed nothing. Now text is truncated and binaries show metadata instead.
- Two Rust readers had different guards. There is now one.

## Decisions
- SVG is never inlined as markup; `<img>` gives script and network isolation without a sanitizer dependency.
- PDF uses the browser viewer through a blob URL (no network, no embedded third-party viewer).
- 1 MiB inline cap keeps encrypted relay frames bounded.

## Verification
- `cargo test -p gnomcode-core file_preview`, daemon `workspace_files` tests (truncation, image data URL, over-cap withheld, binary, SVG never sent as text, `.git` and traversal rejected).
- `npm test` (`file-previews.test.mjs`), Playwright `e2e/file-previews.spec.ts` at 1280px and 390px.
