//! SQLite storage schema, migrations, queries, and rollups (`SPEC §3.6`, `§4.3`).

use chrono::{DateTime, Utc};
use fleethud_core::{
    ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
    SessionState, Surface, TrustLevel, UsageRecord,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use thiserror::Error;

/// Storage layer error type.
#[derive(Debug, Error)]
pub enum Error {
    /// SQLite driver failure.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Schema migration failure.
    #[error("migration error: {0}")]
    Migration(String),
}

/// Checked checkpoint for a source log file on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCheckpoint {
    /// Absolute or canonical path to the log file.
    pub file_path: String,
    /// Last byte offset read and committed into SQLite.
    pub last_read_offset: u64,
    /// Last observed file size on disk.
    pub file_size: u64,
    /// SHA-256 or mtime/identity fingerprint.
    pub fingerprint: String,
    /// UTC timestamp of the last successful ingest.
    pub last_ingested_at: DateTime<Utc>,
}

/// Aggregated report row grouped by project, agent, and model (`SPEC §3.6`).
#[derive(Debug, Clone, PartialEq)]
pub struct ReportRow {
    /// Logical project name.
    pub project_name: String,
    /// Agent identifier.
    pub agent: AgentId,
    /// Model identifier.
    pub model: String,
    /// Total uncached input tokens.
    pub input_tokens: u64,
    /// Total cached input tokens read.
    pub cache_read_tokens: u64,
    /// Total output tokens.
    pub output_tokens: u64,
    /// Total API-equivalent USD cost.
    pub total_cost_usd: f64,
    /// Total prompt cache savings in USD.
    pub total_cache_savings_usd: f64,
    /// Total turns / usage records.
    pub turn_count: u64,
}

