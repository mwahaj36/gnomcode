# 0016 — PTY launcher and independent session owner

- **Status:** Accepted
- **Date:** 2026-10-28

## Context

An agent connected to a pseudoterminal must survive the daemon and desktop exiting. A terminal whose master is owned solely by the daemon cannot provide that guarantee: closing the master can end the terminal session. Reconnection must not expose terminal input or output to other local users, and terminal output must not be written to the metadata database or cloud sync.

## Direction

A separate session-owner process (`spawnt pty-host`) holds the pseudoterminal master and child process for the life of the session. The daemon and desktop are clients; their exit does not close the master. Session owners authenticate local clients via bearer tokens and keep output in a bounded in-memory buffer (`Scrollback`). Discovery metadata and authentication material are stored in private, owner-only session files under `$SPAWNT_HOME/sessions/`. Only a user-requested stop terminates the agent process tree. The owner does not read subscription credentials or add permission-bypass flags (`--dangerously-skip-permissions`, `-yolo`).

Windows uses ConPTY through `conpty-oxide`; macOS and Linux use POSIX pseudoterminal through `portable-pty`. The daemon exposes authenticated loopback endpoints (`POST /api/v1/sessions/launch`, `POST /api/v1/sessions/{id}/stop`, `GET /api/v1/sessions/{id}/pty`, `GET /api/v1/models`) and proxies terminal WebSocket streams bidirectionally to the session owner.

## Consequences

Session owners add a detached background worker process per session. Process lifecycle tests verify that the agent survives client disconnects and daemon stops. Terminal scrollback is bounded in memory (default 256 KB) and is not persisted to disk or cloud metadata.

## Alternatives considered

- Daemon-owned PTY: closing the daemon risks terminating the agent and loses the terminal endpoint.
- Terminal owned by Electron: closing the UI would interrupt the agent.

## Prompt delivery follow-up (2026-10-29)

Prompt submission and model handoff use a bounded, authenticated loopback `POST /input` on the PTY owner. The owner acknowledges queuing the input independently of terminal output; a short-lived streaming WebSocket could previously disconnect before its input was consumed. Interactive terminals continue to use the bidirectional WebSocket. Existing PTY owners retain their running binary, so interactive acceptance tests must start a fresh hosted session after upgrading.
