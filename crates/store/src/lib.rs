//! SQLite storage schema, migrations, queries, and rollups (`SPEC §3.6`, `§4.3`).

use chrono::{DateTime, Utc};
use fleethud_core::rules::{GuardrailRule, ResumeQueueEntry};
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
    /// Invalid stored rule or queue payload.
    #[error("invalid stored JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// A guardrail breach recorded once per rule and session.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RuleAlert {
    /// Session that crossed the threshold.
    pub session_id: String,
    /// Rule that fired.
    pub rule_id: String,
    /// `warn` or `hard_stop`.
    pub action: String,
    /// UTC timestamp.
    pub created_at: String,
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
    /// Total cached input tokens read from Prompt Cache.
    #[serde(default)]
    pub cache_read_tokens: u64,
    /// Latest active context token load.
    #[serde(default)]
    pub context_tokens: u64,
    /// Model context window size in tokens if known.
    #[serde(default)]
    pub context_window: Option<u64>,
    /// Total API-equivalent cost USD.
    pub total_cost_usd: f64,
    /// Session topic or title if known.
    #[serde(default)]
    pub title: Option<String>,
    /// Originating device name (e.g. hostname or "local").
    #[serde(default)]
    pub device_name: Option<String>,
    /// False for presence-only collectors: zero counters do not mean measured zero usage.
    pub usage_available: bool,
    /// Trust level of the API-equivalent cost; unknown when pricing or usage is missing.
    pub cost_basis: TrustLevel,
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

/// User-configured settings for the Pro Labs Context Relay Engine.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextRelayConfig {
    /// Relay execution policy ("ask_first", "auto", "disabled").
    pub policy: String,
    /// Relay routing strategy ("same_agent", "failover").
    pub strategy: String,
    /// Context window saturation percentage threshold to trigger relay (e.g. 85, 90, 95).
    pub threshold_pct: u32,
    /// Timestamp of last modification.
    pub updated_at: String,
}

impl Default for ContextRelayConfig {
    fn default() -> Self {
        Self {
            policy: "ask_first".to_owned(),
            strategy: "same_agent".to_owned(),
            threshold_pct: 90,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Ledger item for a recorded or pending Context Relay event.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContextRelayEntry {
    /// Unique relay event ID.
    pub id: String,
    /// Saturated session ID.
    pub session_id: String,
    /// Source agent slug.
    pub source_agent: String,
    /// Target destination agent slug.
    pub target_agent: String,
    /// Repository project name.
    pub project_name: String,
    /// Working directory path where checkpoint and new session reside.
    pub cwd: String,
    /// Observed context fill percentage at trigger time.
    pub fill_pct: u32,
    /// Current status ("pending", "executed", "dismissed").
    pub status: String,
    /// Timestamp when relay was proposed/recorded.
    pub created_at: String,
    /// Timestamp when relay handoff was executed.
    pub executed_at: Option<String>,
}

/// Managed SQLite database handle wrapped with FleetHUD schema & queries.
pub struct Database {
    conn: Connection,
}

/// Target process or repository path for session window focus (`SPEC §3.13`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFocusTarget {
    /// Agent OS process ID, if tracked.
    pub pid: Option<u32>,
    /// Associated repository root path, if known.
    pub repo_root: Option<String>,
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

        let has_v6: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 6",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v6 {
            self.conn.execute_batch(
                r#"
                UPDATE usage_record SET model = 'Gemini 3.8 Flash (Default)' WHERE model LIKE '%PLACEHOLDER%';
                UPDATE session SET model = 'Gemini 3.8 Flash (Default)' WHERE model LIKE '%PLACEHOLDER%';
                DELETE FROM quota_snapshot WHERE limit_id LIKE '%placeholder%';
                INSERT INTO schema_migrations (version, applied_at) VALUES (6, datetime('now'));
                "#,
            )?;
            let _ = self.reconcile_antigravity_projects();
        }

        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS rule (
                id TEXT PRIMARY KEY,
                payload TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS alert (
                session_id TEXT NOT NULL,
                rule_id TEXT NOT NULL,
                action TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (session_id, rule_id)
            );
            CREATE TABLE IF NOT EXISTS resume_queue (
                session_id TEXT PRIMARY KEY,
                agent TEXT NOT NULL,
                cwd TEXT NOT NULL,
                resume_prompt TEXT NOT NULL,
                max_runs INTEGER NOT NULL,
                runs_completed INTEGER NOT NULL,
                eligible_after TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS context_relay (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                source_agent TEXT NOT NULL,
                target_agent TEXT NOT NULL,
                project_name TEXT NOT NULL,
                cwd TEXT NOT NULL,
                fill_pct INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                executed_at TEXT
            );
            CREATE TABLE IF NOT EXISTS context_relay_config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                policy TEXT NOT NULL DEFAULT 'ask_first',
                strategy TEXT NOT NULL DEFAULT 'same_agent',
                threshold_pct INTEGER NOT NULL DEFAULT 90,
                updated_at TEXT NOT NULL
            );
            "#,
        )?;

        let has_v7: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 7",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v7 {
            self.conn.execute_batch(
                r#"
                UPDATE usage_record
                SET context_tokens = input_tokens + cache_read_tokens
                WHERE cache_read_tokens > 0 AND context_tokens < (input_tokens + cache_read_tokens);
                INSERT INTO schema_migrations (version, applied_at) VALUES (7, datetime('now'));
                "#,
            )?;
        }