/// Managed SQLite database handle wrapped with FleetHUD schema & queries.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens or creates the SQLite database at `path` and runs schema migrations.
    /// Configures WAL mode, normal synchronous, and 5-second busy timeout (`SPEC §4.3`).
    ///
    /// # Errors
    /// Returns [`Error`] if opening the connection or applying migrations fails.
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Error::Migration(e.to_string()))?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Creates an in-memory SQLite database for unit tests.
    ///
    /// # Errors
    /// Returns [`Error`] if initialization or migration fails.
    pub fn open_in_memory() -> Result<Self, Error> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> Result<(), Error> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS project (
                id TEXT PRIMARY KEY,
                canonical_key TEXT NOT NULL UNIQUE,
                repo_root TEXT NOT NULL,
                name TEXT NOT NULL,
                git_branch TEXT
            );

            CREATE TABLE IF NOT EXISTS session (
                id TEXT PRIMARY KEY,
                agent TEXT NOT NULL,
                surface TEXT NOT NULL,
                project_id TEXT NOT NULL REFERENCES project(id),
                parent_session_id TEXT,
                model TEXT,
                title TEXT,
                state TEXT NOT NULL,
                pid INTEGER,
                started_at TEXT NOT NULL,
                last_event_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_session_project ON session(project_id);
            CREATE INDEX IF NOT EXISTS idx_session_last_event ON session(last_event_at);

            CREATE TABLE IF NOT EXISTS usage_record (
                dedup_key TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES session(id),
                timestamp TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                reasoning_tokens INTEGER NOT NULL,
                cache_read_tokens INTEGER NOT NULL,
                cache_write_5m_tokens INTEGER NOT NULL,
                cache_write_1h_tokens INTEGER NOT NULL,
                context_tokens INTEGER NOT NULL,
                context_window INTEGER,
                cost_usd REAL NOT NULL,
                cache_savings_usd REAL NOT NULL,
                cost_basis TEXT NOT NULL,
                pricing_version TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_usage_session ON usage_record(session_id);
            CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_record(timestamp);

            CREATE TABLE IF NOT EXISTS activity_event (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL REFERENCES session(id),
                timestamp TEXT NOT NULL,
                kind TEXT NOT NULL,
                tool_name TEXT,
                summary_redacted TEXT,
                duration_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_activity_session ON activity_event(session_id);
            CREATE INDEX IF NOT EXISTS idx_activity_timestamp ON activity_event(timestamp);

            CREATE TABLE IF NOT EXISTS quota_snapshot (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                limit_id TEXT NOT NULL,
                window TEXT NOT NULL,
                used_pct REAL NOT NULL,
                resets_at TEXT,
                plan TEXT,
                basis TEXT NOT NULL,
                observed_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_quota_lookup ON quota_snapshot(provider, limit_id, window, observed_at);

            CREATE TABLE IF NOT EXISTS source_file (
                file_path TEXT PRIMARY KEY,
                last_read_offset INTEGER NOT NULL,
                file_size INTEGER NOT NULL,
                fingerprint TEXT NOT NULL,
                last_ingested_at TEXT NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    /// Atomically commits parsed events along with the file checkpoint in a single SQLite transaction.
    ///
    /// # Errors
    /// Returns [`Error`] if inserting any entity or updating the checkpoint fails.
    pub fn commit_batch(
        &mut self,
        projects: &[Project],
        sessions: &[Session],
        usage: &[UsageRecord],
        activity: &[ActivityEvent],
        quota: &[QuotaSnapshot],
        checkpoint: &SourceCheckpoint,
    ) -> Result<(), Error> {
        let tx = self.conn.transaction()?;

        // 1. Upsert projects
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO project (id, canonical_key, repo_root, name, git_branch)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(id) DO UPDATE SET
                    repo_root = excluded.repo_root,
                    git_branch = coalesce(excluded.git_branch, project.git_branch)
                "#,
            )?;
            for p in projects {
                stmt.execute(params![
                    p.id,
                    p.canonical_key,
                    p.repo_root,
                    p.name,
                    p.git_branch
                ])?;
            }
        }

        // 2. Upsert sessions
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO session (
                    id, agent, surface, project_id, parent_session_id,
                    model, title, state, pid, started_at, last_event_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    model = coalesce(excluded.model, session.model),
                    title = coalesce(excluded.title, session.title),
                    state = excluded.state,
                    pid = coalesce(excluded.pid, session.pid),
                    last_event_at = excluded.last_event_at
                "#,
            )?;
            for s in sessions {
                stmt.execute(params![
                    s.id,
                    s.agent.as_str(),
                    s.surface.as_str(),
                    s.project_id,
                    s.parent_session_id,
                    s.model,
                    s.title,
                    s.state.as_str(),
                    s.pid,
                    s.started_at.to_rfc3339(),
                    s.last_event_at.to_rfc3339(),
                ])?;
            }
        }

        // 3. Insert usage records (ignore duplicate dedup_key)
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT OR IGNORE INTO usage_record (
                    dedup_key, session_id, timestamp, model,
                    input_tokens, output_tokens, reasoning_tokens,
                    cache_read_tokens, cache_write_5m_tokens, cache_write_1h_tokens,
                    context_tokens, context_window, cost_usd, cache_savings_usd,
                    cost_basis, pricing_version
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
                "#,
            )?;
            for u in usage {
                stmt.execute(params![
                    u.dedup_key,
                    u.session_id,
                    u.timestamp.to_rfc3339(),
                    u.model,
                    i64::try_from(u.input_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.output_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.reasoning_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.cache_read_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.cache_write_5m_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.cache_write_1h_tokens).unwrap_or(i64::MAX),
                    i64::try_from(u.context_tokens).unwrap_or(i64::MAX),
                    u.context_window.and_then(|w| i64::try_from(w).ok()),
                    u.cost_usd,
                    u.cache_savings_usd,
                    u.cost_basis.as_str(),
                    u.pricing_version,
                ])?;
            }
        }

        // 4. Insert activity events
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO activity_event (
                    session_id, timestamp, kind, tool_name, summary_redacted, duration_ms
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                "#,
            )?;
            for a in activity {
                let kind_str = match a.kind {
                    ActivityKind::Prompt => "prompt",
                    ActivityKind::ToolStart => "tool_start",
                    ActivityKind::ToolEnd => "tool_end",
                    ActivityKind::ApprovalWait => "approval_wait",
                    ActivityKind::Error => "error",
                    ActivityKind::Subagent => "subagent",
                    ActivityKind::RateLimited => "rate_limited",
                    ActivityKind::AutoResume => "auto_resume",
                };
                stmt.execute(params![
                    a.session_id,
                    a.timestamp.to_rfc3339(),
                    kind_str,
                    a.tool_name,
                    a.summary_redacted,
                    a.duration_ms.and_then(|d| i64::try_from(d).ok()),
                ])?;
            }
        }

        // 5. Insert quota snapshots
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO quota_snapshot (
                    provider, limit_id, window, used_pct, resets_at, plan, basis, observed_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )?;
            for q in quota {
                let win_str = match q.window {
                    QuotaWindow::FiveHour => "five_hour",
                    QuotaWindow::Weekly => "weekly",
                    QuotaWindow::Monthly => "monthly",
                };
                stmt.execute(params![
                    q.provider,
                    q.limit_id,
                    win_str,
                    q.used_pct,
                    q.resets_at.map(|dt| dt.to_rfc3339()),
                    q.plan,
                    q.basis.as_str(),
                    q.observed_at.to_rfc3339(),
                ])?;
            }
        }

        // 6. Upsert source file checkpoint
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO source_file (file_path, last_read_offset, file_size, fingerprint, last_ingested_at)
                VALUES (?1, ?2, ?3, ?4, ?5)
                ON CONFLICT(file_path) DO UPDATE SET
                    last_read_offset = excluded.last_read_offset,
                    file_size = excluded.file_size,
                    fingerprint = excluded.fingerprint,
                    last_ingested_at = excluded.last_ingested_at
                "#,
            )?;
            stmt.execute(params![
                checkpoint.file_path,
                i64::try_from(checkpoint.last_read_offset).unwrap_or(i64::MAX),
                i64::try_from(checkpoint.file_size).unwrap_or(i64::MAX),
                checkpoint.fingerprint,
                checkpoint.last_ingested_at.to_rfc3339(),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Gets the checkpoint for `file_path`, if previously recorded.
    ///
    /// # Errors
    /// Returns [`Error`] if querying fails.
    pub fn get_checkpoint(&self, file_path: &str) -> Result<Option<SourceCheckpoint>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT file_path, last_read_offset, file_size, fingerprint, last_ingested_at
            FROM source_file WHERE file_path = ?1
            "#,
        )?;
        let res = stmt
            .query_row(params![file_path], |row| {
                let offset_i64: i64 = row.get(1)?;
                let size_i64: i64 = row.get(2)?;
                let ts_str: String = row.get(4)?;
                let ts = DateTime::parse_from_rfc3339(&ts_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                Ok(SourceCheckpoint {
                    file_path: row.get(0)?,
                    last_read_offset: u64::try_from(offset_i64.max(0)).unwrap_or(0),
                    file_size: u64::try_from(size_i64.max(0)).unwrap_or(0),
                    fingerprint: row.get(3)?,
                    last_ingested_at: ts,
                })
            })
            .optional()?;
        Ok(res)
    }

    /// Queries the most recent live sessions for `fleethud status` CLI display.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn list_active_sessions(&self, limit: usize) -> Result<Vec<Session>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, agent, surface, project_id, parent_session_id, model,
                   title, state, pid, started_at, last_event_at
            FROM session
            ORDER BY last_event_at DESC
            LIMIT ?1
            "#,
        )?;

        let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
        let rows = stmt.query_map(params![limit_i64], |row| {
            let agent_str: String = row.get(1)?;
            let agent = match agent_str.as_str() {
                "claude-code" => AgentId::ClaudeCode,
                "codex" => AgentId::Codex,
                "antigravity" => AgentId::Antigravity,
                "gemini-cli" => AgentId::GeminiCli,
                "cline" => AgentId::Cline,
                "cursor" => AgentId::Cursor,
                "aider" => AgentId::Aider,
                _ => AgentId::ClaudeCode,
            };
            let surface_str: String = row.get(2)?;
            let surface = match surface_str.as_str() {
                "ide" => Surface::Ide,
                "desktop_app" => Surface::DesktopApp,
                _ => Surface::Cli,
            };
            let state_str: String = row.get(7)?;
            let state = match state_str.as_str() {
                "thinking" => SessionState::Thinking,
                "tool" => SessionState::Tool,
                "awaiting_approval" => SessionState::AwaitingApproval,
                "error" => SessionState::Error,
                "ended" => SessionState::Ended,
                _ => SessionState::Idle,
            };
            let s_at_str: String = row.get(9)?;
            let l_at_str: String = row.get(10)?;
            let started_at = DateTime::parse_from_rfc3339(&s_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let last_event_at = DateTime::parse_from_rfc3339(&l_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Session {
                id: row.get(0)?,
                agent,
                surface,
                project_id: row.get(3)?,
                parent_session_id: row.get(4)?,
                model: row.get(5)?,
                title: row.get(6)?,
                state,
                pid: row.get(8)?,
                started_at,
                last_event_at,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Aggregates usage report grouped by project, agent, and model for `fleethud report`.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn generate_usage_report(&self) -> Result<Vec<ReportRow>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                p.name AS project_name,
                s.agent AS agent_str,
                u.model,
                SUM(u.input_tokens) AS in_tok,
                SUM(u.cache_read_tokens) AS read_tok,
                SUM(u.output_tokens + u.reasoning_tokens) AS out_tok,
                SUM(u.cost_usd) AS total_cost,
                SUM(u.cache_savings_usd) AS total_savings,
                COUNT(u.dedup_key) AS turns
            FROM usage_record u
            JOIN session s ON u.session_id = s.id
            JOIN project p ON s.project_id = p.id
            GROUP BY p.name, s.agent, u.model
            ORDER BY total_cost DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let agent_str: String = row.get(1)?;
            let agent = match agent_str.as_str() {
                "claude-code" => AgentId::ClaudeCode,
                "codex" => AgentId::Codex,
                "antigravity" => AgentId::Antigravity,
                "gemini-cli" => AgentId::GeminiCli,
                "cline" => AgentId::Cline,
                "cursor" => AgentId::Cursor,
                "aider" => AgentId::Aider,
                _ => AgentId::ClaudeCode,
            };
            let in_tok: i64 = row.get(3)?;
            let read_tok: i64 = row.get(4)?;
            let out_tok: i64 = row.get(5)?;
            let turn_count: i64 = row.get(8)?;

            Ok(ReportRow {
                project_name: row.get(0)?,
                agent,
                model: row.get(2)?,
                input_tokens: u64::try_from(in_tok.max(0)).unwrap_or(0),
                cache_read_tokens: u64::try_from(read_tok.max(0)).unwrap_or(0),
                output_tokens: u64::try_from(out_tok.max(0)).unwrap_or(0),
                total_cost_usd: row.get(6)?,
                total_cache_savings_usd: row.get(7)?,
                turn_count: u64::try_from(turn_count.max(0)).unwrap_or(0),
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Queries the latest quota pool snapshot for each distinct `(provider, limit_id, window)`.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_latest_quota_snapshots(&self) -> Result<Vec<QuotaSnapshot>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT q1.provider, q1.limit_id, q1.window, q1.used_pct, q1.resets_at, q1.plan, q1.basis, q1.observed_at
            FROM quota_snapshot q1
            JOIN (
                SELECT provider, limit_id, window, MAX(observed_at) AS max_obs
                FROM quota_snapshot
                GROUP BY provider, limit_id, window
            ) q2 ON q1.provider = q2.provider
                AND q1.limit_id = q2.limit_id
                AND q1.window = q2.window
                AND q1.observed_at = q2.max_obs
            ORDER BY q1.provider, q1.limit_id
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let win_str: String = row.get(2)?;
            let window = match win_str.as_str() {
                "five_hour" => QuotaWindow::FiveHour,
                "monthly" => QuotaWindow::Monthly,
                _ => QuotaWindow::Weekly,
            };
            let resets_str: Option<String> = row.get(4)?;
            let resets_at = resets_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });
            let obs_str: String = row.get(7)?;
            let observed_at = DateTime::parse_from_rfc3339(&obs_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(QuotaSnapshot {
                provider: row.get(0)?,
                limit_id: row.get(1)?,
                window,
                used_pct: row.get(3)?,
                resets_at,
                plan: row.get(5)?,
                basis: TrustLevel::Exact,
                observed_at,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, SourceCheckpoint};
    use chrono::Utc;
    use fleethud_core::{
        ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
        SessionState, Surface, TrustLevel, UsageRecord,
    };

    #[test]
    fn commit_batch_persists_and_queries_idempotently() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let p = Project::from_origin(
            Some("https://github.com/mwahaj36/fleethud"),
            "/workspace/fleethud",
            Some("main"),
        );
        let s = Session {
            id: "claude-code:s1".to_owned(),
            agent: AgentId::ClaudeCode,
            surface: Surface::Ide,
            project_id: p.id.clone(),
            parent_session_id: None,
            model: Some("claude-opus-5".to_owned()),
            title: Some("Implement M1".to_owned()),
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let u = UsageRecord {
            dedup_key: "msg1:req1".to_owned(),
            session_id: s.id.clone(),
            timestamp: Utc::now(),
            model: "claude-opus-5".to_owned(),
            input_tokens: 1000,
            output_tokens: 200,
            reasoning_tokens: 0,
            cache_read_tokens: 5000,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 6000,
            context_window: Some(200_000),
            cost_usd: 0.15,
            cache_savings_usd: 0.05,
            cost_basis: TrustLevel::Exact,
            pricing_version: "2026-10-26".to_owned(),
        };
        let a = ActivityEvent {
            session_id: s.id.clone(),
            timestamp: Utc::now(),
            kind: ActivityKind::ToolStart,
            tool_name: Some("cargo test".to_owned()),
            summary_redacted: Some("cargo test --workspace".to_owned()),
            duration_ms: None,
        };
        let q = QuotaSnapshot {
            provider: "openai".to_owned(),
            limit_id: "codex".to_owned(),
            window: QuotaWindow::FiveHour,
            used_pct: 42.0,
            resets_at: None,
            plan: Some("plus".to_owned()),
            basis: TrustLevel::Exact,
            observed_at: Utc::now(),
        };
        let cp = SourceCheckpoint {
            file_path: "/path/to/log.jsonl".to_owned(),
            last_read_offset: 1234,
            file_size: 1234,
            fingerprint: "hash".to_owned(),
            last_ingested_at: Utc::now(),
        };

        db.commit_batch(&[p], &[s], std::slice::from_ref(&u), &[a], &[q], &cp)
            .expect("first commit");

        // Re-committing with identical usage dedup_key must be idempotent (no duplicate key error)
        db.commit_batch(&[], &[], &[u], &[], &[], &cp)
            .expect("idempotent second commit");

        let active = db.list_active_sessions(10).expect("list active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "claude-code:s1");

        let report = db.generate_usage_report().expect("report");
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].turn_count, 1);
        assert_eq!(report[0].input_tokens, 1000);

        let latest_quota = db.get_latest_quota_snapshots().expect("latest quota");
        assert_eq!(latest_quota.len(), 1);
        assert_eq!(latest_quota[0].limit_id, "codex");

        let saved_cp = db
            .get_checkpoint("/path/to/log.jsonl")
            .expect("get checkpoint")
            .expect("checkpoint exists");
        assert_eq!(saved_cp.last_read_offset, 1234);
    }
}
