# Cloud session collectors

FleetHUD can register a session from inside its cloud workspace on Windows,
macOS or Linux. This is an opt-in collector, not account-wide discovery of every
hosted provider session. The collector never changes agent permissions.

## Automatic startup

Starting the FleetHUD daemon or tray backend automatically starts a cloud
collector when runtime identity is available. Codex cloud identity is detected
without extra flags. The daemon reuses its existing paired account; no separate
collector command or pairing file argument is needed in an already paired environment.
Run `fleethud pair --name "My cloud workspace"` once in a new environment.
Open its browser link, compare the verification code, sign in to the same account
as your desktop, and approve. Wait for `Account paired` in the terminal. Pairing
starts the daemon automatically. On the desktop use **Auth & Cloud Sync → Connect
Account** for the same flow. A persistent environment keeps its credentials;
an ephemeral environment must pair again if its FleetHUD home is discarded.

A new environment still requires one secure account link; repository ownership
and runtime IDs are not account authorization.

For other agents, the environment can bind `FLEETHUD_CLOUD_AGENT` (one of the
provider slugs below) and `FLEETHUD_CLOUD_SESSION_ID` to its actual native session
ID. The same automatic startup works on Windows, macOS and Linux. Do not set a
fixed ID in a shared environment snapshot: use the current runtime's real ID.

Use the standalone collector commands below only when the FleetHUD daemon is not
running, or when you need an explicit metadata usage file. Avoid running both for
the same session. Desktop sync and automatic collection serialize credential
refresh inside the daemon so they do not race over its rotating login session.

## Standalone collector

Build the CLI with stable Rust (minimum 1.87 for the patched `ctutils` dependency): `cargo build --locked -p fleethud`.
Use `target/debug/fleethud` on Unix or `target/debug/fleethud.exe` on Windows.
For an installed binary, the commands are identical on all three OSes:

```sh
fleethud cloud-session
fleethud cloud-session --agent claude-code --session-id PROVIDER_SESSION_ID
fleethud cloud-session --agent cursor --session-id PROVIDER_SESSION_ID
fleethud cloud-session --agent antigravity --session-id PROVIDER_SESSION_ID
fleethud cloud-session --agent gemini-cli --session-id PROVIDER_SESSION_ID
fleethud cloud-session --agent cline --session-id PROVIDER_SESSION_ID
fleethud cloud-session --agent aider --session-id PROVIDER_SESSION_ID
```

Only Codex cloud identity is automatically detected: a nonempty
`CODEX_ENVIRONMENT_ID` or `CODEX_CAAS_SESSION_ID` plus `CODEX_THREAD_ID` (fallback
`CODEX_SESSION_ID`). Other agents require their actual native session ID.
The collector must run inside the environment to observe it. Installations in
one workspace cannot discover inaccessible sessions on other provider machines.
Cloud workers must allow the collector process to run for continuous tracking.

The collector updates the existing SQLite database every 30 seconds.
`--once` saves one heartbeat and exits. `FLEETHUD_HOME` and the global `--db`
option work normally. A connected collector does not prove that the agent is
thinking or running tools. Presence expires after 90 seconds without a heartbeat;
these sessions are excluded from the active-agent filter.

## Measured usage and estimated cost

When native agent logs are accessible, the collector reuses FleetHUD's adapters
and prices usage for the selected native session ID. Other sessions in those
logs are not published by the collector. No prompts or raw transcripts are sent.
The local adapters retain their existing experimental/support limitations.

An integration can alternatively append metadata-only JSONL invocation deltas:

```sh
fleethud cloud-session --agent cursor --session-id PROVIDER_SESSION_ID --usage-file usage.jsonl
```

Each complete line has this schema (example counts are illustrative):

```json
{"event_id":"invocation-1","timestamp":"2026-10-27T00:00:00Z","model":"gpt-6.1-sol","input_tokens":1000,"output_tokens":200,"cache_read_tokens":300,"cache_write_5m_tokens":0,"cache_write_1h_tokens":0,"reasoning_tokens":0}
```

`event_id`, `timestamp`, `model`, `input_tokens`, and `output_tokens` are required.
Other counters default to zero. Input counts must exclude cache reads/writes.
Output counts include reasoning tokens; `reasoning_tokens` is a subset and is
never charged twice. Events are invocation deltas, never cumulative session totals.
IDs must remain stable across retries; separate invocations need distinct IDs.
Use one source per session: `--usage-file` disables native log ingestion for that
collector to avoid counting the same invocation through two independent sources.

Incomplete lines wait for the next pass. Stable event IDs prevent duplicate costs
across restarts, replay and truncation. Keep the source append-only; use a new
file path when replacing/rotating it. At most 8 MiB of pending input is accepted
per pass. Unknown fields (including prompt/output/content) and invalid counters
reject the whole batch; `--once` exits with an error. Long-running collection
keeps its previous measurements and retries without affecting the agent.

