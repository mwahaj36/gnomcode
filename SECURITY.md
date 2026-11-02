# Security Policy

## Reporting a Vulnerability

If you discover a security or privacy issue in Gnomcode (such as local API authentication bypass, DNS-rebinding exposure, or unredacted sensitive data in local storage), please report it privately rather than opening a public issue:

- **GitHub Private Vulnerability Reporting:** Use the **Report a vulnerability** button under the Security tab of this repository.
- **Email:** `security@gnomcode.vercel.app`

We respond to all reports within 48 hours.

## Security & Privacy Guarantees

Gnomcode enforces strict architectural security invariants (`docs/SPEC.md` §3.1, §3.11):

1. **Zero content persistence:** Prompts, source code, file contents, and raw tool outputs are never written to SQLite or sent over the network.
2. **Loopback-only API:** The local HTTP/WebSocket server binds strictly to `127.0.0.1` (never `0.0.0.0`), requires a per-install bearer token (`0600` / user-only ACL), and validates `Host` and `Origin` headers against DNS-rebinding attacks.
3. **Fail-open hooks:** Agent lifecycle hooks (`gnomcode hook`) enforce a strict 50 ms IPC timeout and fail open if the daemon is unreachable.
4. **No telemetry:** Outbound network calls are disabled by default (apart from the opt-out daily update manifest check). Run `gnomcode doctor --network` to inspect every configured endpoint.
