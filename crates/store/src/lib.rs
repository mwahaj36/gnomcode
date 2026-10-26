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

/// User profile with per-provider active subscription plans.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    /// Unique user identifier.
    pub id: String,
    /// User handle.
    pub username: String,
    /// Display name shown in UI.
    pub display_name: String,
    /// Monthly plan price for OpenAI tools (Codex).
    pub openai_plan_usd: f64,
    /// Monthly plan price for Anthropic tools (Claude Code).
    pub claude_plan_usd: f64,
    /// Monthly plan price for Google tools (Antigravity).
    pub google_plan_usd: f64,
    /// Monthly plan price for other tools.
    pub other_plan_usd: f64,
    /// ISO timestamp of last update.
    pub updated_at: String,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            id: "local_user".to_owned(),
            username: "developer".to_owned(),
            display_name: "Local Developer".to_owned(),
            openai_plan_usd: 20.0,
            claude_plan_usd: 20.0,
            google_plan_usd: 0.0,
            other_plan_usd: 0.0,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Enriched session item for UI list with project name and clean state.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SessionListItem {
    /// Session ID.
    pub id: String,
    /// Agent name.
    pub agent: String,
    /// Surface.
    pub surface: String,
    /// Project name.
    pub project_name: String,
    /// Model name.
    pub model: String,
    /// State (tool, thinking, awaiting_approval, idle, ended).
    pub state: String,
    /// Whether actively executing in the last 15 minutes.
    pub is_active: bool,
    /// Started at timestamp.
    pub started_at: String,
    /// Last event timestamp.
    pub last_event_at: String,
    /// Total input tokens consumed.
    pub input_tokens: u64,
    /// Total output tokens generated.
    pub output_tokens: u64,
    /// Total API-equivalent cost USD.
    pub total_cost_usd: f64,
}

/// Breakdown for a single model under a provider.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelReport {
    /// Model name.
    pub model: String,
    /// Agent slug.
    pub agent: String,
    /// Input tokens.
    pub input_tokens: u64,
    /// Cached tokens read.
    pub cache_read_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Dollar spend.
    pub cost_usd: f64,
    /// Cache savings.
    pub cache_savings_usd: f64,
    /// Turn count.
    pub turn_count: u64,
}

/// Breakdown for an AI provider (Anthropic, OpenAI, Google).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProviderReport {
    /// Provider name ("Anthropic", "OpenAI", "Google", "Other").
    pub provider: String,
    /// Total cost in USD.
    pub total_cost_usd: f64,
    /// Total prompt cache savings.
    pub total_cache_savings_usd: f64,
    /// Percentage of total spend across all providers.
    pub pct_of_spend: f64,
    /// Models under this provider.
    pub models: Vec<ModelReport>,
}

/// Filtered timeline report with date spans and per-provider ROI comparison.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelineReport {
    /// Range label (e.g. "Today", "Last 7 Days", "All Time").
    pub range_label: String,
    /// Earliest event timestamp in range.
    pub earliest_at: Option<String>,
    /// Latest event timestamp in range.
    pub latest_at: Option<String>,
    /// Total cost across all providers.
    pub total_cost_usd: f64,
    /// Total cache savings across all providers.
    pub total_cache_savings_usd: f64,
    /// Total input tokens.
    pub total_input_tokens: u64,
    /// Total cache read tokens.
    pub total_cache_read_tokens: u64,
    /// Total output tokens.
    pub total_output_tokens: u64,
    /// Total turns.
    pub total_turns: u64,
    /// Providers list.
    pub providers: Vec<ProviderReport>,
    /// Total item count.
    pub items_count: usize,
}

/// Local database storage statistics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DatabaseStats {
    /// Total sessions recorded.
    pub total_sessions: u64,
    /// Total turns recorded.
    pub total_turns: u64,
    /// Total activity events recorded.
    pub total_events: u64,
    /// Total projects tracked.
    pub total_projects: u64,
    /// Database size in bytes.
    pub db_size_bytes: u64,
}