        Ok(())
    }

    /// Returns configured local guardrail rules.
    ///
    /// # Errors
    /// Returns [`Error`] for invalid stored JSON or SQLite errors.
    pub fn list_rules(&self) -> Result<Vec<GuardrailRule>, Error> {
        let mut stmt = self.conn.prepare("SELECT payload FROM rule ORDER BY id")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }

    /// Inserts or updates one guardrail rule.
    ///
    /// # Errors
    /// Returns [`Error`] for serialization or SQLite errors.
    pub fn save_rule(&self, rule: &GuardrailRule) -> Result<(), Error> {
        let payload = serde_json::to_string(rule)?;
        self.conn.execute(
            "INSERT INTO rule (id, payload) VALUES (?1, ?2) ON CONFLICT(id) DO UPDATE SET payload=excluded.payload",
            params![rule.id, payload],
        )?;
        Ok(())
    }

    /// Deletes a guardrail rule by ID.
    ///
    /// # Errors
    /// Returns [`Error`] for SQLite errors.
    pub fn delete_rule(&self, id: &str) -> Result<(), Error> {
        self.conn
            .execute("DELETE FROM rule WHERE id = ?1", params![id])?;
        self.conn
            .execute("DELETE FROM alert WHERE rule_id = ?1", params![id])?;
        Ok(())
    }

    /// Returns one session's accumulated cost and turn count.
    ///
    /// # Errors
    /// Returns [`Error`] for SQLite errors.
    pub fn session_guardrail_totals(&self, id: &str) -> Result<(f64, u64), Error> {
        let (cost, turns): (f64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0), COUNT(*) FROM usage_record WHERE session_id = ?1",
            params![id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((cost, turns.max(0) as u64))
    }

    /// Records an alert once for a rule and session; returns whether it was new.
    ///
    /// # Errors
    /// Returns [`Error`] for SQLite errors.
    pub fn insert_alert_once(
        &self,
        session_id: &str,
        rule_id: &str,
        action: &str,
    ) -> Result<bool, Error> {
        let inserted = self.conn.execute(
            "INSERT OR IGNORE INTO alert (session_id, rule_id, action, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, rule_id, action, Utc::now().to_rfc3339()],
        )?;
        Ok(inserted > 0)
    }

    /// Returns recent guardrail alerts.
    ///
    /// # Errors
    /// Returns [`Error`] for SQLite errors.
    pub fn list_alerts(&self, limit: usize) -> Result<Vec<RuleAlert>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, rule_id, action, created_at FROM alert ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(RuleAlert {
                session_id: row.get(0)?,
                rule_id: row.get(1)?,
                action: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Arms a session in the Resume Queue.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn arm_resume_queue(&self, entry: &ResumeQueueEntry) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO resume_queue (session_id, agent, cwd, resume_prompt, max_runs, runs_completed, eligible_after, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(session_id) DO UPDATE SET
                agent = excluded.agent,
                cwd = excluded.cwd,
                resume_prompt = excluded.resume_prompt,
                max_runs = excluded.max_runs,
                runs_completed = excluded.runs_completed,
                eligible_after = excluded.eligible_after
            "#,
            params![
                entry.session_id,
                entry.agent.as_str(),
                entry.cwd,
                entry.resume_prompt,
                entry.max_runs,
                entry.runs_completed,
                entry.eligible_after.to_rfc3339(),
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Disarms a session from the Resume Queue.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn disarm_resume_queue(&self, session_id: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM resume_queue WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    /// Lists all currently armed sessions in the Resume Queue.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn list_resume_queue(&self) -> Result<Vec<ResumeQueueEntry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT session_id, agent, cwd, resume_prompt, max_runs, runs_completed, eligible_after FROM resume_queue ORDER BY eligible_after ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let session_id: String = row.get(0)?;
            let agent_str: String = row.get(1)?;
            let cwd: String = row.get(2)?;
            let resume_prompt: String = row.get(3)?;
            let max_runs: u32 = row.get(4)?;
            let runs_completed: u32 = row.get(5)?;
            let eligible_str: String = row.get(6)?;
            let eligible_after = DateTime::parse_from_rfc3339(&eligible_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
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
            Ok(ResumeQueueEntry {
                session_id,
                agent,
                cwd,
                resume_prompt,
                max_runs,
                runs_completed,
                eligible_after,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Updates the runs completed count for an armed session.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn update_resume_runs(&self, session_id: &str, runs_completed: u32) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE resume_queue SET runs_completed = ?1 WHERE session_id = ?2",
            params![runs_completed, session_id],
        )?;
        Ok(())
    }

    /// Returns current Context Relay Engine configuration.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn get_context_relay_config(&self) -> Result<ContextRelayConfig, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT policy, strategy, threshold_pct, updated_at FROM context_relay_config WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(ContextRelayConfig {
                policy: row.get(0)?,
                strategy: row.get(1)?,
                threshold_pct: row.get(2)?,
                updated_at: row.get(3)?,
            })
        } else {
            let def = ContextRelayConfig::default();
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO context_relay_config (id, policy, strategy, threshold_pct, updated_at) VALUES (1, ?1, ?2, ?3, ?4)",
                params![def.policy, def.strategy, def.threshold_pct, def.updated_at],
            );
            Ok(def)
        }
    }

    /// Saves updated Context Relay Engine configuration.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn save_context_relay_config(&self, cfg: &ContextRelayConfig) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO context_relay_config (id, policy, strategy, threshold_pct, updated_at)
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                policy = excluded.policy,
                strategy = excluded.strategy,
                threshold_pct = excluded.threshold_pct,
                updated_at = excluded.updated_at
            "#,
            params![
                cfg.policy,
                cfg.strategy,
                cfg.threshold_pct,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Lists recent recorded Context Relay events.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn list_context_relays(&self) -> Result<Vec<ContextRelayEntry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, source_agent, target_agent, project_name, cwd, fill_pct, status, created_at, executed_at FROM context_relay ORDER BY created_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ContextRelayEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                source_agent: row.get(2)?,
                target_agent: row.get(3)?,
                project_name: row.get(4)?,
                cwd: row.get(5)?,
                fill_pct: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                executed_at: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Records or upserts a Context Relay event.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn record_context_relay(&self, entry: &ContextRelayEntry) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO context_relay (id, session_id, source_agent, target_agent, project_name, cwd, fill_pct, status, created_at, executed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                executed_at = excluded.executed_at
            "#,
            params![
                entry.id,
                entry.session_id,
                entry.source_agent,
                entry.target_agent,
                entry.project_name,
                entry.cwd,
                entry.fill_pct,
                entry.status,
                entry.created_at,
                entry.executed_at,
            ],
        )?;
        Ok(())
    }

    /// Updates status of a Context Relay event.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn update_context_relay_status(&self, id: &str, status: &str) -> Result<(), Error> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE context_relay SET status = ?1, executed_at = ?2 WHERE id = ?3",
            params![status, now, id],
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

    /// Marks an explicitly registered session as cloud-hosted, including sessions first seen in native logs.
    ///
    /// # Errors
    /// Returns an error if SQLite cannot update the session.
    pub fn mark_cloud_session(&self, id: &str) -> Result<(), Error> {
        self.conn.execute(
            obfstr::obfstr!("UPDATE session SET surface = 'cloud', model = coalesce((SELECT model FROM usage_record WHERE session_id = ?1 ORDER BY timestamp DESC LIMIT 1), model) WHERE id = ?1"),
            [id],
        )?;
        Ok(())
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
                "cloud" => Surface::Cloud,
                _ => Surface::Cli,
            };
            let state_str: String = row.get(7)?;
            let raw_state = match state_str.as_str() {
                "thinking" => SessionState::Thinking,
                "tool" => SessionState::Tool,
                "awaiting_approval" => SessionState::AwaitingApproval,
                "error" => SessionState::Error,
                "ended" => SessionState::Ended,
                "connected" => SessionState::Connected,
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
            let state = if raw_state == SessionState::Connected
                && (now - last_event_at).num_seconds() > 90
            {
                SessionState::Ended
            } else if is_stale
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
                SUM(u.output_tokens) AS out_tok,
                SUM(u.cost_usd) AS total_cost,
                SUM(u.cache_savings_usd) AS total_savings,
                COUNT(u.dedup_key) AS turns
            FROM usage_record u
            JOIN session s ON u.session_id = s.id
            JOIN project p ON s.project_id = p.id
            GROUP BY p.id, s.agent, u.model
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

        out.retain(|q| !q.limit_id.to_lowercase().contains("placeholder"));
        Ok(out)
    }

    /// Reconciles Antigravity session projects by querying the local Antigravity
    /// metadata SQLite database (`~/.gemini/antigravity/conversation_summaries.db`).
    ///
    /// # Errors
    /// Returns [`Error`] if querying or updating the local SQLite store fails.
    pub fn reconcile_antigravity_projects(&mut self) -> Result<usize, Error> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let agy_db_path = home
            .join(".gemini")
            .join("antigravity")
            .join("conversation_summaries.db");

        if !agy_db_path.exists() {
            return Ok(0);
        }

        let agy_conn = match rusqlite::Connection::open_with_flags(
            &agy_db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        ) {
            Ok(c) => c,
            Err(_) => return Ok(0),
        };

        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.id, s.project_id
            FROM session s
            LEFT JOIN project p ON s.project_id = p.id
            WHERE s.agent = 'antigravity'
              AND (p.name = 'unknown' OR p.name IS NULL OR s.title IS NULL OR s.title = '')
            "#,
        )?;

        let sessions: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();

        let mut updated = 0;
        let mut agy_stmt = match agy_conn.prepare(
            "SELECT title, workspace_uris FROM conversation_summaries WHERE conversation_id = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Ok(0),
        };

        for (sid, _curr_pid) in sessions {
            let conv_id = sid.strip_prefix("antigravity:").unwrap_or(&sid);
            if let Ok(mut rows) = agy_stmt.query(params![conv_id]) {
                if let Ok(Some(row)) = rows.next() {
                    let title: Option<String> =
                        row.get(0).ok().filter(|s: &String| !s.trim().is_empty());
                    let uris_json: Option<String> = row.get(1).ok();
                    let mut detected_ws = None;
                    if let Some(raw_uris) = uris_json {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_uris) {
                            if let Some(arr) = val.as_array() {
                                if let Some(first) = arr.first().and_then(|v| v.as_str()) {
                                    let unquoted = first.replace("%3A", ":").replace("%3a", ":");
                                    let clean = unquoted
                                        .strip_prefix("file:///")
                                        .or_else(|| unquoted.strip_prefix("file://"))
                                        .unwrap_or(&unquoted);
                                    detected_ws = Some(clean.to_owned());
                                }
                            }
                        }
                    }

                    if let Some(ws) = detected_ws {
                        let project = fleethud_core::Project::from_origin(None, &ws, None);
                        let _ = self.conn.execute(
                            r#"
                            INSERT INTO project (id, canonical_key, repo_root, name, git_branch)
                            VALUES (?1, ?2, ?3, ?4, ?5)
                            ON CONFLICT(canonical_key) DO UPDATE SET
                                name = excluded.name,
                                repo_root = excluded.repo_root,
                                git_branch = coalesce(excluded.git_branch, project.git_branch)
                            "#,
                            params![
                                project.id,
                                project.canonical_key,
                                project.repo_root,
                                project.name,
                                project.git_branch
                            ],
                        );

                        let proj_id: String = self
                            .conn
                            .query_row(
                                "SELECT id FROM project WHERE canonical_key = ?1",
                                params![project.canonical_key],
                                |r| r.get(0),
                            )
                            .unwrap_or(project.id);

                        let _ = self.conn.execute(
                            "UPDATE session SET project_id = ?1, title = COALESCE(title, ?2) WHERE id = ?3",
                            params![proj_id, title, sid],
                        );
                        updated += 1;
                    } else if title.is_some() {
                        let _ = self.conn.execute(
                            "UPDATE session SET title = COALESCE(title, ?1) WHERE id = ?2",
                            params![title, sid],
                        );
                        updated += 1;
                    }
                }
            }
        }

        Ok(updated)
    }

    /// Queries the workspace path for a specific Antigravity conversation from
    /// `~/.gemini/antigravity/conversation_summaries.db`.
    #[must_use]
    pub fn find_antigravity_workspace(&self, conv_id: &str) -> Option<String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let agy_db = home
            .join(".gemini")
            .join("antigravity")
            .join("conversation_summaries.db");
        if !agy_db.exists() {
            return None;
        }
        let conn = rusqlite::Connection::open_with_flags(
            &agy_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .ok()?;

        let mut stmt = conn
            .prepare("SELECT workspace_uris FROM conversation_summaries WHERE conversation_id = ?1")
            .ok()?;
        let uris: String = stmt.query_row(params![conv_id], |r| r.get(0)).ok()?;
        let val: serde_json::Value = serde_json::from_str(&uris).ok()?;
        let first = val.as_array()?.first()?.as_str()?;
        let unquoted = first.replace("%3A", ":").replace("%3a", ":");
        let clean = unquoted
            .strip_prefix("file:///")
            .or_else(|| unquoted.strip_prefix("file://"))
            .unwrap_or(&unquoted);
        Some(clean.to_owned())
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
        let cutoff = (now - chrono::Duration::minutes(15)).to_rfc3339();
        let agent = agent_filter
            .filter(|value| !value.is_empty())
            .unwrap_or("all");
        let status = status_filter
            .filter(|value| !value.is_empty())
            .unwrap_or("all");
        let total_count: i64 = self.conn.query_row(
            r#"SELECT COUNT(*) FROM session s
               WHERE (?1 = 'all' OR s.agent = ?1)
                 AND (?2 = 'all'
                   OR (?2 = 'active' AND s.state IN ('thinking','tool','awaiting_approval') AND s.last_event_at >= ?3)
                   OR (?2 = 'idle' AND NOT (s.state IN ('thinking','tool','awaiting_approval') AND s.last_event_at >= ?3)))"#,
            params![agent, status, cutoff],
            |row| row.get(0),
        )?;
        let mut stmt = self.conn.prepare(
            r#"
            WITH page AS (
                SELECT s.*, p.name AS project_name,
                    CASE WHEN s.state IN ('thinking','tool','awaiting_approval') AND s.last_event_at >= ?3 THEN 1 ELSE 0 END AS active_rank
                FROM session s JOIN project p ON s.project_id = p.id
                WHERE (?1 = 'all' OR s.agent = ?1)
                  AND (?2 = 'all'
                    OR (?2 = 'active' AND s.state IN ('thinking','tool','awaiting_approval') AND s.last_event_at >= ?3)
                    OR (?2 = 'idle' AND NOT (s.state IN ('thinking','tool','awaiting_approval') AND s.last_event_at >= ?3)))
                ORDER BY active_rank DESC, s.last_event_at DESC
                LIMIT ?4 OFFSET ?5
            )
            SELECT
                s.id, s.agent, s.surface, s.project_name, s.model, s.state, s.started_at, s.last_event_at,
                COALESCE(SUM(u.input_tokens), 0),
                COALESCE(SUM(u.output_tokens), 0),
                COALESCE(SUM(u.cost_usd), 0.0),
                COALESCE(SUM(u.cache_read_tokens), 0),
                COALESCE(
                    -- Codex rows written before the adapter fix stored cumulative context here.
                    (SELECT CASE WHEN s.agent = 'codex' THEN u2.input_tokens + u2.output_tokens ELSE u2.context_tokens END
                     FROM usage_record u2 WHERE u2.session_id = s.id ORDER BY u2.rowid DESC LIMIT 1),
                    0
                ),
                COALESCE(
                    (SELECT u2.context_window FROM usage_record u2 WHERE u2.session_id = s.id ORDER BY u2.rowid DESC LIMIT 1),
                    MAX(u.context_window)
                ),
                s.title,
                COUNT(u.dedup_key),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END)
            FROM page s
            LEFT JOIN usage_record u ON s.id = u.session_id
            GROUP BY s.id
            ORDER BY s.active_rank DESC, s.last_event_at DESC
            "#,
        )?;

        let rows = stmt.query_map(
            params![agent, status, cutoff, limit as i64, offset as i64],
            |row| {
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
                let cache_read_tokens: i64 = row.get(11)?;
                let context_tokens: i64 = row.get(12)?;
                let context_window: Option<i64> = row.get(13)?;
                let title: Option<String> = row.get(14)?;
                let usage_count: i64 = row.get(15)?;
                let unknown_pricing: Option<i64> = row.get(16)?;

                let last_event_at = DateTime::parse_from_rfc3339(&last_event_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or(now);

                let diff_mins = (now - last_event_at).num_minutes();
                let is_recent = diff_mins <= 15;

                let is_active = is_recent
                    && matches!(
                        state_str.as_str(),
                        "thinking" | "tool" | "awaiting_approval"
                    );
                let state = if surface == "cloud" {
                    if (now - last_event_at).num_seconds() <= 90 {
                        "connected"
                    } else {
                        "disconnected"
                    }
                    .to_owned()
                } else if is_active {
                    state_str
                } else {
                    "idle".to_owned()
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
                    surface: surface.clone(),
                    project_name,
                    model,
                    state,
                    is_active,
                    started_at,
                    last_event_at: last_event_at_str,
                    input_tokens: input_tokens.max(0) as u64,
                    output_tokens: output_tokens.max(0) as u64,
                    cache_read_tokens: cache_read_tokens.max(0) as u64,
                    context_tokens: context_tokens.max(0) as u64,
                    context_window: context_window.map(|w| w.max(0) as u64),
                    total_cost_usd,
                    title,
                    device_name: None,
                    usage_available: surface != "cloud" || usage_count > 0,
                    cost_basis: if usage_count == 0 || unknown_pricing.unwrap_or(0) > 0 {
                        TrustLevel::Unknown
                    } else {
                        TrustLevel::Estimated
                    },
                })
            },
        )?;

        let mut filtered = Vec::new();
        for r in rows {
            filtered.push(r?);
        }
        Ok((filtered, total_count.max(0) as usize))
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
                if let Some(f) = from_ts {
                    (Some(f.to_owned()), "Today".to_owned())
                } else {
                    let local_now = chrono::Local::now();
                    let start_of_day = local_now
                        .date_naive()
                        .and_hms_opt(0, 0, 0)
                        .and_then(|dt| dt.and_local_timezone(chrono::Local).earliest())
                        .map(|dt| dt.to_utc())
                        .unwrap_or_else(|| {
                            now.date_naive()
                                .and_hms_opt(0, 0, 0)
                                .map(|dt| dt.and_utc())
                                .unwrap_or(now)
                        });
                    (Some(start_of_day.to_rfc3339()), "Today".to_owned())
                }
            }
            "7d" => {
                if let Some(f) = from_ts {
                    (Some(f.to_owned()), "Last 7 Days".to_owned())
                } else {
                    let d = now - chrono::Duration::days(7);
                    (Some(d.to_rfc3339()), "Last 7 Days".to_owned())
                }
            }
            "30d" => {
                if let Some(f) = from_ts {
                    (Some(f.to_owned()), "Last 30 Days".to_owned())
                } else {
                    let d = now - chrono::Duration::days(30);
                    (Some(d.to_rfc3339()), "Last 30 Days".to_owned())
                }
            }
            "custom" if from_ts.is_some() => {
                (from_ts.map(str::to_owned), "Custom Range".to_owned())
            }
            _ => {
                if let Some(f) = from_ts {
                    (Some(f.to_owned()), "Custom Range".to_owned())
                } else {
                    (None, "All Time".to_owned())
                }
            }
        };

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                p.name AS project_name,
                s.agent AS agent_str,
                u.model,
                SUM(u.input_tokens) AS in_tok,
                SUM(u.cache_read_tokens) AS read_tok,
                SUM(u.output_tokens) AS out_tok,
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
            GROUP BY p.id, s.agent, u.model
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

    /// Returns the session PID and project root path for focus dispatch (`SPEC §3.13`).
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_session_focus_target(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionFocusTarget>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.pid, p.repo_root
            FROM session s
            LEFT JOIN project p ON s.project_id = p.id
            WHERE s.id = ?1
            LIMIT 1
            "#,
        )?;
        let row = stmt
            .query_row(params![session_id], |r| {
                let pid: Option<i64> = r.get(0)?;
                let repo_root: Option<String> = r.get(1)?;
                Ok(SessionFocusTarget {
                    pid: pid.map(|p| p as u32),
                    repo_root,
                })
            })
            .optional()?;
        Ok(row)
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
    use super::{ContextRelayEntry, Database, SourceCheckpoint};
    use chrono::Utc;
    use fleethud_core::rules::{GuardrailRule, RuleAction};
    use fleethud_core::{
        ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
        SessionState, Surface, TrustLevel, UsageRecord,
    };

    #[test]
    fn guardrail_rules_and_alerts_persist_without_duplicates() {
        let db = Database::open_in_memory().expect("in-memory db");
        let rule = GuardrailRule {
            id: "cost-cap".to_owned(),
            project_id: None,
            agent: None,
            max_spend_usd: Some(5.0),
            max_turns: None,
            action: RuleAction::HardStop,
        };
        db.save_rule(&rule).expect("save rule");
        assert_eq!(db.list_rules().expect("list rules"), vec![rule]);
        assert!(
            db.insert_alert_once("claude-code:s1", "cost-cap", "hard_stop")
                .expect("first alert")
        );
        assert!(
            !db.insert_alert_once("claude-code:s1", "cost-cap", "hard_stop")
                .expect("duplicate alert")
        );
        assert_eq!(db.list_alerts(10).expect("list alerts").len(), 1);
        db.delete_rule("cost-cap").expect("delete rule");
        assert!(db.list_rules().expect("list rules").is_empty());
        assert!(db.list_alerts(10).expect("list alerts").is_empty());
    }

    #[test]
    fn resume_queue_persists_arms_and_disarms() {
        use fleethud_core::rules::ResumeQueueEntry;
        let db = Database::open_in_memory().expect("in-memory db");
        let entry = ResumeQueueEntry {
            session_id: "claude-code:session-1".to_owned(),
            agent: AgentId::ClaudeCode,
            cwd: "/workspace/my-app".to_owned(),
            resume_prompt: "continue".to_owned(),
            max_runs: 1,
            runs_completed: 0,
            eligible_after: Utc::now(),
        };
        db.arm_resume_queue(&entry).expect("arm queue");
        let list = db.list_resume_queue().expect("list queue");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_id, "claude-code:session-1");
        db.update_resume_runs("claude-code:session-1", 1)
            .expect("update runs");
        let list2 = db.list_resume_queue().expect("list queue 2");
        assert_eq!(list2[0].runs_completed, 1);
        db.disarm_resume_queue("claude-code:session-1")
            .expect("disarm");
        assert!(db.list_resume_queue().expect("empty queue").is_empty());
    }

    #[test]
    fn context_relay_config_and_ledger_persist() {
        let db = Database::open_in_memory().expect("in-memory db");
        let def = db.get_context_relay_config().expect("default config");
        assert_eq!(def.policy, "ask_first");
        assert_eq!(def.threshold_pct, 90);

        let mut updated = def.clone();
        updated.policy = "auto".to_owned();
        updated.threshold_pct = 85;
        db.save_context_relay_config(&updated).expect("save config");
        let loaded = db.get_context_relay_config().expect("loaded config");
        assert_eq!(loaded.policy, "auto");
        assert_eq!(loaded.threshold_pct, 85);

        let entry = ContextRelayEntry {
            id: "relay-1".to_owned(),
            session_id: "claude-code:s1".to_owned(),
            source_agent: "claude-code".to_owned(),
            target_agent: "claude-code".to_owned(),
            project_name: "fleethud".to_owned(),
            cwd: "/workspace/fleethud".to_owned(),
            fill_pct: 92,
            status: "pending".to_owned(),
            created_at: Utc::now().to_rfc3339(),
            executed_at: None,
        };
        db.record_context_relay(&entry).expect("record relay");
        let list = db.list_context_relays().expect("list relays");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "relay-1");
        assert_eq!(list[0].status, "pending");

        db.update_context_relay_status("relay-1", "executed")
            .expect("update status");
        let list2 = db.list_context_relays().expect("list relays 2");
        assert_eq!(list2[0].status, "executed");
        assert!(list2[0].executed_at.is_some());
    }

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
        db.commit_batch(&[], &[], std::slice::from_ref(&u), &[], &[], &cp)
            .expect("idempotent second commit");

        let active = db.list_active_sessions(10).expect("list active");
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "claude-code:s1");

        let (recent, active_count) = db
            .list_sessions_paginated(None, Some("active"), 0, 10)
            .expect("list recent sessions");
        assert_eq!(active_count, 0);
        assert!(recent.is_empty());

        let (all_recent, total_recent) = db
            .list_sessions_paginated(None, None, 0, 10)
            .expect("list all sessions paginated");
        assert_eq!(total_recent, 1);
        assert_eq!(all_recent[0].title.as_deref(), Some("Implement M1"));

        let report = db.generate_usage_report().expect("report");
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].turn_count, 1);
        assert_eq!(report[0].input_tokens, 1000);

        let other_project = Project::from_origin(
            Some("https://github.com/other/fleethud"),
            "/workspace/other/fleethud",
            None,
        );
        let mut other_session = db.list_active_sessions(1).expect("session").remove(0);
        other_session.id = "claude-code:s2".to_owned();
        other_session.project_id = other_project.id.clone();
        let mut other_usage = u.clone();
        other_usage.dedup_key = "msg2:req2".to_owned();
        other_usage.session_id = other_session.id.clone();
        db.commit_batch(
            &[other_project],
            &[other_session],
            &[other_usage],
            &[],
            &[],
            &cp,
        )
        .expect("second project");
        assert_eq!(db.generate_usage_report().expect("report").len(), 2);

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
    fn codex_session_context_uses_latest_turn_for_previously_stored_rows() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(None, "/workspace/sample", None);
        let session = Session {
            id: "codex:synthetic".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::DesktopApp,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-6-luna".to_owned()),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let usage = UsageRecord {
            dedup_key: "synthetic:1".to_owned(),
            session_id: session.id.clone(),
            timestamp: Utc::now(),
            model: "gpt-6-luna".to_owned(),
            input_tokens: 189_000,
            output_tokens: 1_000,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 313_700_000,
            context_window: Some(258_000),
            cost_usd: 0.0,
            cache_savings_usd: 0.0,
            cost_basis: TrustLevel::Exact,
            pricing_version: "test".to_owned(),
        };
        let checkpoint = SourceCheckpoint {
            file_path: "/synthetic/rollout.jsonl".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "synthetic".to_owned(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch(&[project], &[session], &[usage], &[], &[], &checkpoint)
            .expect("commit synthetic Codex usage");

        let (sessions, count) = db
            .list_sessions_paginated(None, None, 0, 10)
            .expect("query Codex session");
        assert_eq!(count, 1);
        assert_eq!(sessions[0].context_tokens, 190_000);
        assert_eq!(sessions[0].context_window, Some(258_000));
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

    #[test]
    fn usage_report_timeline_honors_explicit_from_ts_and_local_start_of_day() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let p = Project::from_origin(
            Some("https://github.com/mwahaj36/fleethud"),
            "/workspace/fleethud",
            Some("main"),
        );
        let s = Session {
            id: "antigravity:s1".to_owned(),
            agent: AgentId::Antigravity,
            surface: Surface::Ide,
            project_id: p.id.clone(),
            parent_session_id: None,
            model: Some("gemini-2.5-pro".to_owned()),
            title: Some("Date filtering test".to_owned()),
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };

        // Record from 2 days ago
        let two_days_ago = Utc::now() - chrono::Duration::days(2);
        let u_old = UsageRecord {
            dedup_key: "old:1".to_owned(),
            session_id: s.id.clone(),
            timestamp: two_days_ago,
            model: "gemini-2.5-pro".to_owned(),
            input_tokens: 1000,
            output_tokens: 100,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 1100,
            context_window: Some(1_000_000),
            cost_usd: 10.0,
            cache_savings_usd: 0.0,
            cost_basis: TrustLevel::Exact,
            pricing_version: "2026-10-26".to_owned(),
        };

        // Record from now (today)
        let now = Utc::now();
        let u_now = UsageRecord {
            dedup_key: "now:1".to_owned(),
            session_id: s.id.clone(),
            timestamp: now,
            model: "gemini-2.5-pro".to_owned(),
            input_tokens: 500,
            output_tokens: 50,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 550,
            context_window: Some(1_000_000),
            cost_usd: 5.0,
            cache_savings_usd: 0.0,
            cost_basis: TrustLevel::Exact,
            pricing_version: "2026-10-26".to_owned(),
        };

        let cp = SourceCheckpoint {
            file_path: "/path/to/log.jsonl".to_owned(),
            last_read_offset: 100,
            file_size: 100,
            fingerprint: "hash".to_owned(),
            last_ingested_at: Utc::now(),
        };

        db.commit_batch(&[p], &[s], &[u_old, u_now], &[], &[], &cp)
            .expect("commit batch");

        // 1. "all" should return total 15.0
        let rep_all = db
            .generate_usage_report_timeline("all", None, None)
            .expect("report all");
        assert_eq!(rep_all.total_cost_usd, 15.0);

        // 2. "today" should include u_now (5.0) and exclude u_old
        let rep_today = db
            .generate_usage_report_timeline("today", None, None)
            .expect("report today");
        assert_eq!(rep_today.total_cost_usd, 5.0);

        // 3. "today" with explicit from_ts overriding cutoff to before u_old should return 15.0
        let explicit_from = (two_days_ago - chrono::Duration::hours(1)).to_rfc3339();
        let rep_custom_today = db
            .generate_usage_report_timeline("today", Some(&explicit_from), None)
            .expect("report explicit from");
        assert_eq!(rep_custom_today.total_cost_usd, 15.0);

        // 4. "today" with to_ts before u_now should return 0.0
        let before_now = (now - chrono::Duration::seconds(5)).to_rfc3339();
        let rep_to = db
            .generate_usage_report_timeline("today", None, Some(&before_now))
            .expect("report with to_ts");
        assert_eq!(rep_to.total_cost_usd, 0.0);
    }
}
