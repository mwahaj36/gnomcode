# Console files and project trust

Desktop and encrypted Remote use the same read-only file and artifact previews. Files browses the workspace as a folder outline. Download saves the available complete file; capped previews do not offer a partial file as a complete download. Expand preview opens a larger viewer; Escape returns to the panel.

Local document links in assistant replies open Artifacts. Documents referenced by the conversation can appear even when Git ignores them or has no pending change. This covers Claude Code, Codex and Antigravity. In addition to the workspace, referenced documents can be read from provider-owned plan/artifact output folders. Configuration and credential roots are not exposed. Reads remain on demand and are not added to metadata sync. HTML previews disable scripts and network requests.

On desktop, Trust this project remembers folder and tool approval for future sessions in that exact workspace, including after Gnomcode restarts. Permission and sandbox changes still ask individually. No permission bypass flags are enabled. Workspace lists trusted projects with a Revoke control. Revoking does not undo a tool call already approved. Remote approval remains approve-once.

Devices offers Remove desktop for old account listings. Confirming removes that listing and its associated browser connections, while preserving local sessions and files. A desktop still signed in can register again when it reconnects.