/// User-configured custom model pricing rate override.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CustomModelPricing {
    /// Pattern to match against model identifier (e.g. `claude-3-5-sonnet`, `gemini`).
    pub model_pattern: String,
    /// Uncached input tokens USD per 1M tokens.
    pub input_per_mtok_usd: f64,
    /// Output & reasoning tokens USD per 1M tokens.
    pub output_per_mtok_usd: f64,
    /// Cached input tokens USD per 1M tokens read.
    pub cache_read_per_mtok_usd: f64,
    /// Timestamp of last modification.
    pub updated_at: String,
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

            CREATE TABLE IF NOT EXISTS user_profile (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL,
                display_name TEXT NOT NULL,
                openai_plan_usd REAL NOT NULL DEFAULT 20.0,
                claude_plan_usd REAL NOT NULL DEFAULT 20.0,
                google_plan_usd REAL NOT NULL DEFAULT 0.0,
                other_plan_usd REAL NOT NULL DEFAULT 0.0,
                updated_at TEXT NOT NULL
            );

            INSERT OR IGNORE INTO user_profile (id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at)
            VALUES ('local_user', 'developer', 'Local Developer', 20.0, 20.0, 0.0, 0.0, datetime('now'));

            CREATE TABLE IF NOT EXISTS custom_model_pricing (
                model_pattern TEXT PRIMARY KEY,
                input_per_mtok_usd REAL NOT NULL,
                output_per_mtok_usd REAL NOT NULL,
                cache_read_per_mtok_usd REAL NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;

        let has_v5: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 5",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v5 {
            self.conn.execute_batch(
                r#"
                DELETE FROM source_file;
                DELETE FROM activity_event;
                INSERT INTO schema_migrations (version, applied_at) VALUES (5, datetime('now'));
                "#,
            )?;
        }

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
                INSERT INTO activity_event (session_id, timestamp, kind, tool_name, summary_redacted, duration_ms)
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
                let duration_i64 = a.duration_ms.and_then(|d| i64::try_from(d).ok());
                stmt.execute(params![
                    a.session_id,
                    a.timestamp.to_rfc3339(),
                    kind_str,
                    a.tool_name,
                    a.summary_redacted,
                    duration_i64,
                ])?;
            }
        }

        // 5. Insert quota snapshots
        {
            let mut stmt = tx.prepare_cached(
                r#"
                INSERT INTO quota_snapshot (provider, limit_id, window, used_pct, resets_at, plan, basis, observed_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                "#,
            )?;
            for q in quota {
                let resets_str = q.resets_at.map(|dt| dt.to_rfc3339());
                let window_str = match q.window {
                    QuotaWindow::FiveHour => "five_hour",
                    QuotaWindow::Weekly => "weekly",
                    QuotaWindow::Monthly => "monthly",
                };
                stmt.execute(params![
                    q.provider,
                    q.limit_id,
                    window_str,
                    q.used_pct,
                    resets_str,
                    q.plan,
                    q.basis.as_str(),
                    q.observed_at.to_rfc3339(),
                ])?;
            }
        }

        // 6. Update source file checkpoint
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

    /// Queries the latest committed checkpoint for a log file.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_checkpoint(&self, file_path: &str) -> Result<Option<SourceCheckpoint>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT file_path, last_read_offset, file_size, fingerprint, last_ingested_at
            FROM source_file
            WHERE file_path = ?1
            "#,
        )?;

        let mut rows = stmt.query(params![file_path])?;
        if let Some(row) = rows.next()? {
            let offset_i64: i64 = row.get(1)?;
            let size_i64: i64 = row.get(2)?;
            let ts_str: String = row.get(4)?;
            let last_ingested_at = DateTime::parse_from_rfc3339(&ts_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Some(SourceCheckpoint {
                file_path: row.get(0)?,
                last_read_offset: u64::try_from(offset_i64.max(0)).unwrap_or(0),
                file_size: u64::try_from(size_i64.max(0)).unwrap_or(0),
                fingerprint: row.get(3)?,
                last_ingested_at,
            }))
        } else {
            Ok(None)
        }
    }

    /// Queries live and recent active agent sessions, ordered by `last_event_at DESC`.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn list_active_sessions(&self, limit: usize) -> Result<Vec<Session>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, agent, surface, project_id, parent_session_id, model, title, state, pid, started_at, last_event_at
            FROM session
            ORDER BY last_event_at DESC
            LIMIT ?1
            "#,
        )?;

        let limit_i64 = i64::try_from(limit).unwrap_or(50);
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
            let raw_state = match state_str.as_str() {
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

            let now = Utc::now();
            let is_stale = (now - last_event_at).num_minutes() > 15;
            let state = if is_stale
                && (raw_state == SessionState::Tool || raw_state == SessionState::Thinking)
            {
                SessionState::Idle
            } else {
                raw_state
            };

            let raw_model: Option<String> = row.get(5)?;
            let model = match raw_model {
                Some(ref m) if m.contains("MODEL_PLACEHOLDER") => {
                    Some("Gemini 3.8 Flash (Default)".to_owned())
                }
                other => other,
            };

            Ok(Session {
                id: row.get(0)?,
                agent,
                surface,
                project_id: row.get(3)?,
                parent_session_id: row.get(4)?,
                model,
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

        let custom_prices = self.get_custom_model_pricings().unwrap_or_default();
        let mut out = Vec::new();
        for r in rows {
            let mut row = r?;
            if let Some(custom) = custom_prices.iter().find(|cp| {
                row.model
                    .to_lowercase()
                    .contains(&cp.model_pattern.to_lowercase())
            }) {
                let mtok = 1_000_000.0_f64;
                let in_c = (row.input_tokens as f64 / mtok) * custom.input_per_mtok_usd;
                let out_c = (row.output_tokens as f64 / mtok) * custom.output_per_mtok_usd;
                let read_c = (row.cache_read_tokens as f64 / mtok) * custom.cache_read_per_mtok_usd;
                row.total_cost_usd = in_c + out_c + read_c;
                let full_in = (row.cache_read_tokens as f64 / mtok) * custom.input_per_mtok_usd;
                row.total_cache_savings_usd = (full_in - read_c).max(0.0);
            }
            out.push(row);
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

        // Rolling 5-hour and 7-day quota pools for Antigravity & Claude Code models
        let cutoff_5h = (Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
        let cutoff_7d = (Utc::now() - chrono::Duration::days(7)).to_rfc3339();

        if let Ok(mut stmt_roll) = self.conn.prepare(
            r#"
            SELECT s.agent, u.model,
                   SUM(CASE WHEN u.timestamp >= ?1 THEN u.input_tokens + u.output_tokens ELSE 0 END) AS tok_5h,
                   SUM(CASE WHEN u.timestamp >= ?2 THEN u.input_tokens + u.output_tokens ELSE 0 END) AS tok_7d,
                   MIN(CASE WHEN u.timestamp >= ?1 THEN u.timestamp ELSE NULL END) AS earliest_5h,
                   MIN(CASE WHEN u.timestamp >= ?2 THEN u.timestamp ELSE NULL END) AS earliest_7d
            FROM usage_record u
            JOIN session s ON u.session_id = s.id
            WHERE u.timestamp >= ?2
              AND s.agent IN ('antigravity', 'claude-code')
            GROUP BY s.agent, u.model
            ORDER BY tok_5h DESC
            "#,
        ) {
            if let Ok(model_rows) = stmt_roll.query_map(params![cutoff_5h, cutoff_7d], |r| {
                let agent: String = r.get(0)?;
                let model: String = r.get(1)?;
                let tok_5h: i64 = r.get(2)?;
                let tok_7d: i64 = r.get(3)?;
                let early_5h: Option<String> = r.get(4)?;
                let early_7d: Option<String> = r.get(5)?;
                Ok((
                    agent,
                    model,
                    tok_5h.max(0) as u64,
                    tok_7d.max(0) as u64,
                    early_5h,
                    early_7d,
                ))
            }) {
                use std::collections::BTreeMap;
                struct AggQuota {
                    tok_5h: u64,
                    tok_7d: u64,
                    early_5h: Option<String>,
                    early_7d: Option<String>,
                }
                let mut agg_map: BTreeMap<(String, String), AggQuota> = BTreeMap::new();

                for mr in model_rows.flatten() {
                    let (agent, raw_model, tok_5h, tok_7d, early_5h, early_7d) = mr;
                    let lower = raw_model.to_lowercase();
                    let limit_slug = if lower.contains("flash") {
                        "gemini-3.8-flash".to_owned()
                    } else if lower.contains("argon") {
                        "gemini-4-argon".to_owned()
                    } else if lower.contains("opus") {
                        if lower.contains("5.5") {
                            "claude-opus-5.5".to_owned()
                        } else {
                            "claude-opus-5".to_owned()
                        }
                    } else if lower.contains("sonnet") {
                        "claude-sonnet-4".to_owned()
                    } else {
                        let base = if let Some(idx) = raw_model.find('(') {
                            raw_model[..idx].trim()
                        } else {
                            raw_model.trim()
                        };
                        base.to_lowercase()
                            .replace([' ', '/', '_'], "-")
                            .trim_matches('-')
                            .to_owned()
                    };

                    let entry = agg_map.entry((agent, limit_slug)).or_insert_with(|| AggQuota {
                        tok_5h: 0,
                        tok_7d: 0,
                        early_5h: None,
                        early_7d: None,
                    });
                    entry.tok_5h += tok_5h;
                    entry.tok_7d += tok_7d;
                    if let Some(e) = early_5h {
                        if entry.early_5h.as_ref().is_none_or(|cur| &e < cur) {
                            entry.early_5h = Some(e);
                        }
                    }
                    if let Some(e) = early_7d {
                        if entry.early_7d.as_ref().is_none_or(|cur| &e < cur) {
                            entry.early_7d = Some(e);
                        }
                    }
                }

                for ((agent, limit_slug), agg) in agg_map {
                    let (cap_5h, cap_7d) = if limit_slug.contains("claude-opus") {
                        (1_500_000.0_f64, 15_000_000.0_f64)
                    } else if limit_slug.contains("argon") {
                        (5_000_000.0_f64, 50_000_000.0_f64)
                    } else if limit_slug.contains("flash") {
                        (10_000_000.0_f64, 100_000_000.0_f64)
                    } else {
                        (3_000_000.0_f64, 30_000_000.0_f64)
                    };

                    let now = Utc::now();

                    // 5-hour window
                    if !out.iter().any(|q| {
                        q.provider == agent
                            && q.limit_id == limit_slug
                            && q.window == QuotaWindow::FiveHour
                    }) {
                        let used_5h_pct = ((agg.tok_5h as f64 / cap_5h) * 100.0).clamp(0.0, 100.0);
                        let resets_at_5h = agg.early_5h.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::hours(5))
                                .ok()
                        });
                        out.push(QuotaSnapshot {
                            provider: agent.clone(),
                            limit_id: limit_slug.clone(),
                            window: QuotaWindow::FiveHour,
                            used_pct: used_5h_pct,
                            resets_at: resets_at_5h,
                            plan: Some("standard".to_owned()),
                            basis: TrustLevel::Estimated,
                            observed_at: now,
                        });
                    }

                    // Weekly window
                    if !out.iter().any(|q| {
                        q.provider == agent
                            && q.limit_id == limit_slug
                            && q.window == QuotaWindow::Weekly
                    }) {
                        let used_7d_pct = ((agg.tok_7d as f64 / cap_7d) * 100.0).clamp(0.0, 100.0);
                        let resets_at_7d = agg.early_7d.and_then(|s| {
                            DateTime::parse_from_rfc3339(&s)
                                .map(|dt| dt.with_timezone(&Utc) + chrono::Duration::days(7))
                                .ok()
                        });
                        out.push(QuotaSnapshot {
                            provider: agent,
                            limit_id: limit_slug,
                            window: QuotaWindow::Weekly,
                            used_pct: used_7d_pct,
                            resets_at: resets_at_7d,
                            plan: Some("standard".to_owned()),
                            basis: TrustLevel::Estimated,
                            observed_at: now,
                        });
                    }
                }
            }
        }

        Ok(out)
    }

    /// Queries the local user profile and preferences.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_user_profile(&self) -> Result<UserProfile, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at
            FROM user_profile
            WHERE id = 'local_user'
            LIMIT 1
            "#,
        )?;
        let res = stmt
            .query_row([], |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    openai_plan_usd: row.get(3)?,
                    claude_plan_usd: row.get(4)?,
                    google_plan_usd: row.get(5)?,
                    other_plan_usd: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(res.unwrap_or_default())
    }

    /// Updates or inserts the local user profile and plan settings.
    ///
    /// # Errors
    /// Returns [`Error`] if database update fails.
    pub fn update_user_profile(&self, p: &UserProfile) -> Result<(), Error> {
        let mut stmt = self.conn.prepare(
            r#"
            INSERT INTO user_profile (id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                username = excluded.username,
                display_name = excluded.display_name,
                openai_plan_usd = excluded.openai_plan_usd,
                claude_plan_usd = excluded.claude_plan_usd,
                google_plan_usd = excluded.google_plan_usd,
                other_plan_usd = excluded.other_plan_usd,
                updated_at = excluded.updated_at
            "#,
        )?;
        stmt.execute(params![
            "local_user",
            p.username,
            p.display_name,
            p.openai_plan_usd,
            p.claude_plan_usd,
            p.google_plan_usd,
            p.other_plan_usd,
            Utc::now().to_rfc3339()
        ])?;
        Ok(())
    }

    /// Queries all custom model pricing overrides.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_custom_model_pricings(&self) -> Result<Vec<CustomModelPricing>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT model_pattern, input_per_mtok_usd, output_per_mtok_usd, cache_read_per_mtok_usd, updated_at FROM custom_model_pricing ORDER BY model_pattern",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CustomModelPricing {
                model_pattern: r.get(0)?,
                input_per_mtok_usd: r.get(1)?,
                output_per_mtok_usd: r.get(2)?,
                cache_read_per_mtok_usd: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Sets or updates a custom model pricing override.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn set_custom_model_pricing(&mut self, pricing: &CustomModelPricing) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO custom_model_pricing (model_pattern, input_per_mtok_usd, output_per_mtok_usd, cache_read_per_mtok_usd, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(model_pattern) DO UPDATE SET
                input_per_mtok_usd = excluded.input_per_mtok_usd,
                output_per_mtok_usd = excluded.output_per_mtok_usd,
                cache_read_per_mtok_usd = excluded.cache_read_per_mtok_usd,
                updated_at = excluded.updated_at
            "#,
            params![
                pricing.model_pattern,
                pricing.input_per_mtok_usd,
                pricing.output_per_mtok_usd,
                pricing.cache_read_per_mtok_usd,
                pricing.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Deletes a custom model pricing override.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn delete_custom_model_pricing(&mut self, model_pattern: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM custom_model_pricing WHERE model_pattern = ?1",
            params![model_pattern],
        )?;
        Ok(())
    }

    /// Queries the stored model name for a session if already known.
    #[must_use]
    pub fn get_session_model(&self, session_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT model FROM session WHERE id = ?1",
                params![session_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
            .filter(|m| !m.trim().is_empty())
    }

    /// Automatically populates subscription plan prices from local agent tiers
    /// (`quota_snapshot.plan` for OpenAI, plus passed-in Anthropic/Google tiers)
    /// when not yet customized by the user.
    ///
    /// # Errors
    /// Returns [`Error`] if querying or updating `user_profile` fails.
    pub fn auto_populate_subscriptions(
        &mut self,
        anthropic_usd: Option<f64>,
        google_usd: Option<f64>,
    ) -> Result<(), Error> {
        let mut profile = self.get_user_profile()?;
        let mut changed = false;

        let codex_plan: Option<String> = self
            .conn
            .query_row(
                "SELECT plan FROM quota_snapshot WHERE provider = 'codex' AND plan IS NOT NULL ORDER BY observed_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(plan) = codex_plan {
            let detected_openai = match plan.to_ascii_lowercase().as_str() {
                "pro" => Some(200.0),
                "enterprise" => Some(60.0),
                "team" => Some(30.0),
                "plus" => Some(20.0),
                _ => None,
            };
            if let Some(price) = detected_openai {
                if (profile.openai_plan_usd - 0.0).abs() < f64::EPSILON
                    || ((profile.openai_plan_usd - 20.0).abs() < f64::EPSILON && price > 20.0)
                {
                    profile.openai_plan_usd = price;
                    changed = true;
                }
            }
        }

        if let Some(price) = anthropic_usd {
            if (profile.claude_plan_usd - 0.0).abs() < f64::EPSILON {
                profile.claude_plan_usd = price;
                changed = true;
            }
        }

        if let Some(price) = google_usd {
            if (profile.google_plan_usd - 0.0).abs() < f64::EPSILON {
                profile.google_plan_usd = price;
                changed = true;
            }
        }

        if changed {
            self.update_user_profile(&profile)?;
        }
        Ok(())
    }

    /// Returns paginated session list with joined project names and dynamic active status.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn list_sessions_paginated(
        &self,
        agent_filter: Option<&str>,
        status_filter: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SessionListItem>, usize), Error> {
        let now = Utc::now();
        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                s.id, s.agent, s.surface, p.name, s.model, s.state, s.started_at, s.last_event_at,
                COALESCE(SUM(u.input_tokens), 0),
                COALESCE(SUM(u.output_tokens), 0),
                COALESCE(SUM(u.cost_usd), 0.0)
            FROM session s
            JOIN project p ON s.project_id = p.id
            LEFT JOIN usage_record u ON s.id = u.session_id
            GROUP BY s.id
            ORDER BY s.last_event_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let agent: String = row.get(1)?;
            let surface: String = row.get(2)?;
            let project_name: String = row.get(3)?;
            let raw_model: Option<String> = row.get(4)?;
            let state_str: String = row.get(5)?;
            let started_at: String = row.get(6)?;
            let last_event_at_str: String = row.get(7)?;
            let input_tokens: i64 = row.get(8)?;
            let output_tokens: i64 = row.get(9)?;
            let total_cost_usd: f64 = row.get(10)?;

            let last_event_at = DateTime::parse_from_rfc3339(&last_event_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now);

            let diff_mins = (now - last_event_at).num_minutes();
            let is_recent = diff_mins <= 15;

            let (state, is_active) = if is_recent && state_str != "ended" {
                let effective = if state_str == "idle" {
                    "thinking".to_owned()
                } else {
                    state_str
                };
                (effective, true)
            } else {
                ("idle".to_owned(), false)
            };

            let model = match raw_model {
                Some(ref m) if m.contains("MODEL_PLACEHOLDER") => {
                    "Gemini 3.8 Flash (Default)".to_owned()
                }
                Some(m) if !m.trim().is_empty() => m,
                _ => "Unspecified".to_owned(),
            };

            Ok(SessionListItem {
                id,
                agent,
                surface,
                project_name,
                model,
                state,
                is_active,
                started_at,
                last_event_at: last_event_at_str,
                input_tokens: input_tokens.max(0) as u64,
                output_tokens: output_tokens.max(0) as u64,
                total_cost_usd,
            })
        })?;

        let mut filtered = Vec::new();
        for r in rows {
            let item = r?;
            if let Some(af) = agent_filter {
                if !af.is_empty() && af != "all" && item.agent != af {
                    continue;
                }
            }
            if let Some(sf) = status_filter {
                if sf == "active" && !item.is_active {
                    continue;
                }
                if sf == "idle" && item.is_active {
                    continue;
                }
            }
            filtered.push(item);
        }

        filtered.sort_by(|a, b| {
            let ts_a = DateTime::parse_from_rfc3339(&a.last_event_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            let ts_b = DateTime::parse_from_rfc3339(&b.last_event_at)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            b.is_active.cmp(&a.is_active).then_with(|| ts_b.cmp(&ts_a))
        });

        let total_count = filtered.len();
        let paginated = filtered.into_iter().skip(offset).take(limit).collect();
        Ok((paginated, total_count))
    }

    /// Generates timeline-filtered report grouped by provider (Anthropic, OpenAI, Google)
    /// and model.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn generate_usage_report_timeline(
        &self,
        range_kind: &str,
        from_ts: Option<&str>,
        to_ts: Option<&str>,
    ) -> Result<TimelineReport, Error> {
        let now = Utc::now();
        let (cutoff, range_label) = match range_kind {
            "today" => {
                let start_of_day = now
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .map(|dt| dt.and_utc())
                    .unwrap_or(now);
                (Some(start_of_day.to_rfc3339()), "Today".to_owned())
            }
            "7d" => {
                let d = now - chrono::Duration::days(7);
                (Some(d.to_rfc3339()), "Last 7 Days".to_owned())
            }
            "30d" => {
                let d = now - chrono::Duration::days(30);
                (Some(d.to_rfc3339()), "Last 30 Days".to_owned())
            }
            "custom" if from_ts.is_some() => {
                (from_ts.map(str::to_owned), "Custom Range".to_owned())
            }
            _ => (None, "All Time".to_owned()),
        };

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
                COUNT(u.dedup_key) AS turns,
                MIN(u.timestamp) AS earliest,
                MAX(u.timestamp) AS latest
            FROM usage_record u
            JOIN session s ON u.session_id = s.id
            JOIN project p ON s.project_id = p.id
            WHERE (?1 IS NULL OR u.timestamp >= ?1)
              AND (?2 IS NULL OR u.timestamp <= ?2)
            GROUP BY p.name, s.agent, u.model
            ORDER BY total_cost DESC
            "#,
        )?;

        let mut total_spend = 0.0;
        let mut total_savings = 0.0;
        let mut total_in = 0u64;
        let mut total_read = 0u64;
        let mut total_out = 0u64;
        let mut total_turns = 0u64;
        let mut global_earliest: Option<String> = None;
        let mut global_latest: Option<String> = None;

        struct RawModelItem {
            agent: String,
            model: String,
            in_tok: u64,
            read_tok: u64,
            out_tok: u64,
            cost: f64,
            savings: f64,
            turns: u64,
        }
        let mut raw_items = Vec::new();

        let rows = stmt.query_map(params![cutoff, to_ts], |row| {
            let agent: String = row.get(1)?;
            let raw_m: String = row.get(2)?;
            let in_tok: i64 = row.get(3)?;
            let read_tok: i64 = row.get(4)?;
            let out_tok: i64 = row.get(5)?;
            let cost: f64 = row.get(6)?;
            let savings: f64 = row.get(7)?;
            let turns: i64 = row.get(8)?;
            let earliest: Option<String> = row.get(9)?;
            let latest: Option<String> = row.get(10)?;

            let model = if raw_m.contains("MODEL_PLACEHOLDER") {
                "Gemini 3.8 Flash (Default)".to_owned()
            } else {
                raw_m
            };

            let in_u = u64::try_from(in_tok.max(0)).unwrap_or(0);
            let read_u = u64::try_from(read_tok.max(0)).unwrap_or(0);
            let out_u = u64::try_from(out_tok.max(0)).unwrap_or(0);
            let turns_u = u64::try_from(turns.max(0)).unwrap_or(0);

            Ok((
                RawModelItem {
                    agent,
                    model,
                    in_tok: in_u,
                    read_tok: read_u,
                    out_tok: out_u,
                    cost,
                    savings,
                    turns: turns_u,
                },
                earliest,
                latest,
            ))
        })?;

        let custom_prices = self.get_custom_model_pricings().unwrap_or_default();

        for r in rows {
            let (mut item, e, l) = r?;

            if let Some(custom) = custom_prices.iter().find(|cp| {
                item.model
                    .to_lowercase()
                    .contains(&cp.model_pattern.to_lowercase())
            }) {
                let mtok = 1_000_000.0_f64;
                let in_c = (item.in_tok as f64 / mtok) * custom.input_per_mtok_usd;
                let out_c = (item.out_tok as f64 / mtok) * custom.output_per_mtok_usd;
                let read_c = (item.read_tok as f64 / mtok) * custom.cache_read_per_mtok_usd;
                item.cost = in_c + out_c + read_c;
                let full_in = (item.read_tok as f64 / mtok) * custom.input_per_mtok_usd;
                item.savings = (full_in - read_c).max(0.0);
            }

            total_spend += item.cost;
            total_savings += item.savings;
            total_in += item.in_tok;
            total_read += item.read_tok;
            total_out += item.out_tok;
            total_turns += item.turns;

            if let Some(e_str) = e {
                if global_earliest.as_ref().is_none_or(|cur| &e_str < cur) {
                    global_earliest = Some(e_str);
                }
            }
            if let Some(l_str) = l {
                if global_latest.as_ref().is_none_or(|cur| &l_str > cur) {
                    global_latest = Some(l_str);
                }
            }

            raw_items.push(item);
        }

        use std::collections::BTreeMap;
        let mut agent_models: BTreeMap<&'static str, BTreeMap<String, ModelReport>> =
            BTreeMap::new();
        for item in raw_items {
            let agent_label = fleethud_core::pricing::format_agent_display(&item.agent);
            let models_map = agent_models.entry(agent_label).or_default();
            let entry = models_map
                .entry(item.model.clone())
                .or_insert_with(|| ModelReport {
                    model: item.model,
                    agent: item.agent,
                    input_tokens: 0,
                    cache_read_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    cache_savings_usd: 0.0,
                    turn_count: 0,
                });
            entry.input_tokens += item.in_tok;
            entry.cache_read_tokens += item.read_tok;
            entry.output_tokens += item.out_tok;
            entry.cost_usd += item.cost;
            entry.cache_savings_usd += item.savings;
            entry.turn_count += item.turns;
        }

        let mut providers = Vec::new();
        for (agent_label, models_map) in agent_models {
            let mut models: Vec<ModelReport> = models_map.into_values().collect();
            models.sort_by(|a, b| {
                b.cost_usd
                    .partial_cmp(&a.cost_usd)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let mut prov_cost = 0.0;
            let mut prov_savings = 0.0;
            for m in &models {
                prov_cost += m.cost_usd;
                prov_savings += m.cache_savings_usd;
            }
            if prov_cost > 0.0 || !models.is_empty() {
                let pct = if total_spend > 0.0 {
                    (prov_cost / total_spend) * 100.0
                } else {
                    0.0
                };
                providers.push(ProviderReport {
                    provider: agent_label.to_owned(),
                    total_cost_usd: prov_cost,
                    total_cache_savings_usd: prov_savings,
                    pct_of_spend: pct,
                    models,
                });
            }
        }

        let items_count = providers.iter().map(|p| p.models.len()).sum();

        Ok(TimelineReport {
            range_label,
            earliest_at: global_earliest,
            latest_at: global_latest,
            total_cost_usd: total_spend,
            total_cache_savings_usd: total_savings,
            total_input_tokens: total_in,
            total_cache_read_tokens: total_read,
            total_output_tokens: total_out,
            total_turns,
            providers,
            items_count,
        })
    }

    /// Queries SQLite storage metrics and file size.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_database_stats(&self) -> Result<DatabaseStats, Error> {
        let total_sessions: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM session", [], |r| r.get(0))
            .unwrap_or(0);
        let total_turns: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM usage_record", [], |r| r.get(0))
            .unwrap_or(0);
        let total_events: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM activity_event", [], |r| r.get(0))
            .unwrap_or(0);
        let total_projects: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM project", [], |r| r.get(0))
            .unwrap_or(0);
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap_or(0);
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(4096);
        let db_size_bytes = u64::try_from((page_count * page_size).max(0)).unwrap_or(0);

        Ok(DatabaseStats {
            total_sessions: u64::try_from(total_sessions.max(0)).unwrap_or(0),
            total_turns: u64::try_from(total_turns.max(0)).unwrap_or(0),
            total_events: u64::try_from(total_events.max(0)).unwrap_or(0),
            total_projects: u64::try_from(total_projects.max(0)).unwrap_or(0),
            db_size_bytes,
        })
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
        assert!(!latest_quota.is_empty());
        assert!(latest_quota.iter().any(|q| q.limit_id == "codex"));

        let saved_cp = db
            .get_checkpoint("/path/to/log.jsonl")
            .expect("get checkpoint")
            .expect("checkpoint exists");
        assert_eq!(saved_cp.last_read_offset, 1234);
    }

    #[test]
    fn custom_pricing_overrides_and_recalculates_cost() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let initial = db.get_custom_model_pricings().expect("query custom prices");
        assert!(initial.is_empty());

        let cp = super::CustomModelPricing {
            model_pattern: "claude-opus".to_owned(),
            input_per_mtok_usd: 10.0,
            output_per_mtok_usd: 50.0,
            cache_read_per_mtok_usd: 1.0,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.set_custom_model_pricing(&cp)
            .expect("set custom pricing");

        let loaded = db.get_custom_model_pricings().expect("query custom prices");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].model_pattern, "claude-opus");
        assert_eq!(loaded[0].input_per_mtok_usd, 10.0);

        db.delete_custom_model_pricing("claude-opus")
            .expect("delete");
        let after_del = db.get_custom_model_pricings().expect("query custom prices");
        assert!(after_del.is_empty());
    }
}