Token counters are supplied by the source. Pricing uses the existing Rust pricing
table and is labeled `estimated` API-equivalent USD, not a subscription invoice.
Missing usage and missing model prices display `Unknown`, not measured zero.
Cloud costs appear in session rows; the existing summary charts remain desktop
reports. This importer does not implement each provider's telemetry exporter.

## Pair the account and deploy

1. Apply `crates/store/migrations/supabase_cloud_presence_v3.sql` to the account's
   Supabase project (Postgres 15+). It is additive and can be applied repeatedly.
2. Deploy the updated web app. Old clients do not read the cloud-session table.
3. Apply `crates/store/migrations/supabase_device_pairing_v4.sql` to that same
   project. This enables expiring, single-use browser approvals.
4. Run `fleethud pair` in the cloud terminal. Open the link in a browser on your
   own computer. Compare the code, then sign in using GitHub or email/password
   and approve. The collector receives its own login session; no file or token
   copying is required. The terminal must remain open until pairing completes.
5. Open the desktop or web **All sessions** view. Allow up to 60 seconds. The
   Windows tray lists **Connected cloud sessions** separately from active local
   agents. A stopped collector becomes disconnected after 90 seconds.

Approval links expire after ten minutes; approved envelopes can be collected once
within five minutes. Credentials are encrypted in the browser with AES-256-GCM;
the encryption key is in the URL fragment and is not sent to the web server or
Supabase. The collector validates the decrypted login with the configured auth
server before saving it atomically with owner-only permissions. Approval uses a
separate browser login to avoid sharing rotating refresh tokens with your desktop.
Only approve links you started. Do not share pairing links, `session.json`, or
screenshots of credential files. Deleting a local pairing disconnects that device;
existing historical account data remains in your account.

Advanced existing integrations can still supply `--pairing-file PATH`. Do not
share one rotating session file across separate collectors. A cloud daemon
publishes cloud rows only; it does not replace the desktop summary snapshot.
The local console and tray fetch account cloud presence every 30 seconds. If
remote sync fails, local monitoring continues and the UI reports that status.
Summary charts remain desktop reports; cloud usage and costs are on session rows.

The default project URL/key match the existing FleetHUD client. Custom projects
use `FLEETHUD_SUPABASE_URL` and `FLEETHUD_SUPABASE_KEY` for the collector and the
corresponding `NEXT_PUBLIC_SUPABASE_*` values for the web build. HTTPS/TLS checks
remain enabled. Sync requests have a ten-second timeout and retry every 30 seconds.

Each account/session gets its own row, so simultaneous collectors and desktop
snapshots cannot overwrite one another. Row-level security isolates accounts;
server-controlled timestamps expire stale presence. Remote payloads omit titles,
repository paths and repository names. The web console refreshes every 15 seconds.

## Current provider evidence and limits

- Codex documents JSON events including `turn.completed` usage in
  https://developers.openai.com/codex/noninteractive . This describes CLI runs;
  it does not establish access to usage for an independently hosted chat.
- Gemini CLI documents session IDs, token metrics and OTLP export in
  https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/telemetry.md .
  An OTLP receiver/exporter-to-JSONL integration is not included in this change.
- Cursor documents CLI output formats at
  https://cursor.com/docs/cli/reference/output-format and cloud APIs at
  https://cursor.com/docs/cloud-agent/api/endpoints . Access to an endpoint does
  not establish availability of per-session model/token counters.
- Claude Code, Antigravity, Cline/Roo and Aider can use their existing native log
  adapters when present, or explicit identity plus a metadata exporter. Provider
  hooks/telemetry are not automatically installed or silently enabled.

In the managed Codex session inspected during development, identity variables
exist but no rollout log or supported usage feed is exposed. Its presence can be
recorded; its token usage and cost remain unknown. File timestamps, heartbeat
counts and runtime duration must never be converted into invented token costs.

## Development checks

Run the repository Rust quality gate and `npm run build` in `web`.
For the migration checks, install PGlite into a temporary directory (outside the
checkout), then run `node scripts/check-cloud-presence.mjs DIRECTORY` with that
npm prefix. The development run used `@electric-sql/pglite@0.5.12`. The script
creates disposable auth roles/users and checks repeated migration application,
account isolation, typed metadata, measured usage, server timestamps and expiry.
It never connects to production. Run
`node scripts/check-device-pairing.mjs DIRECTORY` to check approval permissions,
secret verification, one-time consumption and replay rejection. Run
`node scripts/check-console.mjs` before packaging. The browser integration check
is `node scripts/check-pairing-ui.mjs PLAYWRIGHT_PREFIX DAEMON_JSON WEB_ORIGIN`;
it exercises a real daemon and web build, stubbing only external auth transport. Native Windows/macOS checks run through the
manual CI matrix alongside Linux; Linux validation alone is not proof of native
execution on the other systems.
