# Console files and project trust

Desktop and encrypted Remote use the same read-only file and artifact previews. Files browses the workspace as a folder outline. Download saves the available complete file; capped previews do not offer a partial file as a complete download. Expand preview opens a larger viewer; Escape returns to the panel.

Local document links in assistant replies open Artifacts. Documents referenced by the conversation can appear even when Git ignores them or has no pending change. This covers Claude Code, Codex and Antigravity. In addition to the workspace, referenced documents can be read from provider-owned plan/artifact output folders. Configuration and credential roots are not exposed. Reads remain on demand and are not added to metadata sync. HTML previews disable scripts and network requests.

On desktop, Trust this project remembers folder and tool approval for future sessions in that exact workspace, including after Gnomcode restarts. Permission and sandbox changes still ask individually. No permission bypass flags are enabled. Workspace lists trusted projects with a Revoke control. Revoking does not undo a tool call already approved. Remote approval remains approve-once.

Devices offers Remove desktop for old account listings. Confirming removes that listing and its associated browser connections, while preserving local sessions and files. A desktop still signed in can register again when it reconnects.

Remote attachment transfers retain the 15 MiB per-file limit. Send waits for saved-file acknowledgement; failures and timeouts require removing or reattaching the failed selection. Encrypted frames stay below the relay size ceiling.

Remote Agent terminal shows the live hosted agent terminal with retained scrollback and accepts direct keyboard input; arbitrary workspace shells remain local. Login links open on the current device, and one-time codes can be entered directly in the terminal. Desktop localhost OAuth callbacks may still require the desktop browser. No credentials or terminal output are added to cloud metadata or persisted by the Remote terminal feature.

Review Changes keeps its changed-file list when a combined diff exceeds the 1 MiB preview limit. Select a file to load its diff separately. An individual file over that limit shows a clear limit message; review the full diff in your editor. Git operations in this pane remain read-only.

The local and account consoles share navigation icons and responsive layout rules. Compact windows use labeled bottom navigation and a history drawer. Opening a conversation or New closes that drawer. Workspace notices sit below the page toolbar; tool panes use the remaining work area.
