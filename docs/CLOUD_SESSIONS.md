# Cloud session collectors

FleetHUD can register a session from inside its cloud workspace on Windows,
macOS or Linux. This is an opt-in collector, not account-wide discovery of every
hosted provider session. The collector never changes agent permissions.

## Start a collector

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
3. Provision a valid Supabase login session JSON securely into the collector's
   environment. Reuse FleetHUD's existing paired `FLEETHUD_HOME/session.json`, or
   supply `--pairing-file PATH`. This file contains credentials: use user-only
   permissions/ACLs and never commit it, print it, or paste it into chat.
   It needs a `user.id`, access token, expiry and refresh token from the same
   Supabase project. Give the cloud collector its own login session rather than
   concurrently sharing a rotating refresh-token file with a desktop process.
4. Start the collector. A successful publish prints `Cloud presence synchronized.`
   Without pairing, local collection works and the remote dashboard is unchanged.

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
It never connects to production. Native Windows/macOS checks run through the
manual CI matrix alongside Linux; Linux validation alone is not proof of native
execution on the other systems.
