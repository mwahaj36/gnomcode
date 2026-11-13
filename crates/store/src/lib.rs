//! SQLite storage schema, migrations, queries, and rollups (`SPEC §3.6`, `§4.3`).

use chrono::{DateTime, Datelike, Utc};
use gnomcode_core::rules::{GuardrailRule, ResumeQueueEntry};
use gnomcode_core::{
    ActivityEvent, ActivityKind, AgentId, ChatEvent, ChatRole, Project, QuotaSnapshot, QuotaWindow,
    RoutingPolicy, Session, SessionState, Surface, TrustLevel, UsageRecord,
};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::Path;
use thiserror::Error;

mod automation;
mod conversation;
mod trajectory;
pub use automation::{AutomationNotice, PendingPushNotice, ScheduledMessage};
pub use conversation::ConversationLaunch;
pub use trajectory::{TrajectoryRows, read_trajectory, trajectory_max_generation};

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

fn event_marker(id: &str) -> Vec<u8> {
    Sha256::digest(id.as_bytes()).to_vec()
}

fn mark_deleted<P: rusqlite::Params>(
    tx: &rusqlite::Transaction<'_>,
    query: &str,
    params: P,
) -> Result<(), Error> {
    let mut statement = tx.prepare(query)?;
    let ids = statement.query_map(params, |row| row.get::<_, String>(0))?;
    let mut insert = tx.prepare_cached(obfstr::obfstr!(
        "INSERT OR IGNORE INTO deleted_chat_event(event_id) VALUES (?1)"
    ))?;
    for id in ids {
        insert.execute([event_marker(&id?)])?;
    }
    Ok(())
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
    /// Priced invocation count.
    #[serde(default)]
    pub priced_turns: u64,
    /// Unpriced invocation count; cost is partial when nonzero.
    #[serde(default)]
    pub unpriced_turns: u64,
    /// Measured session tokens with cache counted once.
    #[serde(default)]
    pub total_tokens: Option<u64>,
    /// Latest context fill percentage, calculated in Rust.
    #[serde(default)]
    pub context_fill_pct: Option<f64>,
    /// Session ID.
    pub id: String,
    /// Agent name.
    pub agent: String,
    /// Surface.
    pub surface: String,
    /// Product name when the surface is a separate product (`Antigravity IDE`).
    #[serde(default)]
    pub surface_label: Option<String>,
    /// Project name.
    pub project_name: String,
    /// Model name.
    pub model: String,
    /// State (tool, thinking, awaiting_approval, idle, ended).
    pub state: String,
    /// Whether execution or an approval wait is established by source observations.
    pub is_active: bool,
    /// Running, approval-waiting, or idle within the two-minute grace period.
    #[serde(default)]
    pub is_ongoing: bool,
    /// Source-independent presence classification.
    #[serde(default)]
    pub presence: gnomcode_core::presence::SessionPresence,
    /// Home board group derived from presence; absent in payloads from older daemons.
    #[serde(default)]
    pub home_group: Option<gnomcode_core::presence::HomeGroup>,
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
    /// Parent session ID if this is a subagent session.
    #[serde(default)]
    pub parent_session_id: Option<String>,
    /// Originating device name (e.g. hostname or "local").
    #[serde(default)]
    pub device_name: Option<String>,
    /// Whether this session has a native usage source.
    pub usage_available: bool,
    /// Trust level of the API-equivalent cost; unknown when pricing or usage is missing.
    pub cost_basis: TrustLevel,
    /// Total subagent sessions linked to this session.
    #[serde(default)]
    pub subagent_count: usize,
    /// Linked subagents that are running, waiting for approval or briefly idle.
    #[serde(default)]
    pub subagents_ongoing: usize,
    /// Most recent linked subagents (capped), newest first.
    #[serde(default)]
    pub subagents: Vec<SubagentItem>,
}

/// Maximum nested subagents returned per session; the count still covers all of them.
pub const SUBAGENT_LIST_LIMIT: usize = 50;

/// Lean row for a subagent nested under its parent session.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubagentItem {
    /// Session ID.
    pub id: String,
    /// Subagent title if known.
    pub title: Option<String>,
    /// Model name, or "Unspecified".
    pub model: String,
    /// Stored state, "unknown" when a stale open execution can no longer be trusted.
    pub state: String,
    /// Source-independent presence classification.
    pub presence: gnomcode_core::presence::SessionPresence,
    /// Whether execution or an approval wait is established by source observations.
    pub is_active: bool,
    /// Running, approval-waiting or recently idle.
    pub is_ongoing: bool,
    /// Home board group derived from presence.
    pub home_group: gnomcode_core::presence::HomeGroup,
    /// Started at timestamp.
    pub started_at: String,
    /// Last event timestamp.
    pub last_event_at: String,
}

/// Breakdown for a single model under a provider.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ModelReport {
    /// Priced invocation count.
    #[serde(default)]
    pub priced_turns: u64,
    /// Unpriced invocation count; costs are partial when nonzero.
    #[serde(default)]
    pub unpriced_turns: u64,
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
    /// Priced invocation count.
    #[serde(default)]
    pub priced_turns: u64,
    /// Unpriced invocation count; costs are partial when nonzero.
    #[serde(default)]
    pub unpriced_turns: u64,
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
pub struct DailyUsage {
    /// Invocations with a recorded pricing basis.
    #[serde(default)]
    pub priced_turns: u64,
    /// Invocations whose price is unknown; recorded costs are a partial subtotal.
    #[serde(default)]
    pub unpriced_turns: u64,
    /// UTC date of measured usage.
    pub date: String,
    /// Tokens including cache exactly once.
    pub total_tokens: u64,
    /// Recorded API-equivalent cost.
    pub cost_usd: f64,
    /// Pricing provenance, unknown if any record is unpriced.
    pub cost_basis: TrustLevel,
    /// Measured invocations.
    pub turns: u64,
    /// Bar height relative to the busiest recorded day, computed in Rust.
    pub chart_pct: f64,
    /// Activity intensity 0..=4 for grid cells, computed in Rust.
    #[serde(default)]
    pub level: u8,
}

/// Intensity level 0..=4 for `value` relative to `max`, square-root scaled so outliers do not flatten the grid.
pub(crate) fn activity_level(value: u64, max: u64) -> u8 {
    if value == 0 || max == 0 {
        return 0;
    }
    let ratio = (value as f64 / max as f64).sqrt();
    ((ratio * 4.0).ceil() as u8).clamp(1, 4)
}

fn group_activity(rows: Vec<DailyUsage>, range: &str) -> (Vec<DailyUsage>, &'static str) {
    let bounds = rows.first().zip(rows.last()).and_then(|(first, last)| {
        Some((
            chrono::NaiveDate::parse_from_str(first.date.get(..10)?, "%Y-%m-%d").ok()?,
            chrono::NaiveDate::parse_from_str(last.date.get(..10)?, "%Y-%m-%d").ok()?,
        ))
    });
    let monday = |date: chrono::NaiveDate| {
        date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday()))
    };
    let unit = if range == "today" {
        "hour"
    } else if range != "all" || bounds.is_none_or(|(start, end)| (end - start).num_days() < 30) {
        "day"
    } else if bounds.is_some_and(|(start, end)| (monday(end) - monday(start)).num_weeks() < 30) {
        "week"
    } else if bounds.is_some_and(|(start, end)| {
        (end.year() - start.year()) * 12 + end.month() as i32 - (start.month() as i32) < 30
    }) {
        "month"
    } else {
        "year"
    };
    if matches!(unit, "day" | "hour") {
        return (rows, unit);
    }
    let mut groups = std::collections::BTreeMap::<String, DailyUsage>::new();
    for mut row in rows {
        let key = if unit == "week" {
            chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d")
                .map(|date| monday(date).to_string())
                .unwrap_or_else(|_| row.date.clone())
        } else {
            row.date
                .get(..if unit == "month" { 7 } else { 4 })
                .unwrap_or(&row.date)
                .to_owned()
        };
        row.date = match unit {
            "week" => key.clone(),
            "month" => format!("{key}-01"),
            _ => format!("{key}-01-01"),
        };
        groups
            .entry(key)
            .and_modify(|group| {
                group.total_tokens = group.total_tokens.saturating_add(row.total_tokens);
                group.turns = group.turns.saturating_add(row.turns);
                group.priced_turns = group.priced_turns.saturating_add(row.priced_turns);
                group.unpriced_turns = group.unpriced_turns.saturating_add(row.unpriced_turns);
                group.cost_usd += row.cost_usd;
                if row.cost_basis == TrustLevel::Unknown {
                    group.cost_basis = TrustLevel::Unknown;
                }
            })
            .or_insert(row);
    }
    (groups.into_values().collect(), unit)
}

/// One UTC day in the activity grid.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HeatmapDay {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    /// Tokens including cache exactly once.
    pub total_tokens: u64,
    /// Recorded API-equivalent cost.
    pub cost_usd: f64,
    /// Pricing provenance.
    pub cost_basis: TrustLevel,
    /// Measured invocations.
    pub turns: u64,
    /// Intensity 0..=4.
    pub level: u8,
}

/// Summary figures for the activity grid.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HeatmapStats {
    /// Tokens over days since the first recording.
    pub total_tokens: u64,
    /// Days with recorded tokens.
    pub active_days: u32,
    /// Busiest recorded day, earliest on ties.
    pub busiest: Option<HeatmapBusiest>,
    /// Consecutive active days ending today (or yesterday when today is empty).
    pub current_streak: u32,
    /// Longest run of consecutive active days.
    pub longest_streak: u32,
}

/// Busiest recorded day.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct HeatmapBusiest {
    /// UTC date `YYYY-MM-DD`.
    pub date: String,
    /// Tokens on that day.
    pub total_tokens: u64,
}

/// Dense daily activity for the trailing year, starting on a Monday.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ActivityHeatmap {
    /// First date (a Monday).
    pub from: String,
    /// Last date (today, UTC).
    pub to: String,
    /// Earliest UTC date with any recorded usage ever.
    pub first_recorded: Option<String>,
    /// One entry per day from `from` to `to`.
    pub days: Vec<HeatmapDay>,
    /// Summary figures over days since `first_recorded`.
    pub stats: HeatmapStats,
}

/// Summary figures over `days` on or after `first_recorded`; all zero when nothing is recorded.
pub(crate) fn heatmap_stats(
    days: &[HeatmapDay],
    first_recorded: Option<chrono::NaiveDate>,
    today: chrono::NaiveDate,
) -> HeatmapStats {
    let mut stats = HeatmapStats {
        total_tokens: 0,
        active_days: 0,
        busiest: None,
        current_streak: 0,
        longest_streak: 0,
    };
    let Some(first) = first_recorded else {
        return stats;
    };
    let mut run = 0u32;
    let mut recorded = Vec::new();
    for day in days {
        let Ok(date) = chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d") else {
            continue;
        };
        if date < first || date > today {
            continue;
        }
        stats.total_tokens = stats.total_tokens.saturating_add(day.total_tokens);
        let active = day.total_tokens > 0;
        if active {
            stats.active_days += 1;
            run += 1;
            stats.longest_streak = stats.longest_streak.max(run);
            if stats
                .busiest
                .as_ref()
                .is_none_or(|best| day.total_tokens > best.total_tokens)
            {
                stats.busiest = Some(HeatmapBusiest {
                    date: day.date.clone(),
                    total_tokens: day.total_tokens,
                });
            }
        } else {
            run = 0;
        }
        recorded.push((date, active));
    }
    let mut backwards = recorded.iter().rev().peekable();
    if backwards
        .peek()
        .is_some_and(|(date, active)| *date == today && !*active)
    {
        backwards.next();
    }
    stats.current_streak = backwards.take_while(|(_, active)| *active).count() as u32;
    stats
}

/// Local-only project usage rollup; repository identity is not published to cloud.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProjectUsage {
    /// Priced invocation count.
    #[serde(default)]
    pub priced_turns: u64,
    /// Unpriced invocation count; cost is partial when nonzero.
    #[serde(default)]
    pub unpriced_turns: u64,
    /// Stable canonical project ID.
    pub id: String,
    /// Local project display name.
    pub name: String,
    /// Measured tokens with cache counted once.
    pub total_tokens: u64,
    /// API-equivalent spend.
    pub cost_usd: f64,
    /// Pricing trust; unknown if any invocation is unpriced.
    pub cost_basis: TrustLevel,
    /// Number of sessions with recorded usage in the selected period.
    pub sessions: u64,
    /// Local folder of the project; empty for shared scratch buckets. Never synced to cloud.
    #[serde(default)]
    pub repo_root: String,
    /// Timestamp of the latest recorded usage in the selected period.
    #[serde(default)]
    pub last_active: Option<String>,
    /// Agent identifiers with recorded usage in the selected period.
    #[serde(default)]
    pub agents: Vec<String>,
}

/// Filtered timeline report with date spans and per-provider ROI comparison.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelineReport {
    /// Invocations with a recorded pricing basis.
    #[serde(default)]
    pub priced_turns: u64,
    /// Invocations whose price is unknown; recorded costs are a partial subtotal.
    #[serde(default)]
    pub unpriced_turns: u64,
    /// Aggregate pricing trust; unknown when any included invocation is unpriced.
    #[serde(default = "unknown_cost_basis")]
    pub cost_basis: TrustLevel,
    /// Share of measured input tokens served from read cache.
    #[serde(default)]
    pub cache_share_pct: Option<f64>,
    /// Daily UTC measurements for charts; missing dates are not fabricated zeros.
    #[serde(default)]
    pub daily: Vec<DailyUsage>,
    /// Chart bucket unit: hour, day, ISO calendar week, month or year.
    #[serde(default)]
    pub activity_interval: String,
    /// Measured tokens with provider-specific cache accounting applied once.
    #[serde(default)]
    pub total_tokens: u64,
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

fn unknown_cost_basis() -> TrustLevel {
    TrustLevel::Unknown
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

/// Managed SQLite database handle wrapped with Gnomcode schema & queries.
pub struct Database {
    conn: Connection,
}

/// Target process or repository path for session window focus (`SPEC §3.13`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionFocusTarget {
    /// Agent identifier for application targeting.
    pub agent: String,
    /// Recorded session surface; distinguishes desktop apps from terminals.
    pub surface: String,
    /// Agent OS process ID, if tracked.
    pub pid: Option<u32>,
    /// Associated repository root path, if known.
    pub repo_root: Option<String>,
}

impl Database {
    /// Reads the last observed activity for incremental parsing.
    ///
    /// # Errors
    /// Returns an error if the state cannot be read.
    pub fn session_state(&self, id: &str) -> Result<Option<SessionState>, Error> {
        let state: Option<String> = self
            .conn
            .query_row(
                obfstr::obfstr!("SELECT state FROM session WHERE id=?1"),
                [id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(state.and_then(|state| match state.as_str() {
            "idle" => Some(SessionState::Idle),
            "thinking" => Some(SessionState::Thinking),
            "tool" => Some(SessionState::Tool),
            "awaiting_approval" => Some(SessionState::AwaitingApproval),
            "error" => Some(SessionState::Error),
            "ended" => Some(SessionState::Ended),
            _ => None,
        }))
    }
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

            CREATE TABLE IF NOT EXISTS quota_pool_member (
                provider TEXT NOT NULL,
                limit_id TEXT NOT NULL,
                member TEXT NOT NULL,
                PRIMARY KEY (provider, limit_id, member)
            );

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

        // The folder a session itself ran in; projects can merge folders, so it is stored per session.
        if !self
            .conn
            .prepare("SELECT 1 FROM pragma_table_info('session') WHERE name = 'cwd'")?
            .exists([])?
        {
            self.conn
                .execute("ALTER TABLE session ADD COLUMN cwd TEXT", [])?;
        }

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
                UPDATE usage_record SET model = 'unknown' WHERE model LIKE '%PLACEHOLDER%';
                UPDATE session SET model = NULL WHERE model LIKE '%PLACEHOLDER%';
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

        let has_v8: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE version = 8",
                [],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !has_v8 {
            self.conn.execute_batch(
                r#"
                CREATE TABLE IF NOT EXISTS chat_event (
                    event_id TEXT PRIMARY KEY,
                    session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
                    turn_index INTEGER NOT NULL,
                    role TEXT NOT NULL,
                    content TEXT NOT NULL,
                    tool_name TEXT,
                    status TEXT,
                    created_at TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_chat_event_session ON chat_event(session_id, turn_index);
                INSERT INTO schema_migrations (version, applied_at) VALUES (8, datetime('now'));
                "#,
            )?;
        }

        self.conn.execute_batch(obfstr::obfstr!(r#"
            CREATE TABLE IF NOT EXISTS conversation_member (
                host_id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                native_id TEXT UNIQUE,
                position INTEGER NOT NULL,
                needs_context INTEGER NOT NULL DEFAULT 0,
                agent TEXT, repo TEXT, requested_model TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_conversation_member ON conversation_member(conversation_id, position);
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (9, datetime('now'));
        "#))?;
        self.conn.execute_batch(obfstr::obfstr!(r#"
            CREATE TABLE IF NOT EXISTS deleted_chat_event (event_id BLOB PRIMARY KEY);
            INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES (10, datetime('now'));
        "#))?;
        self.conn.execute_batch(obfstr::obfstr!(
            r#"
            CREATE TABLE IF NOT EXISTS scheduled_message (
                id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, text TEXT NOT NULL,
                due_at INTEGER NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL, attempted_at INTEGER, finished_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_scheduled_due ON scheduled_message(status,due_at);
            CREATE TABLE IF NOT EXISTS automation_notice (
                id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, message TEXT NOT NULL,
                created_at INTEGER NOT NULL, notified INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (12,datetime('now'));
        "#
        ))?;
        let push_column_exists: bool = self.conn.query_row(
            obfstr::obfstr!("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=14)"),
            [],
            |row| row.get(0),
        )?;
        if !push_column_exists {
            self.conn.execute_batch(obfstr::obfstr!("BEGIN; ALTER TABLE automation_notice ADD COLUMN pushed INTEGER NOT NULL DEFAULT 0; INSERT INTO schema_migrations(version,applied_at) VALUES (14,datetime('now')); COMMIT;"))?;
        }
        let v15_exists: bool = self.conn.query_row(
            obfstr::obfstr!("SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version=15)"),
            [],
            |row| row.get(0),
        )?;
        if !v15_exists {
            self.conn.execute_batch(obfstr::obfstr!(
                r#"BEGIN;
                CREATE TABLE IF NOT EXISTS notification_settings (
                    id INTEGER PRIMARY KEY CHECK (id = 1),
                    approval_waiting INTEGER NOT NULL DEFAULT 1,
                    turn_finished INTEGER NOT NULL DEFAULT 1,
                    turn_finished_min_minutes INTEGER NOT NULL DEFAULT 2,
                    limit_low INTEGER NOT NULL DEFAULT 1,
                    limit_reset INTEGER NOT NULL DEFAULT 1,
                    updated_at TEXT NOT NULL
                );
                ALTER TABLE automation_notice ADD COLUMN push_payload TEXT;
                INSERT INTO schema_migrations(version,applied_at) VALUES (15,datetime('now'));
                COMMIT;"#
            ))?;
        }
        self.conn.execute_batch(obfstr::obfstr!(
            "CREATE TABLE IF NOT EXISTS desktop_preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL);"
        ))?;
        self.conn.execute_batch(obfstr::obfstr!(r#"
            DELETE FROM usage_record
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM activity_event
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM chat_event
            WHERE session_id IN (
                SELECT id FROM session
                WHERE agent = 'codex'
                  AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%')
            );
            DELETE FROM session
            WHERE agent = 'codex'
              AND (id LIKE 'codex:ctc_%' OR id LIKE 'codex:fc_%' OR id LIKE 'codex:ctco_%' OR id LIKE 'codex:call_%' OR id LIKE 'codex:rs_%' OR id LIKE 'codex:msg_%');

            -- Clean up self-referencing subagents where background tasks set parent to self
            UPDATE session SET parent_session_id = NULL WHERE parent_session_id = id;
            UPDATE session SET title = NULL WHERE title = 'Subagent' AND parent_session_id IS NULL;
        "#))?;
        Ok(())
    }

    /// Checks whether pre-format-fix chat needs a one-time source repair.
    ///
    /// # Errors
    /// Returns [`Error`] if the maintenance marker cannot be read.
    pub fn chat_format_needs_repair(&self) -> Result<bool, Error> {
        Ok(self.conn.query_row(obfstr::obfstr!("SELECT NOT EXISTS(SELECT 1 FROM schema_migrations WHERE version=13) AND EXISTS(SELECT 1 FROM session)"), [], |row| row.get(0))?)
    }

    /// Repairs existing session metadata from native logs without replaying usage or deleted chats.
    ///
    /// # Errors
    /// Returns an error if metadata cannot be updated atomically.
    pub fn repair_session_metadata(
        &self,
        projects: &[Project],
        sessions: &[Session],
    ) -> Result<(), Error> {
        let tx = self.conn.unchecked_transaction()?;
        for project in projects {
            tx.execute(obfstr::obfstr!("INSERT OR IGNORE INTO project(id,canonical_key,repo_root,name,git_branch) VALUES (?1,?2,?3,?4,?5)"),params![project.id,project.canonical_key,project.repo_root,project.name,project.git_branch])?;
        }
        for session in sessions {
            tx.execute(obfstr::obfstr!("UPDATE session SET project_id=?2,parent_session_id=coalesce(?3,parent_session_id),surface=?4 WHERE id=?1"),params![session.id,session.project_id,session.parent_session_id,session.surface.as_str()])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Restores content for an existing event without inserting deleted history.
    ///
    /// # Errors
    /// Returns [`Error`] if the existing row cannot be updated.
    pub fn repair_chat_contents(&self, events: &[ChatEvent]) -> Result<(), Error> {
        let transaction = self.conn.unchecked_transaction()?;
        {
            let mut previous = transaction.prepare(obfstr::obfstr!(
                "SELECT content,role FROM chat_event WHERE event_id=?1 AND session_id=?2"
            ))?;
            let mut statement = transaction.prepare(obfstr::obfstr!(
                "UPDATE chat_event SET content=?1 WHERE event_id=?2 AND session_id=?3"
            ))?;
            for event in events {
                let stored: Option<(String, String)> = previous
                    .query_row(params![event.event_id, event.session_id], |row| {
                        Ok((row.get(0)?, row.get(1)?))
                    })
                    .optional()?;
                let Some((old, role)) = stored else {
                    continue;
                };
                if role != event.role.as_str() {
                    continue;
                }
                let content = if event.role == ChatRole::Tool {
                    String::new()
                } else {
                    gnomcode_core::redact::redact_chat_content(&event.content)
                };
                let old = if event.role == ChatRole::User {
                    gnomcode_core::redact::clean_user_message(&old)
                } else {
                    old
                };
                let old = gnomcode_core::redact::redact_chat_content(&old);
                if !old.split_whitespace().eq(content.split_whitespace()) {
                    continue;
                }
                statement.execute(params![content, event.event_id, event.session_id])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Records completion of the idempotent local chat-format repair.
    ///
    /// # Errors
    /// Returns [`Error`] if the maintenance marker cannot be saved.
    pub fn finish_chat_format_repair(&self) -> Result<(), Error> {
        self.conn.execute(obfstr::obfstr!("INSERT OR IGNORE INTO schema_migrations(version,applied_at) VALUES (11,datetime('now')),(13,datetime('now'))"), [])?;
        Ok(())
    }

    /// Inserts a local redacted structured chat event.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite insert failure.
    pub fn insert_chat_event(&self, event: &ChatEvent) -> Result<(), Error> {
        let created_at = event.created_at.to_rfc3339();
        self.conn.execute(
            obfstr::obfstr!(r#"
            INSERT INTO chat_event (event_id, session_id, turn_index, role, content, tool_name, status, created_at)
            SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8
            WHERE NOT EXISTS (SELECT 1 FROM deleted_chat_event WHERE event_id=?9)
            ON CONFLICT(event_id) DO UPDATE SET
                content = excluded.content,
                tool_name = excluded.tool_name,
                status = excluded.status
            "#),
            params![
                event.event_id,
                event.session_id,
                event.turn_index,
                event.role.as_str(),
                if event.role == ChatRole::Tool { String::new() } else { gnomcode_core::redact::redact_chat_content(&event.content) },
                event.tool_name.as_deref().map(gnomcode_core::redact::redact_chat_content),
                event.status.as_deref().map(gnomcode_core::redact::redact_chat_content),
                created_at,
                event_marker(&event.event_id),
            ],
        )?;
        Ok(())
    }

    /// Reads the native label before bounded opening-message fallback candidates.
    ///
    /// # Errors
    /// Returns [`Error`] if the query fails.
    pub fn conversation_title_candidates(&self, session_id: &str) -> Result<Vec<String>, Error> {
        let mut titles: Vec<String> = self.conn.query_row(obfstr::obfstr!("SELECT title FROM session WHERE id=?1 AND title IS NOT NULL AND trim(title)<>''"), [session_id], |row| row.get(0)).optional()?.into_iter().collect();
        let mut statement = self.conn.prepare(obfstr::obfstr!("SELECT substr(content, 1, 8192) FROM chat_event WHERE session_id = ?1 AND role = 'user' ORDER BY turn_index, created_at, event_id LIMIT 16"))?;
        let rows = statement.query_map(params![session_id], |row| row.get(0))?;
        titles.extend(rows.collect::<Result<Vec<_>, _>>()?);
        Ok(titles)
    }

    /// Updates a native app's redacted label without changing activity timestamps.
    ///
    /// # Errors
    /// Returns an error if SQLite cannot update existing session metadata.
    pub fn update_native_session_title(&self, session_id: &str, title: &str) -> Result<(), Error> {
        let title: String = gnomcode_core::redact::redact_chat_content(title)
            .trim()
            .chars()
            .take(256)
            .collect();
        if !title.is_empty() {
            self.conn.execute(
                obfstr::obfstr!("UPDATE session SET title=?2 WHERE id=?1 AND title IS NOT ?2"),
                params![session_id, title],
            )?;
        }
        Ok(())
    }

    /// Retrieves the most recent structured chat events for a session in turn order.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_session_chat(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<ChatEvent>, Error> {
        self.get_session_chat_before(session_id, limit, None)
    }

    /// Reads a persisted handoff notice by its target host identity.
    ///
    /// # Errors
    /// Returns [`Error`] if SQLite cannot read the notice.
    pub fn handoff_notice(&self, host: &str) -> Result<Option<ChatEvent>, Error> {
        let id = format!("handoff:{host}:source");
        let session: Option<String> = self
            .conn
            .query_row(
                obfstr::obfstr!("SELECT session_id FROM chat_event WHERE event_id=?1"),
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        match session {
            Some(session) => Ok(self
                .get_session_chat(&session, None)?
                .into_iter()
                .find(|event| event.event_id == id)),
            None => Ok(None),
        }
    }

    /// Retrieves one page of a session's chat before an event ID, newest page first.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_session_chat_before(
        &self,
        session_id: &str,
        limit: Option<usize>,
        before: Option<&str>,
    ) -> Result<Vec<ChatEvent>, Error> {
        let limit = limit.unwrap_or(1000).min(5000);
        let mut stmt = self.conn.prepare(
            r#"
            SELECT event_id, session_id, turn_index, role, content, tool_name, status, created_at
            FROM chat_event
            WHERE session_id = ?1
              AND (?3 IS NULL OR (turn_index, created_at, event_id) < (
                SELECT turn_index, created_at, event_id FROM chat_event
                WHERE session_id = ?1 AND event_id = ?3
              ))
            ORDER BY turn_index DESC, created_at DESC, event_id DESC
            LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![session_id, limit as i64, before], |row| {
            let event_id: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let turn_index: i64 = row.get(2)?;
            let role_str: String = row.get(3)?;
            let content: String = row.get(4)?;
            let tool_name: Option<String> = row.get(5)?;
            let status: Option<String> = row.get(6)?;
            let created_at_str: String = row.get(7)?;

            let role = ChatRole::parse_role(&role_str).unwrap_or(ChatRole::System);
            let content = if role == ChatRole::Tool {
                String::new()
            } else {
                gnomcode_core::redact::redact_chat_content(&content)
            };
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(ChatEvent {
                event_id,
                session_id,
                turn_index,
                role,
                content,
                tool_name,
                status,
                created_at,
            })
        })?;

        let mut events = Vec::new();
        for r in rows {
            events.push(r?);
        }
        events.reverse();
        Ok(events)
    }

    /// Builds a handoff packet from a known session and its latest chat events.
    ///
    /// # Errors
    /// Returns [`Error`] when the session or its history cannot be queried.
    pub fn prepare_handoff(
        &self,
        session_id: &str,
        target_model: &str,
        touched_files: Vec<String>,
    ) -> Result<Option<gnomcode_core::HandoffPacket>, Error> {
        let source_model: Option<Option<String>> = self
            .conn
            .query_row(
                "SELECT model FROM session WHERE id = ?1",
                params![session_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(source_model) = source_model else {
            return Ok(None);
        };
        let events = self.get_session_chat(session_id, Some(50))?;
        Ok(Some(gnomcode_core::HandoffPacket {
            session_id: session_id.to_owned(),
            source_model: source_model.unwrap_or_else(|| "unknown".to_owned()),
            target_model: target_model.to_owned(),
            prompt_summary: gnomcode_core::handoff_summary(&events),
            touched_files,
            turn_count: u32::try_from(events.len()).unwrap_or(u32::MAX),
            created_at: Utc::now(),
        }))
    }

    /// Hard-deletes all stored conversation events for a session.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite deletion failure.
    pub fn delete_session_chat(&self, session_id: &str) -> Result<usize, Error> {
        self.cancel_conversation_schedules(session_id)?;
        let tx = self.conn.unchecked_transaction()?;
        mark_deleted(
            &tx,
            obfstr::obfstr!("SELECT event_id FROM chat_event WHERE session_id=?1"),
            [session_id],
        )?;
        let deleted = tx.execute(
            obfstr::obfstr!("DELETE FROM chat_event WHERE session_id=?1"),
            [session_id],
        )?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Deletes a conversation and its owned handoff notices atomically.
    ///
    /// # Errors
    /// Returns [`Error`] if SQLite cannot delete both identities.
    pub fn delete_session_chat_with_host(
        &self,
        session_id: &str,
        host_id: &str,
    ) -> Result<usize, Error> {
        if self.conversation_target(host_id)?.is_some() {
            return self.delete_conversation_chat(host_id);
        }
        self.cancel_conversation_schedules(session_id)?;
        self.cancel_conversation_schedules(host_id)?;
        let tx = self.conn.unchecked_transaction()?;
        let notice = format!("handoff:{host_id}:source");
        mark_deleted(
            &tx,
            obfstr::obfstr!(
                "SELECT event_id FROM chat_event WHERE session_id IN (?1, ?2) OR event_id=?3"
            ),
            params![session_id, host_id, notice],
        )?;
        let deleted = tx.execute(
            obfstr::obfstr!("DELETE FROM chat_event WHERE session_id IN (?1, ?2) OR event_id=?3"),
            params![session_id, host_id, notice],
        )?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Prunes chat events older than the specified retention window in days.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite deletion failure.
    pub fn purge_chat_events_older_than(&self, days: u32) -> Result<usize, Error> {
        let tx = self.conn.unchecked_transaction()?;
        let cutoff: f64 = tx.query_row(
            obfstr::obfstr!("SELECT julianday('now', '-' || ?1 || ' days')"),
            [days],
            |row| row.get(0),
        )?;
        mark_deleted(
            &tx,
            obfstr::obfstr!("SELECT event_id FROM chat_event WHERE julianday(created_at) < ?1"),
            [cutoff],
        )?;
        let deleted = tx.execute(
            obfstr::obfstr!("DELETE FROM chat_event WHERE julianday(created_at) < ?1"),
            [cutoff],
        )?;
        tx.commit()?;
        Ok(deleted)
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
                "continue",
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
        let root = self.conversation_id(session_id)?;
        self.conn.execute(obfstr::obfstr!("DELETE FROM resume_queue WHERE session_id=?1 OR session_id=?2 OR session_id IN (SELECT host_id FROM conversation_member WHERE conversation_id=?2 UNION SELECT native_id FROM conversation_member WHERE conversation_id=?2)"),params![session_id,root])?;
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
            // Rows from unknown or removed agents stay stored but are never relabeled.
            let Some(agent) = AgentId::parse(&agent_str) else {
                return Ok(None);
            };
            Ok(Some(ResumeQueueEntry {
                session_id,
                agent,
                cwd,
                resume_prompt,
                max_runs,
                runs_completed,
                eligible_after,
            }))
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.extend(row?);
        }
        Ok(out)
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

    /// Returns the current global routing and failover policy.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn get_routing_policy(&self) -> Result<RoutingPolicy, Error> {
        let cfg = self.get_context_relay_config()?;
        Ok(match cfg.policy.as_str() {
            "auto" | "automatic" => RoutingPolicy::Automatic,
            "deny_cross_provider" => RoutingPolicy::DenyCrossProvider,
            _ => RoutingPolicy::AskFirst,
        })
    }

    /// Sets and saves the global routing and failover policy.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn save_routing_policy(&self, policy: RoutingPolicy) -> Result<(), Error> {
        let mut cfg = self.get_context_relay_config().unwrap_or_default();
        cfg.policy = policy.as_str().to_owned();
        cfg.updated_at = Utc::now().to_rfc3339();
        self.save_context_relay_config(&cfg)
    }

    /// Returns whether automatic background desktop update checks are enabled (default `true`).
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_auto_update_enabled(&self) -> Result<bool, Error> {
        let val: Option<String> = self
            .conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT value FROM desktop_preferences WHERE key = 'auto_update_enabled'"
                ),
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(val.as_deref() != Some("0"))
    }

    /// Persists the opt-out automatic desktop update check preference.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write failure.
    pub fn set_auto_update_enabled(&self, enabled: bool) -> Result<(), Error> {
        self.conn.execute(
            obfstr::obfstr!(
                "INSERT INTO desktop_preferences(key, value) VALUES ('auto_update_enabled', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value"
            ),
            [if enabled { "1" } else { "0" }],
        )?;
        Ok(())
    }

    /// Returns the E3 off-by-default feature toggles (all `false` when unset).
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_feature_toggles(&self) -> Result<gnomcode_core::FeatureToggles, Error> {
        let val: Option<String> = self
            .conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT value FROM desktop_preferences WHERE key = 'feature_toggles'"
                ),
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(val
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default())
    }

    /// Persists the E3 feature toggles in `desktop_preferences`.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write or JSON serialization failure.
    pub fn set_feature_toggles(
        &self,
        toggles: &gnomcode_core::FeatureToggles,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(toggles)?;
        self.conn.execute(
            obfstr::obfstr!(
                "INSERT INTO desktop_preferences(key, value) VALUES ('feature_toggles', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value"
            ),
            [json],
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
        self.commit_batch_with_chat(projects, sessions, usage, activity, quota, checkpoint, &[])
    }

    /// Atomically commits parsed events, local conversation and the checkpoint.
    ///
    /// # Errors
    /// Returns [`Error`] if any insert or checkpoint update fails.
    #[allow(clippy::too_many_arguments)]
    pub fn commit_batch_with_chat(
        &mut self,
        projects: &[Project],
        sessions: &[Session],
        usage: &[UsageRecord],
        activity: &[ActivityEvent],
        quota: &[QuotaSnapshot],
        checkpoint: &SourceCheckpoint,
        chat: &[ChatEvent],
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
            let mut stmt = tx.prepare_cached(obfstr::obfstr!(
                r#"
                INSERT INTO session (
                    id, agent, surface, project_id, parent_session_id,
                    model, title, state, pid, started_at, last_event_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                    parent_session_id = coalesce(excluded.parent_session_id,session.parent_session_id),
                    project_id = CASE WHEN (SELECT name FROM project WHERE id=session.project_id)='unknown' AND (SELECT name FROM project WHERE id=excluded.project_id)!='unknown' THEN excluded.project_id ELSE session.project_id END,
                    surface = CASE WHEN excluded.surface <> 'cli' THEN excluded.surface ELSE session.surface END,
                    model = coalesce(excluded.model, session.model),
                    title = coalesce(excluded.title, session.title),
                    state = excluded.state,
                    pid = coalesce(excluded.pid, session.pid),
                    last_event_at = excluded.last_event_at
                "#
            ))?;
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

        for event in chat {
            let content = if event.role == ChatRole::Tool {
                String::new()
            } else {
                gnomcode_core::redact::redact_chat_content(&event.content)
            };
            tx.execute(obfstr::obfstr!("INSERT OR IGNORE INTO chat_event (event_id, session_id, turn_index, role, content, tool_name, status, created_at) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8 WHERE NOT EXISTS (SELECT 1 FROM deleted_chat_event WHERE event_id=?9)"), params![event.event_id, event.session_id, event.turn_index, event.role.as_str(), content, event.tool_name.as_deref().map(gnomcode_core::redact::redact_chat_content), event.status.as_deref().map(gnomcode_core::redact::redact_chat_content), event.created_at.to_rfc3339(), event_marker(&event.event_id)])?;
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

    /// Lists recorded sessions with source state, ordered by latest activity.
    /// Use the paginated ongoing projection for live presence and badge counts.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn list_active_sessions(&self, limit: usize) -> Result<Vec<Session>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, agent, surface, project_id, parent_session_id, model, title, state, pid, started_at, last_event_at
            FROM session
            WHERE surface <> 'cloud' AND id NOT GLOB 'codex:ctc_*' AND id NOT GLOB 'codex:ctco_*' AND id NOT GLOB 'codex:fc_*' AND id NOT GLOB 'codex:call_*' AND id NOT GLOB 'codex:rs_*' AND id NOT GLOB 'codex:msg_*'
            ORDER BY last_event_at DESC
            LIMIT ?1
            "#,
        )?;

        let limit_i64 = i64::try_from(limit).unwrap_or(50);
        let rows = stmt.query_map(params![limit_i64], |row| {
            let agent_str: String = row.get(1)?;
            // Rows from unknown or removed agents stay stored but are never relabeled.
            let Some(agent) = AgentId::parse(&agent_str) else {
                return Ok(None);
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

            let state = raw_state;
            let raw_model: Option<String> = row.get(5)?;
            let model = match raw_model {
                Some(ref model) if model.contains("MODEL_PLACEHOLDER") => None,
                other => other,
            };

            Ok(Some(Session {
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
            }))
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.extend(r?);
        }
        Ok(out)
    }

    /// Aggregates usage report grouped by project, agent, and model for `gnomcode report`.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn generate_usage_report(&self) -> Result<Vec<ReportRow>, Error> {
        let mut stmt = self.conn.prepare(obfstr::obfstr!(
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
            WHERE s.surface <> 'cloud'
            GROUP BY p.id, s.agent, u.model
            ORDER BY total_cost DESC
            "#
        ))?;

        let rows = stmt.query_map([], |row| {
            let agent_str: String = row.get(1)?;
            // Rows from unknown or removed agents stay stored but are never relabeled.
            let Some(agent) = AgentId::parse(&agent_str) else {
                return Ok(None);
            };
            let in_tok: i64 = row.get(3)?;
            let read_tok: i64 = row.get(4)?;
            let out_tok: i64 = row.get(5)?;
            let turn_count: i64 = row.get(8)?;

            Ok(Some(ReportRow {
                project_name: row.get(0)?,
                agent,
                model: row.get(2)?,
                input_tokens: u64::try_from(in_tok.max(0)).unwrap_or(0),
                cache_read_tokens: u64::try_from(read_tok.max(0)).unwrap_or(0),
                output_tokens: u64::try_from(out_tok.max(0)).unwrap_or(0),
                total_cost_usd: row.get(6)?,
                total_cache_savings_usd: row.get(7)?,
                turn_count: u64::try_from(turn_count.max(0)).unwrap_or(0),
            }))
        })?;

        let custom_prices = self.get_custom_model_pricings().unwrap_or_default();
        let mut out = Vec::new();
        for r in rows {
            let Some(mut row) = r? else { continue };
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
                basis: match row.get::<_, String>(6)?.as_str() {
                    "exact" => TrustLevel::Exact,
                    "estimated" => TrustLevel::Estimated,
                    "api" => TrustLevel::Api,
                    _ => TrustLevel::Unknown,
                },
                observed_at,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }

        out.retain(|q| !q.limit_id.to_lowercase().contains("placeholder"));
        self.apply_antigravity_resource_exhausted_crosscheck(&mut out)?;
        Ok(out)
    }

    /// Replaces `provider`'s pool membership (`(pool, model name)` pairs) as reported by its limits
    /// source. An empty list keeps what is stored, since a failed read says nothing about pools.
    ///
    /// # Errors
    /// Returns [`Error`] if the transaction fails.
    pub fn replace_quota_pool_members(
        &mut self,
        provider: &str,
        members: &[(String, String)],
    ) -> Result<(), Error> {
        if members.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM quota_pool_member WHERE provider = ?1",
            params![provider],
        )?;
        for (pool, member) in members {
            tx.execute(
                "INSERT OR IGNORE INTO quota_pool_member (provider, limit_id, member) VALUES (?1, ?2, ?3)",
                params![provider, pool, member],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Applies each recorded `RESOURCE_EXHAUSTED` error to the pool of the model that raised it.
    ///
    /// The model is mapped to a pool from the limits source's own data (the pool name and its
    /// listed models). Exactly one pool: its unreset windows read 100% used, `exact`. No pool or
    /// several: no reading is changed and an `estimated` exhausted hint for that model is added.
    fn apply_antigravity_resource_exhausted_crosscheck(
        &self,
        snapshots: &mut Vec<QuotaSnapshot>,
    ) -> Result<(), Error> {
        let mut stmt = self.conn.prepare(
            "SELECT a.tool_name, MAX(a.timestamp) FROM activity_event a
             JOIN session s ON s.id = a.session_id
             WHERE s.agent = 'antigravity' AND a.kind = 'rate_limited'
             GROUP BY a.tool_name",
        )?;
        let errors: Vec<(Option<String>, DateTime<Utc>)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(Result::ok)
            .filter_map(|(model, at)| {
                let at = DateTime::parse_from_rfc3339(&at).ok()?.with_timezone(&Utc);
                Some((model, at))
            })
            .collect();
        if errors.is_empty() {
            return Ok(());
        }
        let mut members = self.conn.prepare(
            "SELECT limit_id, member FROM quota_pool_member WHERE provider = 'antigravity'",
        )?;
        let members: Vec<(String, String)> = members
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();
        // Added rows are never newer than the provider's newest reading, so the API does not
        // treat them as a fresher report and supersede the other pools.
        let newest = snapshots
            .iter()
            .filter(|snap| snap.provider == "antigravity")
            .map(|snap| snap.observed_at)
            .max();
        let now = Utc::now();

        for (model, rl_dt) in errors {
            let mut pools: Vec<&str> = Vec::new();
            if let Some(model) = model.as_deref() {
                for snap in snapshots
                    .iter()
                    .filter(|snap| snap.provider == "antigravity")
                {
                    let pool = snap.limit_id.as_str();
                    let covers = gnomcode_core::quota::model_matches_pool_name(model, pool)
                        || members.iter().any(|(id, member)| {
                            id == pool
                                && gnomcode_core::quota::model_matches_pool_name(model, member)
                        });
                    if covers && !pools.contains(&pool) {
                        pools.push(pool);
                    }
                }
            }
            if let [pool] = pools[..] {
                let pool = pool.to_owned();
                for snap in snapshots
                    .iter_mut()
                    .filter(|snap| snap.provider == "antigravity" && snap.limit_id == pool)
                {
                    let unreset = snap.resets_at.is_none_or(|reset| rl_dt < reset);
                    let recent = rl_dt >= snap.observed_at - chrono::Duration::hours(5);
                    if snap.used_pct < 100.0 && unreset && recent {
                        snap.used_pct = 100.0;
                        snap.basis = TrustLevel::Exact;
                        snap.observed_at =
                            snap.observed_at.max(newest.map_or(rl_dt, |n| rl_dt.min(n)));
                    }
                }
            } else if rl_dt > now - chrono::Duration::hours(5) {
                snapshots.push(QuotaSnapshot {
                    provider: "antigravity".to_owned(),
                    limit_id: model.unwrap_or_else(|| "antigravity".to_owned()),
                    window: QuotaWindow::FiveHour,
                    used_pct: 100.0,
                    resets_at: None,
                    plan: None,
                    basis: TrustLevel::Estimated,
                    observed_at: newest.map_or(rl_dt, |n| rl_dt.min(n)),
                });
            }
        }
        Ok(())
    }

    /// Every stored project, so derived identity can be recomputed.
    ///
    /// # Errors
    /// Returns [`Error`] if the query fails.
    pub fn list_projects(&self) -> Result<Vec<gnomcode_core::Project>, Error> {
        let mut statement = self
            .conn
            .prepare("SELECT id, canonical_key, repo_root, name, git_branch FROM project")?;
        let rows = statement.query_map([], |row| {
            Ok(gnomcode_core::Project {
                id: row.get(0)?,
                canonical_key: row.get(1)?,
                repo_root: row.get(2)?,
                name: row.get(3)?,
                git_branch: row.get(4)?,
            })
        })?;
        rows.collect::<Result<_, _>>().map_err(Error::from)
    }

    /// Records the folder a session itself ran in. Resume and launch use this, never a merged
    /// project's last observed folder.
    ///
    /// # Errors
    /// Returns [`Error`] if the update fails.
    pub fn record_session_cwd(&self, session_id: &str, cwd: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE session SET cwd = ?2 WHERE id = ?1 AND cwd IS NOT ?2",
            params![session_id, cwd],
        )?;
        Ok(())
    }

    /// Gives sessions of `project_id` that have no recorded folder the project's folder. Only valid
    /// before the project is merged: an unmerged project is one folder, so it is each session's own.
    ///
    /// # Errors
    /// Returns [`Error`] if the update fails.
    pub fn backfill_session_cwd(&self, project_id: &str, cwd: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE session SET cwd = ?2 WHERE project_id = ?1 AND cwd IS NULL",
            params![project_id, cwd],
        )?;
        Ok(())
    }

    /// Moves every session of project `old_id` to `project`, merging into an existing row with
    /// the same canonical key. Only derived project fields change; sessions, usage and chat are kept.
    ///
    /// # Errors
    /// Returns [`Error`] if the transaction fails.
    pub fn reassign_project(
        &mut self,
        old_id: &str,
        project: &gnomcode_core::Project,
    ) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        let target: Option<String> = tx
            .query_row(
                "SELECT id FROM project WHERE canonical_key = ?1",
                params![project.canonical_key],
                |row| row.get(0),
            )
            .optional()?;
        match target {
            Some(id) if id == old_id => {
                tx.execute(
                    "UPDATE project SET name = ?2, repo_root = ?3 WHERE id = ?1",
                    params![old_id, project.name, project.repo_root],
                )?;
            }
            Some(id) => {
                tx.execute(
                    "UPDATE session SET project_id = ?1 WHERE project_id = ?2",
                    params![id, old_id],
                )?;
                tx.execute("DELETE FROM project WHERE id = ?1", params![old_id])?;
            }
            None => {
                tx.execute(
                    "INSERT INTO project (id, canonical_key, repo_root, name, git_branch) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        project.id,
                        project.canonical_key,
                        project.repo_root,
                        project.name,
                        project.git_branch
                    ],
                )?;
                tx.execute(
                    "UPDATE session SET project_id = ?1 WHERE project_id = ?2",
                    params![project.id, old_id],
                )?;
                tx.execute("DELETE FROM project WHERE id = ?1", params![old_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reconciles Antigravity session projects by querying the local Antigravity
    /// metadata SQLite database (`~/.gemini/antigravity/conversation_summaries.db`).
    ///
    /// # Errors
    /// Returns [`Error`] if querying or updating the local SQLite store fails.
    pub fn reconcile_antigravity_projects(&mut self) -> Result<usize, Error> {
        let gemini_base = std::env::var("GEMINI_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map_or_else(
                || {
                    let home = std::env::var("USERPROFILE")
                        .or_else(|_| std::env::var("HOME"))
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|_| std::path::PathBuf::from("."));
                    home.join(".gemini")
                },
                std::path::PathBuf::from,
            );

        let agy_db_path = gemini_base
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

        // Antigravity's own summaries are authoritative for subagent links; repair any drift.
        if let Ok(mut links) = agy_conn
            .prepare("SELECT conversation_id, parent_conversation_id FROM conversation_summaries")
        {
            let rows: Vec<(String, String)> = links
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .filter_map(Result::ok)
                .collect();
            for (conversation, parent) in rows {
                let parent = (!parent.trim().is_empty()).then(|| format!("antigravity:{parent}"));
                self.conn.execute(
                    "UPDATE session SET parent_session_id = ?1 WHERE id = ?2 AND parent_session_id IS NOT ?1",
                    params![parent, format!("antigravity:{conversation}")],
                )?;
            }
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT s.id, p.name
            FROM session s
            LEFT JOIN project p ON s.project_id = p.id
            WHERE s.agent = 'antigravity'
              AND (p.name = 'unknown' OR p.name IS NULL OR s.title IS NULL OR s.title = '')
            "#,
        )?;

        let sessions: Vec<(String, Option<String>)> = stmt
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

        for (sid, current_project) in sessions {
            let conv_id = sid.strip_prefix("antigravity:").unwrap_or(&sid);
            if let Ok(mut rows) = agy_stmt.query(params![conv_id])
                && let Ok(Some(row)) = rows.next()
            {
                let title: Option<String> =
                    row.get(0).ok().filter(|s: &String| !s.trim().is_empty());
                let uris_json: Option<String> = row.get(1).ok();
                let mut detected_ws = None;
                if let Some(raw_uris) = uris_json
                    && let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw_uris)
                    && let Some(arr) = val.as_array()
                    && let Some(first) = arr.first().and_then(|v| v.as_str())
                {
                    let unquoted = first.replace("%3A", ":").replace("%3a", ":");
                    let clean = unquoted
                        .strip_prefix("file:///")
                        .or_else(|| unquoted.strip_prefix("file://"))
                        .unwrap_or(&unquoted);
                    detected_ws = Some(clean.to_owned());
                }

                // A session that already has a project keeps it; ingest resolves it to the repo root.
                let unassigned = current_project
                    .as_deref()
                    .is_none_or(|name| name == "unknown");
                if let Some(ws) = detected_ws.filter(|_| unassigned) {
                    let project = gnomcode_core::Project::from_origin(None, &ws, None);
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

        Ok(updated)
    }

    /// Queries the workspace path for a specific Antigravity conversation from
    /// the IDE and CLI summary databases under `GEMINI_HOME` (default `~/.gemini`).
    #[must_use]
    pub fn find_antigravity_workspace(&self, conv_id: &str) -> Option<String> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));

        let root = std::env::var_os("GEMINI_HOME")
            .map_or_else(|| home.join(".gemini"), std::path::PathBuf::from);
        antigravity_workspace(&root, conv_id)
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
            if let Some(price) = detected_openai
                && ((profile.openai_plan_usd - 0.0).abs() < f64::EPSILON
                    || ((profile.openai_plan_usd - 20.0).abs() < f64::EPSILON && price > 20.0))
            {
                profile.openai_plan_usd = price;
                changed = true;
            }
        }

        if let Some(price) = anthropic_usd
            && (profile.claude_plan_usd - 0.0).abs() < f64::EPSILON
        {
            profile.claude_plan_usd = price;
            changed = true;
        }

        if let Some(price) = google_usd
            && (profile.google_plan_usd - 0.0).abs() < f64::EPSILON
        {
            profile.google_plan_usd = price;
            changed = true;
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
        self.list_sessions_filtered(agent_filter, status_filter, None, offset, limit)
    }

    /// Lists sessions with an optional exact local project ID before pagination.
    ///
    /// # Errors
    /// Returns an error on SQLite failure.
    pub fn list_sessions_filtered(
        &self,
        agent_filter: Option<&str>,
        status_filter: Option<&str>,
        project_filter: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SessionListItem>, usize), Error> {
        let now = Utc::now();
        let cutoff = (now
            - chrono::Duration::milliseconds(gnomcode_core::presence::RECENT_ACTIVITY_GRACE_MS))
        .to_rfc3339();
        let stale_cutoff = (now
            - chrono::Duration::milliseconds(gnomcode_core::presence::APPROVAL_WAIT_LIMIT_MS))
        .to_rfc3339();
        // Open tool/model executions use the shorter limit so counts match presence classification.
        let execution_cutoff = (now
            - chrono::Duration::milliseconds(gnomcode_core::presence::OPEN_EXECUTION_LIMIT_MS))
        .to_rfc3339();
        let agent = agent_filter
            .filter(|value| !value.is_empty())
            .unwrap_or("all");
        let status = status_filter
            .filter(|value| !value.is_empty())
            .unwrap_or("all");
        // A parent with a live subagent is itself ongoing; this mirrors the child presence rules.
        let child_clause = |approval: &str, exec: &str| {
            format!(
                "EXISTS (SELECT 1 FROM session c WHERE c.parent_session_id = s.id AND c.id <> s.id AND (c.state = 'awaiting_approval' AND julianday(c.last_event_at) >= julianday({approval}) OR (c.state IN ('thinking','tool') AND julianday(c.last_event_at) >= julianday({exec}))))"
            )
        };
        let child_count = child_clause("?5", "?6");
        let child_page = child_clause("?7", "?8");
        let total_count: i64 = self.conn.query_row(
            &format!(r#"SELECT COUNT(*) FROM session s
               WHERE (?4 IS NULL OR s.project_id = ?4) AND s.surface <> 'cloud' AND s.parent_session_id IS NULL AND s.id NOT GLOB 'codex:ctc_*' AND s.id NOT GLOB 'codex:ctco_*' AND s.id NOT GLOB 'codex:fc_*' AND s.id NOT GLOB 'codex:call_*' AND s.id NOT GLOB 'codex:rs_*' AND s.id NOT GLOB 'codex:msg_*' AND (?1 = 'all' OR s.agent = ?1)
                 AND (?2 = 'all'
                   OR (?2 = 'ended' AND s.state = 'ended')
                   OR (?2 = 'awaiting_approval' AND s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5))
                   OR (?2 = 'ongoing' AND ({child_count} OR (s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?6))) OR (s.state = 'idle' AND julianday(s.last_event_at) >= julianday(?3))))
                   OR (?2 = 'active' AND ({child_count} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?6))))
                   OR (?2 = 'idle' AND s.state = 'idle'))"#),
            params![agent, status, cutoff, project_filter, stale_cutoff, execution_cutoff],
            |row| row.get(0),
        )?;
        let mut stmt = self.conn.prepare(
            &format!(r#"
            WITH page AS (
                SELECT s.*, p.name AS project_name,
                    CASE WHEN ({child_page} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))) THEN 1 ELSE 0 END AS active_rank
                FROM session s JOIN project p ON s.project_id = p.id
                WHERE (?4 IS NULL OR s.project_id = ?4) AND s.surface <> 'cloud' AND s.parent_session_id IS NULL AND s.id NOT GLOB 'codex:ctc_*' AND s.id NOT GLOB 'codex:ctco_*' AND s.id NOT GLOB 'codex:fc_*' AND s.id NOT GLOB 'codex:call_*' AND s.id NOT GLOB 'codex:rs_*' AND s.id NOT GLOB 'codex:msg_*' AND (?1 = 'all' OR s.agent = ?1)
                  AND (?2 = 'all'
                    OR (?2 = 'ended' AND s.state = 'ended')
                   OR (?2 = 'awaiting_approval' AND s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7))
                   OR (?2 = 'ongoing' AND ({child_page} OR (s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))) OR (s.state = 'idle' AND julianday(s.last_event_at) >= julianday(?3))))
                   OR (?2 = 'active' AND ({child_page} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))))
                    OR (?2 = 'idle' AND s.state = 'idle'))
                ORDER BY active_rank DESC, s.last_event_at DESC
                LIMIT ?5 OFFSET ?6
            )
            SELECT
                s.id, s.agent, s.surface, s.project_name, s.model, s.state, s.started_at, s.last_event_at,
                COALESCE(SUM(u.input_tokens), 0),
                COALESCE(SUM(u.output_tokens), 0),
                COALESCE(SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END), 0.0),
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
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END),
                COALESCE(SUM(u.input_tokens + u.output_tokens + CASE WHEN s.agent IN ('codex', 'gemini-cli') THEN 0
                    ELSE u.cache_read_tokens + u.cache_write_5m_tokens + u.cache_write_1h_tokens END), 0),
                (SELECT kind FROM activity_event ae WHERE ae.session_id = s.id ORDER BY julianday(ae.timestamp) DESC, ae.id DESC LIMIT 1),
                s.parent_session_id
            FROM page s
            LEFT JOIN usage_record u ON s.id = u.session_id
            GROUP BY s.id
            ORDER BY s.active_rank DESC, s.last_event_at DESC
            "#),
        )?;

        let rows = stmt.query_map(
            params![
                agent,
                status,
                cutoff,
                project_filter,
                limit as i64,
                offset as i64,
                stale_cutoff,
                execution_cutoff
            ],
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

                let age = DateTime::parse_from_rfc3339(&last_event_at_str)
                    .ok()
                    .map(|date| (now - date.with_timezone(&Utc)).num_milliseconds());
                let latest_kind: Option<String> = row.get(18)?;
                let open_execution = matches!(
                    (state_str.as_str(), latest_kind.as_deref()),
                    ("thinking", Some("prompt")) | ("tool", Some("tool_start"))
                );
                let presence =
                    gnomcode_core::presence::classify_presence(&state_str, age, open_execution);
                let is_ongoing = presence.is_ongoing();
                let is_active = presence.is_active();
                let state = if presence == gnomcode_core::presence::SessionPresence::Inactive
                    && matches!(state_str.as_str(), "thinking" | "tool")
                {
                    "unknown".to_owned()
                } else {
                    state_str
                };

                let model = match raw_model {
                    Some(ref m) if m.contains("MODEL_PLACEHOLDER") => "Unspecified".to_owned(),
                    Some(m) if !m.trim().is_empty() => m,
                    _ => "Unspecified".to_owned(),
                };

                Ok(SessionListItem {
                    priced_turns: (usage_count - unknown_pricing.unwrap_or(0)).max(0) as u64,
                    unpriced_turns: unknown_pricing.unwrap_or(0).max(0) as u64,
                    total_tokens: if usage_count > 0 {
                        Some(row.get::<_, i64>(17)?.max(0) as u64)
                    } else {
                        None
                    },
                    context_fill_pct: context_window
                        .filter(|window| *window > 0)
                        .filter(|_| usage_count > 0)
                        .map(|window| {
                            (context_tokens.max(0) as f64 / window as f64 * 100.0).clamp(0.0, 100.0)
                        }),
                    surface_label: gnomcode_core::surface_label(&agent, &surface)
                        .map(str::to_owned),
                    id,
                    agent,
                    surface: surface.clone(),
                    project_name,
                    model,
                    state,
                    is_active,
                    is_ongoing,
                    presence,
                    home_group: Some(presence.home_group()),
                    started_at,
                    last_event_at: last_event_at_str,
                    input_tokens: input_tokens.max(0) as u64,
                    output_tokens: output_tokens.max(0) as u64,
                    cache_read_tokens: cache_read_tokens.max(0) as u64,
                    context_tokens: context_tokens.max(0) as u64,
                    context_window: context_window.map(|w| w.max(0) as u64),
                    total_cost_usd,
                    title,
                    parent_session_id: row.get(19)?,
                    device_name: None,
                    usage_available: true,
                    cost_basis: if usage_count == 0 || unknown_pricing.unwrap_or(0) > 0 {
                        TrustLevel::Unknown
                    } else {
                        TrustLevel::Estimated
                    },
                    subagent_count: 0,
                    subagents_ongoing: 0,
                    subagents: Vec::new(),
                })
            },
        )?;

        let mut filtered = Vec::new();
        for r in rows {
            filtered.push(r?);
        }
        self.attach_subagents(&mut filtered, now)?;
        Ok((filtered, total_count.max(0) as usize))
    }

    /// Nests each session's subagents and lifts a parent with a live subagent into the
    /// ongoing groups, since the parent's own log is quiet while a subagent works.
    fn attach_subagents(
        &self,
        items: &mut [SessionListItem],
        now: DateTime<Utc>,
    ) -> Result<(), Error> {
        use gnomcode_core::presence::{HomeGroup, SessionPresence, classify_presence};
        if items.is_empty() {
            return Ok(());
        }
        let marks = vec!["?"; items.len()].join(",");
        let mut stmt = self.conn.prepare(&format!(
            "SELECT s.parent_session_id, s.id, s.title, s.model, s.state, s.started_at, s.last_event_at,
                (SELECT kind FROM activity_event ae WHERE ae.session_id = s.id ORDER BY julianday(ae.timestamp) DESC, ae.id DESC LIMIT 1)
             FROM session s
             WHERE s.parent_session_id IN ({marks}) AND s.id <> s.parent_session_id
             ORDER BY julianday(s.last_event_at) DESC"
        ))?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(items.iter().map(|item| item.id.as_str())),
            |row| {
                let parent: String = row.get(0)?;
                let state: String = row.get(4)?;
                let last_event_at: String = row.get(6)?;
                let latest_kind: Option<String> = row.get(7)?;
                let age = DateTime::parse_from_rfc3339(&last_event_at)
                    .ok()
                    .map(|date| (now - date.with_timezone(&Utc)).num_milliseconds());
                let open_execution = matches!(
                    (state.as_str(), latest_kind.as_deref()),
                    ("thinking", Some("prompt")) | ("tool", Some("tool_start"))
                );
                let presence = classify_presence(&state, age, open_execution);
                let stale_open = presence == SessionPresence::Inactive
                    && matches!(state.as_str(), "thinking" | "tool");
                let model: Option<String> = row.get(3)?;
                Ok((
                    parent,
                    SubagentItem {
                        id: row.get(1)?,
                        title: row.get(2)?,
                        model: model
                            .filter(|m| !m.trim().is_empty() && !m.contains("MODEL_PLACEHOLDER"))
                            .unwrap_or_else(|| "Unspecified".to_owned()),
                        state: if stale_open {
                            "unknown".to_owned()
                        } else {
                            state
                        },
                        presence,
                        is_active: presence.is_active(),
                        is_ongoing: presence.is_ongoing(),
                        home_group: presence.home_group(),
                        started_at: row.get(5)?,
                        last_event_at,
                    },
                ))
            },
        )?;
        for row in rows {
            let (parent, child) = row?;
            let Some(item) = items.iter_mut().find(|item| item.id == parent) else {
                continue;
            };
            item.subagent_count += 1;
            item.subagents_ongoing += usize::from(child.is_ongoing);
            if item.subagents.len() < SUBAGENT_LIST_LIMIT {
                item.subagents.push(child);
            }
        }
        for item in items.iter_mut().filter(|item| !item.is_ongoing) {
            let live = item.subagents.iter().any(|child| child.is_active);
            if live || item.subagents_ongoing > 0 {
                item.presence = if live {
                    SessionPresence::Running
                } else {
                    SessionPresence::RecentlyIdle
                };
                item.is_ongoing = true;
                item.is_active = live;
                item.home_group = Some(HomeGroup::Running);
                if item.state == "unknown" {
                    "idle".clone_into(&mut item.state);
                }
            }
        }
        Ok(())
    }

    /// Returns local project rollups using the same time bounds as the timeline report.
    ///
    /// # Errors
    /// Returns an error if timeline or project query execution fails.
    pub fn generate_project_report(
        &self,
        range: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<ProjectUsage>, Error> {
        let timeline = self.generate_usage_report_timeline(range, from, to)?;
        if timeline.items_count == 0 {
            return Ok(Vec::new());
        }
        let mut statement = self.conn.prepare(obfstr::obfstr!(r#"
            SELECT p.id, p.name,
                SUM(u.input_tokens + u.output_tokens + CASE WHEN s.agent IN ('codex', 'gemini-cli') THEN 0
                    ELSE u.cache_read_tokens + u.cache_write_5m_tokens + u.cache_write_1h_tokens END),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END), COUNT(DISTINCT s.id), SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END), COUNT(*),
                CASE WHEN p.canonical_key LIKE 'scratch/%' THEN '' ELSE p.repo_root END, MAX(u.timestamp), GROUP_CONCAT(DISTINCT s.agent)
            FROM usage_record u JOIN session s ON s.id = u.session_id JOIN project p ON p.id = s.project_id
            WHERE s.surface <> 'cloud' AND p.canonical_key <> '/unknown' AND julianday(u.timestamp) >= julianday(?1) AND julianday(u.timestamp) <= julianday(?2)
            GROUP BY p.id ORDER BY SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END) DESC, p.id
        "#))?;
        let rows =
            statement.query_map(params![timeline.earliest_at, timeline.latest_at], |row| {
                let tokens: i64 = row.get(2)?;
                let sessions: i64 = row.get(4)?;
                let unpriced: i64 = row.get(5)?;
                let agents: Option<String> = row.get(9)?;
                Ok(ProjectUsage {
                    repo_root: row.get(7)?,
                    last_active: row.get(8)?,
                    agents: agents
                        .iter()
                        .flat_map(|list| list.split(','))
                        .map(str::to_owned)
                        .collect(),
                    priced_turns: (row.get::<_, i64>(6)? - unpriced).max(0) as u64,
                    unpriced_turns: unpriced.max(0) as u64,
                    id: row.get(0)?,
                    name: row.get(1)?,
                    total_tokens: tokens.max(0) as u64,
                    cost_usd: row.get(3)?,
                    sessions: sessions.max(0) as u64,
                    cost_basis: if unpriced > 0 {
                        TrustLevel::Unknown
                    } else {
                        TrustLevel::Estimated
                    },
                })
            })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Per-bucket usage rows shared by the timeline report and the activity grid:
    /// tokens including cache exactly once, recorded cost and turns per UTC day
    /// (per UTC hour when `range_kind` is `today`).
    fn daily_usage_rows(
        &self,
        range_kind: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<DailyUsage>, Error> {
        let mut statement = self.conn.prepare(obfstr::obfstr!(r#"
            SELECT CASE WHEN ?3 = 'today' THEN strftime('%Y-%m-%dT%H:00:00Z', u.timestamp) ELSE date(u.timestamp) END,
                SUM(u.input_tokens + u.output_tokens + CASE WHEN s.agent IN ('codex', 'gemini-cli') THEN 0
                    ELSE u.cache_read_tokens + u.cache_write_5m_tokens + u.cache_write_1h_tokens END),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END), COUNT(*),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END)
            FROM usage_record u JOIN session s ON s.id = u.session_id
            WHERE s.surface <> 'cloud' AND (?1 IS NULL OR julianday(u.timestamp) >= julianday(?1)) AND (?2 IS NULL OR julianday(u.timestamp) <= julianday(?2))
            GROUP BY 1 ORDER BY 1
        "#))?;
        let rows = statement
            .query_map(params![from, to, range_kind], |row| {
                let tokens: i64 = row.get(1)?;
                let turns: i64 = row.get(3)?;
                let unknown: i64 = row.get(4)?;
                Ok(DailyUsage {
                    priced_turns: (turns - unknown).max(0) as u64,
                    unpriced_turns: unknown.max(0) as u64,
                    date: row.get(0)?,
                    total_tokens: tokens.max(0) as u64,
                    cost_usd: row.get(2)?,
                    turns: turns.max(0) as u64,
                    cost_basis: if unknown > 0 {
                        TrustLevel::Unknown
                    } else {
                        TrustLevel::Estimated
                    },
                    chart_pct: 0.0,
                    level: 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Dense daily activity for the 53 ISO weeks ending on `today` (UTC), starting on a Monday.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn activity_heatmap(&self, today: chrono::NaiveDate) -> Result<ActivityHeatmap, Error> {
        let this_monday =
            today - chrono::Duration::days(i64::from(today.weekday().num_days_from_monday()));
        let from = this_monday - chrono::Duration::weeks(52);
        let rows = self.daily_usage_rows(
            "heatmap",
            Some(&format!("{from}T00:00:00Z")),
            Some(&format!("{today}T23:59:59.999Z")),
        )?;
        let mut by_date = std::collections::BTreeMap::new();
        for row in rows {
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&row.date, "%Y-%m-%d") {
                by_date.insert(date, row);
            }
        }
        let first_recorded: Option<chrono::NaiveDate> = self
            .conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT MIN(date(u.timestamp)) FROM usage_record u JOIN session s ON s.id = u.session_id WHERE s.surface <> 'cloud'"
                ),
                [],
                |row| row.get::<_, Option<String>>(0),
            )?
            .and_then(|text| chrono::NaiveDate::parse_from_str(&text, "%Y-%m-%d").ok());
        let mut days = Vec::new();
        let mut date = from;
        while date <= today {
            days.push(match by_date.remove(&date) {
                Some(row) => HeatmapDay {
                    date: date.to_string(),
                    total_tokens: row.total_tokens,
                    cost_usd: row.cost_usd,
                    cost_basis: row.cost_basis,
                    turns: row.turns,
                    level: 0,
                },
                None => HeatmapDay {
                    date: date.to_string(),
                    total_tokens: 0,
                    cost_usd: 0.0,
                    cost_basis: TrustLevel::Unknown,
                    turns: 0,
                    level: 0,
                },
            });
            date += chrono::Duration::days(1);
        }
        let maximum = days.iter().map(|day| day.total_tokens).max().unwrap_or(0);
        for day in &mut days {
            day.level = activity_level(day.total_tokens, maximum);
        }
        let stats = heatmap_stats(&days, first_recorded, today);
        Ok(ActivityHeatmap {
            from: from.to_string(),
            to: today.to_string(),
            first_recorded: first_recorded.map(|date| date.to_string()),
            days,
            stats,
        })
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
            "month" => {
                let local_now = chrono::Local::now();
                let first = local_now
                    .date_naive()
                    .with_day(1)
                    .and_then(|date| date.and_hms_opt(0, 0, 0))
                    .and_then(|time| time.and_local_timezone(chrono::Local).earliest())
                    .map(|time| time.to_rfc3339())
                    .unwrap_or_else(|| format!("{}-01T00:00:00Z", now.format("%Y-%m")));
                (
                    Some(from_ts.unwrap_or(&first).to_owned()),
                    "This month".to_owned(),
                )
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

        let mut stmt = self.conn.prepare(obfstr::obfstr!(
            r#"
            SELECT
                p.name AS project_name,
                s.agent AS agent_str,
                u.model,
                SUM(u.input_tokens) AS in_tok,
                SUM(u.cache_read_tokens) AS read_tok,
                SUM(u.output_tokens) AS out_tok,
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END) AS total_cost,
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cache_savings_usd END) AS total_savings,
                COUNT(u.dedup_key) AS turns,
                MIN(u.timestamp) AS earliest,
                MAX(u.timestamp) AS latest,
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END) AS unpriced
            FROM usage_record u
            JOIN session s ON u.session_id = s.id
            JOIN project p ON s.project_id = p.id
            WHERE s.surface <> 'cloud' AND (?1 IS NULL OR julianday(u.timestamp) >= julianday(?1))
              AND (?2 IS NULL OR julianday(u.timestamp) <= julianday(?2))
            GROUP BY p.id, s.agent, u.model
            ORDER BY total_cost DESC
            "#
        ))?;

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
            unpriced: u64,
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
                "unknown".to_owned()
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
                    unpriced: row.get::<_, i64>(11)?.max(0) as u64,
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

            if let Some(e_str) = e
                && global_earliest.as_ref().is_none_or(|cur| &e_str < cur)
            {
                global_earliest = Some(e_str);
            }
            if let Some(l_str) = l
                && global_latest.as_ref().is_none_or(|cur| &l_str > cur)
            {
                global_latest = Some(l_str);
            }

            raw_items.push(item);
        }

        use std::collections::BTreeMap;
        let mut agent_models: BTreeMap<&'static str, BTreeMap<String, ModelReport>> =
            BTreeMap::new();
        for item in raw_items {
            let agent_label = gnomcode_core::pricing::format_agent_display(&item.agent);
            let models_map = agent_models.entry(agent_label).or_default();
            let entry = models_map
                .entry(item.model.clone())
                .or_insert_with(|| ModelReport {
                    priced_turns: 0,
                    unpriced_turns: 0,
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
            entry.priced_turns += item.turns.saturating_sub(item.unpriced);
            entry.unpriced_turns += item.unpriced;
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
                    priced_turns: models.iter().map(|model| model.priced_turns).sum(),
                    unpriced_turns: models.iter().map(|model| model.unpriced_turns).sum(),
                    provider: agent_label.to_owned(),
                    total_cost_usd: prov_cost,
                    total_cache_savings_usd: prov_savings,
                    pct_of_spend: pct,
                    models,
                });
            }
        }

        let items_count = providers.iter().map(|p| p.models.len()).sum();

        let mut daily = self.daily_usage_rows(range_kind, cutoff.as_deref(), to_ts)?;
        let (grouped, activity_interval) = group_activity(daily, range_kind);
        daily = grouped;
        let maximum = daily.iter().map(|day| day.total_tokens).max().unwrap_or(0);
        let total_tokens = daily
            .iter()
            .fold(0u64, |sum, day| sum.saturating_add(day.total_tokens));
        for day in &mut daily {
            day.chart_pct = if maximum == 0 {
                0.0
            } else {
                day.total_tokens as f64 / maximum as f64 * 100.0
            };
            day.level = activity_level(day.total_tokens, maximum);
        }

        Ok(TimelineReport {
            priced_turns: daily.iter().map(|day| day.priced_turns).sum(),
            unpriced_turns: daily.iter().map(|day| day.unpriced_turns).sum(),
            cost_basis: if daily
                .iter()
                .any(|day| day.cost_basis == TrustLevel::Unknown)
            {
                TrustLevel::Unknown
            } else {
                TrustLevel::Estimated
            },
            cache_share_pct: total_tokens
                .checked_sub(total_out)
                .filter(|input| *input > 0)
                .map(|input| (total_read as f64 / input as f64 * 100.0).clamp(0.0, 100.0)),
            daily,
            activity_interval: activity_interval.to_owned(),
            total_tokens,
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
            SELECT s.pid,
                COALESCE(s.cwd, CASE WHEN p.canonical_key = '/unknown' OR p.canonical_key LIKE 'scratch/%' THEN NULL ELSE p.repo_root END),
                s.agent, s.surface
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
                    agent: r.get(2)?,
                    surface: r.get(3)?,
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

fn antigravity_workspace(root: &std::path::Path, conv_id: &str) -> Option<String> {
    [
        obfstr::obfstr!("antigravity-cli"),
        obfstr::obfstr!("antigravity"),
    ]
    .iter()
    .find_map(|product| {
        let path = root
            .join(product)
            .join(obfstr::obfstr!("conversation_summaries.db"));
        let conn =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .ok()?;
        let uris: String = conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT workspace_uris FROM conversation_summaries WHERE conversation_id = ?1"
                ),
                params![conv_id],
                |row| row.get(0),
            )
            .ok()?;
        let value: serde_json::Value = serde_json::from_str(&uris).ok()?;
        let uri = value
            .as_array()?
            .first()?
            .as_str()?
            .replace("%3A", ":")
            .replace("%3a", ":");
        let path = uri.strip_prefix("file://").unwrap_or(&uri);
        let path = if path.as_bytes().get(2) == Some(&b':') {
            path.strip_prefix('/').unwrap_or(path)
        } else {
            path
        };
        Some(path.to_owned())
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn format_repair_updates_existing_chat_without_resurrecting_deleted_rows() {
        let mut db = super::Database::open_in_memory().expect("db");
        let now = chrono::Utc::now();
        let project = gnomcode_core::Project::from_origin(None, "/fixture", None);
        let session = gnomcode_core::Session {
            id: "codex:format".into(),
            agent: gnomcode_core::AgentId::Codex,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let checkpoint = super::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 10,
            file_size: 10,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("metadata");
        let mut event = gnomcode_core::ChatEvent {
            event_id: "format-event".into(),
            session_id: "codex:format".into(),
            turn_index: 1,
            role: gnomcode_core::ChatRole::Assistant,
            content: "# Heading body".into(),
            tool_name: None,
            status: None,
            created_at: now,
        };
        db.insert_chat_event(&event).expect("old chat");
        let old_secret = "**sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz**\n\nbody";
        db.conn
            .execute(
                "UPDATE chat_event SET content=?1 WHERE event_id=?2",
                rusqlite::params![old_secret, event.event_id],
            )
            .expect("legacy fixture");
        assert_eq!(
            db.get_session_chat("codex:format", None)
                .expect("legacy read")[0]
                .content,
            "**<REDACTED>**\n\nbody"
        );
        assert_eq!(
            db.conversation_chat("codex:format", None, None)
                .expect("continuous legacy read")[0]
                .content,
            "**<REDACTED>**\n\nbody"
        );
        db.conn
            .execute(
                "UPDATE chat_event SET content=?1 WHERE event_id=?2",
                rusqlite::params![event.content, event.event_id],
            )
            .expect("restore fixture");
        assert!(db.chat_format_needs_repair().expect("repair status"));
        let mut different = event.clone();
        different.content = "Different text at a reused source offset".into();
        db.repair_chat_contents(&[different])
            .expect("ignore changed native content");
        assert_eq!(
            db.get_session_chat("codex:format", None)
                .expect("original chat")[0]
                .content,
            event.content
        );
        event.content = "# Heading\n\nbody".into();
        db.repair_chat_contents(std::slice::from_ref(&event))
            .expect("repair");
        assert_eq!(
            db.get_session_chat("codex:format", None).expect("chat")[0].content,
            event.content
        );
        db.delete_session_chat("codex:format").expect("delete");
        db.repair_chat_contents(std::slice::from_ref(&event))
            .expect("do not resurrect");
        assert!(
            db.get_session_chat("codex:format", None)
                .expect("deleted chat")
                .is_empty()
        );
        assert_eq!(
            db.get_checkpoint("fixture")
                .expect("checkpoint")
                .expect("source")
                .last_read_offset,
            10
        );
        db.finish_chat_format_repair().expect("finish");
        assert!(!db.chat_format_needs_repair().expect("finished"));
    }
    #[test]
    fn antigravity_workspace_lookup_includes_cli_and_preserves_ide() {
        let root = tempfile::tempdir().expect("gemini home");
        for (product, id) in [("antigravity", "ide-test"), ("antigravity-cli", "cli-test")] {
            let directory = root.path().join(product);
            std::fs::create_dir_all(&directory).expect("directory");
            let db = rusqlite::Connection::open(directory.join("conversation_summaries.db"))
                .expect("summary db");
            db.execute_batch(
                "CREATE TABLE conversation_summaries(conversation_id TEXT, workspace_uris TEXT);",
            )
            .expect("schema");
            db.execute(
                "INSERT INTO conversation_summaries VALUES (?1, ?2)",
                rusqlite::params![id, r#"["file:///C:/test-workspace"]"#],
            )
            .expect("summary");
        }
        for id in ["ide-test", "cli-test"] {
            assert_eq!(
                super::antigravity_workspace(root.path(), id).as_deref(),
                Some("C:/test-workspace")
            );
        }
        assert_eq!(super::antigravity_workspace(root.path(), "missing"), None);
    }
    use super::{ContextRelayEntry, Database, SourceCheckpoint};
    use chrono::Utc;
    use gnomcode_core::rules::{GuardrailRule, RuleAction};
    use gnomcode_core::{
        ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
        SessionState, Surface, TrustLevel, UsageRecord,
    };

    #[test]
    fn unresolved_tool_step_is_running_only_within_ten_minutes() {
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "/w", None);
        let now = Utc::now();
        let session = |name: &str, age: chrono::Duration| Session {
            id: format!("antigravity:{name}"),
            agent: AgentId::Antigravity,
            surface: Surface::Ide,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Tool,
            pid: None,
            started_at: now - age,
            last_event_at: now - age,
        };
        db.commit_batch(
            std::slice::from_ref(&project),
            &[
                session("recent", chrono::Duration::minutes(5)),
                session("stalled", chrono::Duration::minutes(60)),
                session("abandoned", chrono::Duration::hours(13)),
            ],
            &[],
            &[],
            &[],
            &SourceCheckpoint {
                file_path: "fixture".into(),
                last_read_offset: 0,
                file_size: 0,
                fingerprint: "fixture".into(),
                last_ingested_at: now,
            },
        )
        .expect("sessions");
        for (name, age) in [("recent", 5), ("stalled", 60), ("abandoned", 13 * 60)] {
            db.conn
                .execute(
                    "INSERT INTO activity_event(session_id,timestamp,kind) VALUES (?1,?2,'tool_start')",
                    rusqlite::params![
                        format!("antigravity:{name}"),
                        (now - chrono::Duration::minutes(age)).to_rfc3339()
                    ],
                )
                .expect("activity");
        }
        let (rows, count) = db
            .list_sessions_paginated(None, Some("ongoing"), 0, 10)
            .expect("ongoing");
        assert_eq!(
            count, 1,
            "count must match the presence rules the list uses"
        );
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, "antigravity:recent");
    }

    fn commit_sessions(db: &mut Database, project: &Project, sessions: &[Session]) {
        db.commit_batch(
            std::slice::from_ref(project),
            sessions,
            &[],
            &[],
            &[],
            &SourceCheckpoint {
                file_path: "fixture".into(),
                last_read_offset: 0,
                file_size: 0,
                fingerprint: "fixture".into(),
                last_ingested_at: Utc::now(),
            },
        )
        .expect("sessions");
    }

    #[test]
    fn live_subagent_makes_a_quiet_parent_ongoing_and_nests_under_it() {
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "/w", None);
        let now = Utc::now();
        let session = |id: &str, parent: Option<&str>, state: SessionState, minutes: i64| Session {
            id: format!("claude-code:{id}"),
            agent: AgentId::ClaudeCode,
            surface: Surface::DesktopApp,
            project_id: project.id.clone(),
            parent_session_id: parent.map(|p| format!("claude-code:{p}")),
            model: None,
            title: None,
            state,
            pid: None,
            started_at: now - chrono::Duration::minutes(minutes),
            last_event_at: now - chrono::Duration::minutes(minutes),
        };
        commit_sessions(
            &mut db,
            &project,
            &[
                session("quiet", None, SessionState::Idle, 45),
                session("quiet-sub", Some("quiet"), SessionState::Tool, 1),
                session("done", None, SessionState::Idle, 45),
                session("done-sub", Some("done"), SessionState::Idle, 40),
            ],
        );
        db.conn
            .execute(
                "INSERT INTO activity_event(session_id,timestamp,kind) VALUES ('claude-code:quiet-sub',?1,'tool_start')",
                rusqlite::params![(now - chrono::Duration::minutes(1)).to_rfc3339()],
            )
            .expect("activity");
        let (rows, count) = db
            .list_sessions_paginated(None, Some("ongoing"), 0, 10)
            .expect("ongoing");
        assert_eq!(
            (count, rows.len()),
            (1, 1),
            "only the parent with a live subagent"
        );
        let quiet = &rows[0];
        assert_eq!(quiet.id, "claude-code:quiet");
        assert!(quiet.is_active && quiet.is_ongoing);
        assert_eq!(
            quiet.home_group,
            Some(gnomcode_core::presence::HomeGroup::Running)
        );
        assert_eq!((quiet.subagent_count, quiet.subagents_ongoing), (1, 1));
        assert_eq!(quiet.subagents[0].id, "claude-code:quiet-sub");
        let (all, total) = db.list_sessions_paginated(None, None, 0, 10).expect("all");
        assert_eq!(total, 2, "subagents are nested, never top-level rows");
        let done = all
            .iter()
            .find(|row| row.id == "claude-code:done")
            .expect("done");
        assert!(!done.is_ongoing);
        assert_eq!((done.subagent_count, done.subagents_ongoing), (1, 0));
    }

    #[test]
    fn a_later_chunk_without_a_host_marker_keeps_the_desktop_surface() {
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "/w", None);
        let now = Utc::now();
        let session = |surface: Surface| Session {
            id: "claude-code:s".into(),
            agent: AgentId::ClaudeCode,
            surface,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        commit_sessions(&mut db, &project, &[session(Surface::Cli)]);
        commit_sessions(&mut db, &project, &[session(Surface::DesktopApp)]);
        commit_sessions(&mut db, &project, &[session(Surface::Cli)]);
        let (rows, _) = db.list_sessions_paginated(None, None, 0, 10).expect("rows");
        assert_eq!(rows[0].surface, "desktop_app");
    }

    #[test]
    fn all_time_activity_groups_by_span_without_losing_totals() {
        use super::{DailyUsage, group_activity};
        let rows = ["2024-01-01", "2024-01-31", "2024-10-01"]
            .map(|date| DailyUsage {
                date: date.into(),
                total_tokens: 10,
                cost_usd: 1.0,
                cost_basis: TrustLevel::Estimated,
                turns: 1,
                priced_turns: 1,
                unpriced_turns: 0,
                chart_pct: 0.0,
                level: 0,
            })
            .to_vec();
        let (rows, unit) = group_activity(rows, "all");
        assert_eq!(unit, "month");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].total_tokens, 20);
        let mut rows = rows;
        rows.push(DailyUsage {
            date: "2027-01-01".into(),
            ..rows[0].clone()
        });
        let (rows, unit) = group_activity(rows, "all");
        assert_eq!(unit, "year");
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn activity_uses_calendar_weeks_and_only_recorded_intervals() {
        let row = |date: &str| super::DailyUsage {
            date: date.into(),
            total_tokens: 10,
            cost_usd: 1.0,
            cost_basis: TrustLevel::Estimated,
            turns: 1,
            priced_turns: 1,
            unpriced_turns: 0,
            chart_pct: 0.0,
            level: 0,
        };
        assert_eq!(
            super::group_activity(vec![row("2026-01-01"), row("2026-01-30")], "all").1,
            "day"
        );
        let (weeks, unit) = super::group_activity(
            vec![
                row("2025-12-01"),
                row("2025-12-29"),
                row("2026-01-01"),
                row("2026-01-04"),
                row("2026-01-05"),
            ],
            "all",
        );
        assert_eq!(unit, "week");
        assert_eq!(weeks.len(), 3);
        assert_eq!(weeks[1].date, "2025-12-29");
        assert_eq!(weeks[1].total_tokens, 30);
        assert_eq!(weeks[2].date, "2026-01-05");
        assert_eq!(weeks.iter().map(|row| row.total_tokens).sum::<u64>(), 50);
        assert_eq!(
            super::group_activity(vec![row("2024-01-01"), row("2025-01-01")], "all").1,
            "month"
        );
        assert_eq!(
            super::group_activity(vec![row("2024-01-01"), row("2026-07-01")], "all").1,
            "year"
        );
    }
    #[test]
    fn retired_cloud_rows_stay_hidden_without_deleting_history() {
        let mut db = Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, "fixture", None);
        let now = Utc::now();
        let session = Session {
            id: "codex:retired".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: None,
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        db.commit_batch(
            &[project],
            &[session],
            &[],
            &[],
            &[],
            &SourceCheckpoint {
                file_path: "fixture".to_owned(),
                last_read_offset: 0,
                file_size: 0,
                fingerprint: "fixture".to_owned(),
                last_ingested_at: now,
            },
        )
        .expect("commit");
        db.conn
            .execute(
                "UPDATE session SET surface='cloud' WHERE id='codex:retired'",
                [],
            )
            .expect("legacy row");
        db.conn.execute(
            "INSERT INTO usage_record VALUES ('retired:usage','codex:retired',?1,'fixture-model',1000,100,0,0,0,0,1000,10000,1.0,0.0,'estimated','fixture')",
            [now.to_rfc3339()],
        ).expect("retired usage");
        assert!(db.generate_usage_report().expect("report").is_empty());
        let timeline = db
            .generate_usage_report_timeline("all", None, None)
            .expect("timeline");
        assert!(timeline.daily.is_empty());
        assert_eq!(timeline.total_tokens, 0);
        assert!(
            db.generate_project_report("all", None, None)
                .expect("projects")
                .is_empty()
        );
        assert!(
            db.list_active_sessions(10)
                .expect("CLI sessions")
                .is_empty()
        );
        let (rows, total) = db
            .list_sessions_paginated(None, None, 0, 10)
            .expect("UI sessions");
        assert!(rows.is_empty());
        assert_eq!(total, 0);
        let count: i64 = db
            .conn
            .query_row("SELECT count(*) FROM session", [], |row| row.get(0))
            .expect("history");
        assert_eq!(count, 1);
    }

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
    fn resume_queue_does_not_store_user_prompt_text() {
        use gnomcode_core::rules::ResumeQueueEntry;
        let db = Database::open_in_memory().expect("database");
        let entry = ResumeQueueEntry {
            session_id: "codex:session-privacy".to_owned(),
            agent: AgentId::Codex,
            cwd: ".".to_owned(),
            resume_prompt: "private text that must not be stored".to_owned(),
            max_runs: 1,
            runs_completed: 0,
            eligible_after: Utc::now(),
        };
        db.arm_resume_queue(&entry).expect("arm");
        let stored = db.list_resume_queue().expect("list");
        assert_eq!(stored[0].resume_prompt, "continue");
    }
    #[test]
    fn resume_queue_persists_arms_and_disarms() {
        use gnomcode_core::rules::ResumeQueueEntry;
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
        use gnomcode_core::RoutingPolicy;
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

        // Test RoutingPolicy wrapper
        assert_eq!(
            db.get_routing_policy().expect("get policy"),
            RoutingPolicy::Automatic
        );
        db.save_routing_policy(RoutingPolicy::DenyCrossProvider)
            .expect("save routing policy");
        assert_eq!(
            db.get_routing_policy().expect("get policy 2"),
            RoutingPolicy::DenyCrossProvider
        );

        assert!(db.get_auto_update_enabled().expect("default auto update"));
        db.set_auto_update_enabled(false)
            .expect("disable auto update");
        assert!(!db.get_auto_update_enabled().expect("disabled auto update"));

        let entry = ContextRelayEntry {
            id: "relay-1".to_owned(),
            session_id: "claude-code:s1".to_owned(),
            source_agent: "claude-code".to_owned(),
            target_agent: "claude-code".to_owned(),
            project_name: "gnomcode".to_owned(),
            cwd: "/workspace/gnomcode".to_owned(),
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
            Some("https://github.com/mwahaj36/spawnt"),
            "/workspace/gnomcode",
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
        let (ongoing, ongoing_count) = db
            .list_sessions_paginated(None, Some("ongoing"), 0, 10)
            .expect("ongoing idle session");
        assert_eq!(ongoing_count, 1, "recent idle sessions remain ongoing");
        assert_eq!(ongoing[0].state, "idle");
        assert!(!ongoing[0].is_active);
        let old = (Utc::now() - chrono::Duration::seconds(901)).to_rfc3339();
        db.conn
            .execute(
                "UPDATE session SET last_event_at = ?1 WHERE id = 'claude-code:s1'",
                rusqlite::params![old],
            )
            .expect("age idle");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("stale idle")
                .1,
            0
        );
        db.conn
            .execute(
                "UPDATE session SET state = 'awaiting_approval' WHERE id = 'claude-code:s1'",
                [],
            )
            .expect("approval wait");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("old approval")
                .1,
            1
        );
        db.conn
            .execute(
                "UPDATE session SET state = 'ended' WHERE id = 'claude-code:s1'",
                [],
            )
            .expect("explicit end");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("ended excluded")
                .1,
            0
        );
        assert_eq!(
            db.list_sessions_paginated(None, None, 0, 10)
                .expect("ended history")
                .0[0]
                .state,
            "ended"
        );
        db.conn
            .execute(
                "UPDATE session SET state = 'tool' WHERE id = 'claude-code:s1'",
                [],
            )
            .expect("open tool");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("stalled tool")
                .1,
            0,
            "an unresolved tool step older than ten minutes is not Running"
        );
        let ancient = (Utc::now() - chrono::Duration::days(15)).to_rfc3339();
        db.conn
            .execute(
                "UPDATE session SET last_event_at = ?1 WHERE id = 'claude-code:s1'",
                rusqlite::params![ancient],
            )
            .expect("age tool");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("ancient tool")
                .1,
            0
        );
        db.conn
            .execute(
                "UPDATE session SET last_event_at = ?1 WHERE id = 'claude-code:s1'",
                rusqlite::params![old],
            )
            .expect("restore tool age");
        db.conn
            .execute(
                "UPDATE activity_event SET kind = 'tool_end' WHERE session_id = 'claude-code:s1'",
                [],
            )
            .expect("closed tool evidence");
        assert_eq!(
            db.list_sessions_paginated(None, Some("ongoing"), 0, 10)
                .expect("no execution evidence")
                .1,
            0
        );
        db.conn
            .execute(
                "UPDATE activity_event SET kind = 'tool_start' WHERE session_id = 'claude-code:s1'",
                [],
            )
            .expect("restore activity");
        db.conn
            .execute(
                "UPDATE session SET state = 'idle', last_event_at = ?1 WHERE id = 'claude-code:s1'",
                rusqlite::params![Utc::now().to_rfc3339()],
            )
            .expect("restore idle");

        assert_eq!(all_recent[0].title.as_deref(), Some("Implement M1"));

        let report = db.generate_usage_report().expect("report");
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].turn_count, 1);
        assert_eq!(report[0].input_tokens, 1000);

        let other_project = Project::from_origin(
            Some("https://github.com/other/spawnt"),
            "/workspace/other/gnomcode",
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
        let projects = db
            .generate_project_report("all", None, None)
            .expect("project rollups");
        assert_eq!(projects.len(), 2);
        assert_ne!(projects[0].id, projects[1].id);
        assert_eq!(projects[0].total_tokens, 6_200);
        assert_eq!(projects[0].sessions, 1);
        db.conn
            .execute("UPDATE usage_record SET cost_basis = 'unknown'", [])
            .expect("unpriced usage");
        assert_eq!(
            db.generate_usage_report_timeline("all", None, None)
                .expect("unpriced report")
                .cost_basis,
            TrustLevel::Unknown
        );

        let latest_quota = db.get_latest_quota_snapshots().expect("latest quota");
        assert!(!latest_quota.is_empty());
        assert!(latest_quota.iter().any(|q| q.limit_id == "codex"));
        db.conn
            .execute("UPDATE quota_snapshot SET basis = 'estimated'", [])
            .expect("set quota provenance");
        assert_eq!(
            db.get_latest_quota_snapshots().expect("estimated quota")[0].basis,
            TrustLevel::Estimated
        );

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
        db.commit_batch(
            std::slice::from_ref(&project),
            std::slice::from_ref(&session),
            std::slice::from_ref(&usage),
            &[],
            &[],
            &checkpoint,
        )
        .expect("commit synthetic Codex usage");
        db.conn
            .execute("UPDATE usage_record SET cache_read_tokens = 100000", [])
            .expect("cached Codex input");

        let (sessions, count) = db
            .list_sessions_paginated(None, None, 0, 10)
            .expect("query Codex session");
        assert_eq!(count, 1);
        assert_eq!(sessions[0].context_tokens, 190_000);
        assert_eq!(sessions[0].total_tokens, Some(190_000));
        assert_eq!(sessions[0].context_window, Some(258_000));

        // Every provider must use the latest context snapshot, even when it decreases.
        for agent in [
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Antigravity,
            AgentId::GeminiCli,
        ] {
            let mut provider_session = session.clone();
            provider_session.agent = agent;
            provider_session.id = format!("{}:compaction", agent.as_str());
            let mut before = usage.clone();
            before.session_id = provider_session.id.clone();
            before.dedup_key = format!("{}:before", provider_session.id);
            before.context_tokens = 190_000;
            let mut after = before.clone();
            after.dedup_key = format!("{}:after", provider_session.id);
            after.input_tokens = 19_000;
            after.output_tokens = 1_000;
            after.context_tokens = 20_000;
            db.commit_batch(
                std::slice::from_ref(&project),
                &[provider_session.clone()],
                &[before, after],
                &[],
                &[],
                &checkpoint,
            )
            .expect("store provider context snapshots");
            let (rows, _) = db
                .list_sessions_paginated(None, None, 0, 100)
                .expect("query snapshots");
            let row = rows
                .iter()
                .find(|row| row.id == provider_session.id)
                .expect("provider session");
            assert_eq!(
                row.context_tokens,
                20_000,
                "{} context must decrease",
                agent.as_str()
            );
        }
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
            Some("https://github.com/mwahaj36/spawnt"),
            "/workspace/gnomcode",
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
        assert_eq!(rep_all.daily.len(), 2);
        assert_eq!(rep_all.total_tokens, 1_650);
        assert_eq!(rep_all.cost_basis, TrustLevel::Estimated);
        assert_eq!(rep_all.cache_share_pct, Some(0.0));
        assert_eq!(rep_all.daily[0].total_tokens, 1_100);
        assert_eq!(rep_all.daily[1].total_tokens, 550);

        // 2. "today" should include u_now (5.0) and exclude u_old
        let rep_today = db
            .generate_usage_report_timeline("today", None, None)
            .expect("report today");
        assert_eq!(rep_today.total_cost_usd, 5.0);
        assert_eq!(rep_today.daily.len(), 1);
        assert_eq!(rep_today.total_tokens, 550);

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

        db.conn
            .execute(
                "UPDATE usage_record SET cost_basis = 'unknown' WHERE dedup_key = 'old:1'",
                [],
            )
            .expect("unprice old record");
        let partial = db
            .generate_usage_report_timeline("all", None, None)
            .expect("partial");
        assert_eq!((partial.priced_turns, partial.unpriced_turns), (1, 1));
        assert_eq!(partial.total_cost_usd, 5.0);
        assert_eq!(partial.cost_basis, TrustLevel::Unknown);
        assert_eq!(partial.providers[0].priced_turns, 1);
        assert_eq!(partial.providers[0].unpriced_turns, 1);
        let projects = db
            .generate_project_report("all", None, None)
            .expect("project coverage");
        assert_eq!(
            (projects[0].priced_turns, projects[0].unpriced_turns),
            (1, 1)
        );
        assert_eq!(projects[0].cost_usd, 5.0);
        let (project_rows, count) = db
            .list_sessions_filtered(None, None, Some(&projects[0].id), 0, 1)
            .expect("project filter");
        assert_eq!(count, 1);
        assert_eq!(
            project_rows[0].home_group,
            Some(project_rows[0].presence.home_group())
        );
        let (rows, count) = db
            .list_sessions_filtered(None, None, Some("missing-project"), 0, 1)
            .expect("unmatched project");
        assert!(rows.is_empty());
        assert_eq!(count, 0);

        db.conn.execute("UPDATE usage_record SET timestamp = '2026-10-27T23:30:00+05:00' WHERE dedup_key = 'old:1'", []).expect("before local midnight");
        db.conn.execute("UPDATE usage_record SET timestamp = '2026-10-28T01:30:00+05:00' WHERE dedup_key = 'now:1'", []).expect("after local midnight");
        let local_day = db
            .generate_usage_report_timeline(
                "today",
                Some("2026-10-27T19:00:00Z"),
                Some("2026-10-28T19:00:00Z"),
            )
            .expect("local day");
        assert_eq!(local_day.total_tokens, 550);
        assert_eq!(local_day.daily.len(), 1);
        assert_eq!(local_day.daily[0].date, "2026-10-27T20:00:00Z");
        let month = db
            .generate_usage_report_timeline("month", Some("2026-09-30T19:00:00Z"), None)
            .expect("calendar month");
        assert_eq!(month.range_label, "This month");
        assert_eq!(month.total_tokens, 1650);
    }

    #[test]
    fn chat_event_storage_retrieval_and_hard_deletion() {
        use gnomcode_core::{ChatEvent, ChatRole};
        let mut db = Database::open_in_memory().expect("db");
        let project = Project::from_origin(None, "C:\\repo", None);
        let session = Session {
            id: "chat-sess-1".to_owned(),
            agent: AgentId::ClaudeCode,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("claude-3-5-sonnet".to_owned()),
            title: Some("Chat Test".to_owned()),
            state: SessionState::Idle,
            pid: Some(1234),
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };

        let cp = SourceCheckpoint {
            file_path: "/path/to/chat.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "chat-fp".to_owned(),
            last_ingested_at: Utc::now(),
        };

        db.commit_batch(&[project], &[session], &[], &[], &[], &cp)
            .expect("commit base session");

        let evt1 = ChatEvent {
            event_id: "evt-1".to_owned(),
            session_id: "chat-sess-1".to_owned(),
            turn_index: 1,
            role: ChatRole::User,
            content: "Fix compile error token=secret-value".to_owned(),
            tool_name: None,
            status: None,
            created_at: Utc::now(),
        };

        let evt2 = ChatEvent {
            event_id: "evt-2".to_owned(),
            session_id: "chat-sess-1".to_owned(),
            turn_index: 2,
            role: ChatRole::Tool,
            content: "raw tool output that must not be stored".to_owned(),
            tool_name: Some("Bash".to_owned()),
            status: Some("success".to_owned()),
            created_at: Utc::now(),
        };

        db.insert_chat_event(&evt1).expect("insert evt1");
        db.insert_chat_event(&evt2).expect("insert evt2");
        let invalid = ChatEvent {
            event_id: "invalid".into(),
            session_id: "missing-session".into(),
            ..evt1.clone()
        };
        let mut failed_checkpoint = cp.clone();
        failed_checkpoint.last_read_offset = 999;
        assert!(
            db.commit_batch_with_chat(&[], &[], &[], &[], &[], &failed_checkpoint, &[invalid])
                .is_err()
        );
        assert_eq!(
            db.get_checkpoint(&cp.file_path)
                .expect("checkpoint")
                .expect("existing")
                .last_read_offset,
            cp.last_read_offset
        );
        db.commit_batch_with_chat(&[], &[], &[], &[], &[], &cp, &[evt1.clone(), evt2.clone()])
            .expect("replay chat");
        let recent = db
            .get_session_chat(&evt1.session_id, Some(1))
            .expect("recent");
        assert_eq!(recent[0].event_id, evt2.event_id);
        let older = db
            .get_session_chat_before(&evt1.session_id, Some(1), Some(&evt2.event_id))
            .expect("older page");
        assert_eq!(older[0].event_id, evt1.event_id);
        assert!(
            db.get_session_chat_before(&evt1.session_id, Some(1), Some(&evt1.event_id))
                .expect("end of history")
                .is_empty()
        );

        let events = db
            .get_session_chat("chat-sess-1", None)
            .expect("fetch chat");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_id, "evt-1");
        assert_eq!(events[0].role, ChatRole::User);
        assert!(!events[0].content.contains("secret-value"));
        assert_eq!(events[1].event_id, "evt-2");
        assert_eq!(events[1].tool_name.as_deref(), Some("Bash"));
        assert!(!events[1].content.contains("raw tool output"));
        let packet = db
            .prepare_handoff("chat-sess-1", "o3", vec!["src/main.rs".into()])
            .expect("handoff")
            .expect("session");
        assert_eq!(packet.source_model, "claude-3-5-sonnet");
        assert_eq!(packet.target_model, "o3");
        assert!(packet.prompt_summary.contains("Fix compile error"));
        assert!(!packet.prompt_summary.contains("secret-value"));
        assert_eq!(packet.touched_files, ["src/main.rs"]);
        assert!(
            db.prepare_handoff("missing", "o3", vec![])
                .expect("query")
                .is_none()
        );

        // Hard deletion
        let deleted = db.delete_session_chat("chat-sess-1").expect("hard delete");
        assert_eq!(deleted, 2);

        let after = db
            .get_session_chat("chat-sess-1", None)
            .expect("fetch empty");
        assert!(after.is_empty());
    }

    #[test]
    fn activity_level_zero_for_no_tokens() {
        assert_eq!(super::activity_level(0, 1_000), 0);
        assert_eq!(super::activity_level(10, 0), 0);
    }

    #[test]
    fn activity_level_four_for_max() {
        assert_eq!(super::activity_level(1_000, 1_000), 4);
    }

    #[test]
    fn activity_level_one_for_tiny_values() {
        assert_eq!(super::activity_level(1, 1_000_000), 1);
    }

    #[test]
    fn activity_level_sqrt_scaled() {
        assert_eq!(super::activity_level(400, 1_600), 2);
        assert_eq!(super::activity_level(100, 1_600), 1);
    }

    fn heat_day(date: &str, tokens: u64) -> super::HeatmapDay {
        super::HeatmapDay {
            date: date.to_owned(),
            total_tokens: tokens,
            cost_usd: 0.0,
            cost_basis: TrustLevel::Unknown,
            turns: 0,
            level: 0,
        }
    }

    fn date(text: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d").expect("date")
    }

    fn heatmap_db(timestamps: &[&str]) -> Database {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(
            Some("https://github.com/mwahaj36/spawnt"),
            "/workspace/gnomcode",
            Some("main"),
        );
        let session = Session {
            id: "antigravity:heat".to_owned(),
            agent: AgentId::Antigravity,
            surface: Surface::Ide,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gemini-2.5-pro".to_owned()),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let usage: Vec<UsageRecord> = timestamps
            .iter()
            .enumerate()
            .map(|(index, stamp)| UsageRecord {
                dedup_key: format!("heat:{index}"),
                session_id: session.id.clone(),
                timestamp: chrono::DateTime::parse_from_rfc3339(stamp)
                    .expect("timestamp")
                    .with_timezone(&Utc),
                model: "gemini-2.5-pro".to_owned(),
                input_tokens: 100,
                output_tokens: 10,
                reasoning_tokens: 0,
                cache_read_tokens: 0,
                cache_write_5m_tokens: 0,
                cache_write_1h_tokens: 0,
                context_tokens: 110,
                context_window: Some(1_000_000),
                cost_usd: 1.0,
                cache_savings_usd: 0.0,
                cost_basis: TrustLevel::Exact,
                pricing_version: "2026-10-26".to_owned(),
            })
            .collect();
        let checkpoint = SourceCheckpoint {
            file_path: "heat.jsonl".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "heat".to_owned(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch(&[project], &[session], &usage, &[], &[], &checkpoint)
            .expect("commit batch");
        db
    }

    #[test]
    fn heatmap_starts_on_monday_and_ends_today() {
        use chrono::Datelike;
        let db = heatmap_db(&[]);
        let today = date("2026-11-12");
        let map = db.activity_heatmap(today).expect("heatmap");
        assert!((365..=371).contains(&map.days.len()));
        let first = date(&map.days[0].date);
        assert_eq!(first.weekday(), chrono::Weekday::Mon);
        assert_eq!(map.days.last().expect("last").date, "2026-11-12");
        assert_eq!(map.from, map.days[0].date);
        assert_eq!(map.to, "2026-11-12");
    }

    #[test]
    fn heatmap_is_dense_with_zero_days() {
        let db = heatmap_db(&["2026-11-10T12:00:00Z"]);
        let map = db.activity_heatmap(date("2026-11-12")).expect("heatmap");
        for pair in map.days.windows(2) {
            assert_eq!(
                date(&pair[1].date) - date(&pair[0].date),
                chrono::Duration::days(1)
            );
        }
        let active: Vec<_> = map.days.iter().filter(|day| day.total_tokens > 0).collect();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].date, "2026-11-10");
        assert_eq!(active[0].total_tokens, 110);
        assert_eq!(active[0].level, 4);
        assert_eq!(map.days[0].level, 0);
        assert_eq!(map.days[0].cost_basis, TrustLevel::Unknown);
    }

    #[test]
    fn heatmap_first_recorded_none_when_empty() {
        let db = heatmap_db(&[]);
        let map = db.activity_heatmap(date("2026-11-12")).expect("heatmap");
        assert_eq!(map.first_recorded, None);
        assert_eq!(map.stats.active_days, 0);
        assert!(map.stats.busiest.is_none());
    }

    #[test]
    fn heatmap_stats_streak_across_gap() {
        let days = [
            heat_day("2026-11-09", 5),
            heat_day("2026-11-10", 5),
            heat_day("2026-11-11", 0),
            heat_day("2026-11-12", 5),
            heat_day("2026-11-13", 5),
        ];
        let stats = super::heatmap_stats(&days, Some(date("2026-11-09")), date("2026-11-13"));
        assert_eq!(stats.current_streak, 2);
        assert_eq!(stats.longest_streak, 2);
        assert_eq!(stats.active_days, 4);
        assert_eq!(stats.total_tokens, 20);
    }

    #[test]
    fn heatmap_current_streak_counts_from_yesterday_when_today_empty() {
        let days = [
            heat_day("2026-11-10", 5),
            heat_day("2026-11-11", 5),
            heat_day("2026-11-12", 0),
        ];
        let stats = super::heatmap_stats(&days, Some(date("2026-11-10")), date("2026-11-12"));
        assert_eq!(stats.current_streak, 2);
        let stale = [
            heat_day("2026-11-10", 5),
            heat_day("2026-11-11", 0),
            heat_day("2026-11-12", 0),
        ];
        let stats = super::heatmap_stats(&stale, Some(date("2026-11-10")), date("2026-11-12"));
        assert_eq!(stats.current_streak, 0);
    }

    #[test]
    fn heatmap_busiest_prefers_earliest_on_tie() {
        let days = [
            heat_day("2026-11-10", 9),
            heat_day("2026-11-11", 9),
            heat_day("2026-11-12", 3),
        ];
        let stats = super::heatmap_stats(&days, Some(date("2026-11-10")), date("2026-11-12"));
        let busiest = stats.busiest.expect("busiest");
        assert_eq!(busiest.date, "2026-11-10");
        assert_eq!(busiest.total_tokens, 9);
    }

    #[test]
    fn heatmap_ignores_days_before_first_recorded_in_stats() {
        let days = [
            heat_day("2026-11-09", 100),
            heat_day("2026-11-10", 5),
            heat_day("2026-11-11", 5),
        ];
        let stats = super::heatmap_stats(&days, Some(date("2026-11-10")), date("2026-11-11"));
        assert_eq!(stats.total_tokens, 10);
        assert_eq!(stats.active_days, 2);
        let none = super::heatmap_stats(&days, None, date("2026-11-11"));
        assert_eq!((none.total_tokens, none.active_days), (0, 0));
    }

    #[test]
    fn heatmap_uses_utc_day_boundaries() {
        let db = heatmap_db(&["2026-11-10T23:30:00Z", "2026-11-11T00:30:00Z"]);
        let map = db.activity_heatmap(date("2026-11-12")).expect("heatmap");
        let on = |day: &str| {
            map.days
                .iter()
                .find(|entry| entry.date == day)
                .map(|entry| entry.total_tokens)
        };
        assert_eq!(on("2026-11-10"), Some(110));
        assert_eq!(on("2026-11-11"), Some(110));
        assert_eq!(map.first_recorded.as_deref(), Some("2026-11-10"));
        assert_eq!(map.stats.longest_streak, 2);
    }

    #[test]
    fn project_report_lists_folder_agents_and_last_activity_but_not_unassigned_sessions() {
        let db = heatmap_db(&["2026-11-10T10:00:00Z", "2026-11-11T12:00:00Z"]);
        let unassigned = Project::from_origin(None, "C:/", None);
        assert!(unassigned.is_unassigned());
        db.conn
            .execute(
                "INSERT INTO project(id,canonical_key,repo_root,name) VALUES (?1,?2,?3,?4)",
                rusqlite::params![
                    unassigned.id,
                    unassigned.canonical_key,
                    unassigned.repo_root,
                    unassigned.name
                ],
            )
            .expect("unassigned project");
        db.conn
            .execute(
                "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES ('codex:none','codex','cli',?1,'idle','2026-11-11T00:00:00Z','2026-11-11T00:00:00Z')",
                rusqlite::params![unassigned.id],
            )
            .expect("unassigned session");
        db.conn
            .execute(
                "INSERT INTO usage_record VALUES ('none:1','codex:none','2026-11-11T00:00:00Z','gpt',5,5,0,0,0,0,10,NULL,1.0,0.0,'exact','v')",
                [],
            )
            .expect("unassigned usage");
        let projects = db
            .generate_project_report("all", None, None)
            .expect("project rollups");
        assert_eq!(
            projects.len(),
            1,
            "sessions without a folder are not a project"
        );
        assert_eq!(projects[0].repo_root, "/workspace/gnomcode");
        assert_eq!(projects[0].agents, ["antigravity"]);
        assert!(
            projects[0]
                .last_active
                .as_deref()
                .is_some_and(|stamp| stamp.starts_with("2026-11-11T12:00:00"))
        );
    }

    #[test]
    fn reassigning_a_project_merges_sessions_and_keeps_them() {
        let mut db = heatmap_db(&["2026-11-10T10:00:00Z"]);
        let old = db.list_projects().expect("projects").remove(0);
        let renamed = Project::from_origin(None, "/workspace/renamed", None);
        db.reassign_project(&old.id, &renamed).expect("move");
        let projects = db.list_projects().expect("projects");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "renamed");
        let (rows, total) = db
            .list_sessions_filtered(None, None, Some(&renamed.id), 0, 10)
            .expect("sessions");
        assert_eq!((total, rows[0].project_name.as_str()), (1, "renamed"));
        db.reassign_project(&renamed.id, &renamed)
            .expect("same key");
        assert_eq!(db.list_projects().expect("projects").len(), 1);
    }

    #[test]
    fn resume_folder_is_the_sessions_own_with_a_root_fallback_only_for_repositories() {
        let db = heatmap_db(&["2026-11-10T10:00:00Z"]);
        let add = |id: &str, project: &Project, cwd: Option<&str>| {
            db.conn
                .execute(
                    "INSERT OR IGNORE INTO project(id,canonical_key,repo_root,name) VALUES (?1,?2,?3,?4)",
                    rusqlite::params![project.id, project.canonical_key, project.repo_root, project.name],
                )
                .expect("project");
            db.conn
                .execute(
                    "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at,cwd) VALUES (?1,'codex','cli',?2,'idle','2026-11-11T00:00:00Z','2026-11-11T00:00:00Z',?3)",
                    rusqlite::params![id, project.id, cwd],
                )
                .expect("session");
        };
        let old = "C:\\Users\\W\\Documents\\Codex\\2026-10-14\\old";
        let repo = Project::from_origin(None, "/work/app", None);
        let chats = Project::from_origin(
            None,
            "C:\\Users\\W\\Documents\\Codex\\2026-10-21\\bas",
            None,
        );
        let none = Project::from_origin(None, "C:/", None);
        add("codex:own", &repo, Some("/work/app/crates/x"));
        add("codex:bare-repo", &repo, None);
        add("codex:chat-own", &chats, Some(old));
        add("codex:chat-bare", &chats, None);
        add("codex:unassigned", &none, None);
        let folder = |id: &str| {
            db.get_session_focus_target(id)
                .expect("target")
                .expect("session")
                .repo_root
        };
        assert_eq!(folder("codex:own").as_deref(), Some("/work/app/crates/x"));
        assert_eq!(folder("codex:bare-repo").as_deref(), Some("/work/app"));
        assert_eq!(folder("codex:chat-own").as_deref(), Some(old));
        assert_eq!(
            folder("codex:chat-bare"),
            None,
            "a shared bucket has no folder of its own"
        );
        assert_eq!(folder("codex:unassigned"), None);
    }

    fn pool(
        id: &str,
        window: gnomcode_core::QuotaWindow,
        used: f64,
        now: chrono::DateTime<Utc>,
    ) -> gnomcode_core::QuotaSnapshot {
        gnomcode_core::QuotaSnapshot {
            provider: "antigravity".to_owned(),
            limit_id: id.to_owned(),
            window,
            used_pct: used,
            resets_at: Some(now + chrono::Duration::hours(4)),
            plan: None,
            basis: TrustLevel::Api,
            observed_at: now + chrono::Duration::seconds(30),
        }
    }

    /// Commits Antigravity pools plus one `RESOURCE_EXHAUSTED` error that named `model`, then
    /// returns the cross-checked latest readings as `(pool, window, used, basis)`.
    fn crosscheck(
        model: Option<&str>,
        age: chrono::Duration,
        members: &[(&str, &str)],
        pools: &[gnomcode_core::QuotaSnapshot],
    ) -> Vec<(String, gnomcode_core::QuotaWindow, f64, TrustLevel)> {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut db = Database::open(&dir.path().join("store.db")).expect("open");
        let now = Utc::now();
        let proj = gnomcode_core::Project::from_origin(None, "/repo", None);
        let sess = gnomcode_core::Session {
            id: "antigravity:ag-1".to_owned(),
            agent: gnomcode_core::AgentId::Antigravity,
            surface: gnomcode_core::Surface::Cli,
            project_id: proj.id.clone(),
            parent_session_id: None,
            model: model.map(str::to_owned),
            title: None,
            state: gnomcode_core::SessionState::Error,
            pid: None,
            started_at: now - age,
            last_event_at: now - age,
        };
        let act = gnomcode_core::ActivityEvent {
            session_id: sess.id.clone(),
            timestamp: now - age,
            kind: gnomcode_core::ActivityKind::RateLimited,
            tool_name: model.map(str::to_owned),
            summary_redacted: Some("RESOURCE_EXHAUSTED".to_owned()),
            duration_ms: None,
        };
        let cp = SourceCheckpoint {
            file_path: "ag.jsonl".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "fp".to_owned(),
            last_ingested_at: now,
        };
        db.commit_batch(&[proj], &[sess], &[], &[act], pools, &cp)
            .expect("commit");
        let members: Vec<_> = members
            .iter()
            .map(|(p, m)| ((*p).to_owned(), (*m).to_owned()))
            .collect();
        db.replace_quota_pool_members("antigravity", &members)
            .expect("members");
        db.get_latest_quota_snapshots()
            .expect("latest")
            .into_iter()
            .map(|q| (q.limit_id, q.window, q.used_pct, q.basis))
            .collect()
    }

    const FIVE: gnomcode_core::QuotaWindow = gnomcode_core::QuotaWindow::FiveHour;
    const WEEK: gnomcode_core::QuotaWindow = gnomcode_core::QuotaWindow::Weekly;
    const MEMBERS: [(&str, &str); 5] = [
        ("Gemini Models", "Gemini Models"),
        ("Gemini Models", "Gemini Pro"),
        ("Claude and GPT models", "Claude and GPT models"),
        ("Claude and GPT models", "Claude Opus"),
        ("Claude and GPT models", "GPT-OSS"),
    ];

    fn two_pools(now: chrono::DateTime<Utc>) -> Vec<gnomcode_core::QuotaSnapshot> {
        vec![
            pool("Claude and GPT models", FIVE, 0.0, now),
            pool("Claude and GPT models", WEEK, 10.0, now),
            pool("Gemini Models", FIVE, 20.0, now),
            pool("Gemini Models", WEEK, 30.0, now),
        ]
    }

    #[test]
    fn antigravity_exhausted_error_marks_only_the_pool_of_its_model() {
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 4, "no hint row when the model maps");
        for (id, window, used, basis) in rows {
            if id == "Claude and GPT models" {
                assert_eq!((used, basis), (100.0, TrustLevel::Exact), "{window:?}");
            } else {
                assert_eq!(basis, TrustLevel::Api, "{id} must stay untouched");
                assert!(used < 100.0);
            }
        }
    }

    #[test]
    fn antigravity_exhausted_error_maps_a_model_named_by_the_older_per_model_source() {
        let now = Utc::now();
        let pools = [
            pool("Claude Opus 5.5", FIVE, 0.0, now),
            pool("Gemini 3.1 Pro", FIVE, 0.0, now),
        ];
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &[],
            &pools,
        );
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .any(|r| r.0 == "Claude Opus 5.5" && r.2 == 100.0 && r.3 == TrustLevel::Exact)
        );
        assert!(
            rows.iter()
                .any(|r| r.0 == "Gemini 3.1 Pro" && r.2 == 0.0 && r.3 == TrustLevel::Api)
        );
    }

    #[test]
    fn antigravity_exhausted_error_for_an_unmapped_model_marks_nothing_exact() {
        let rows = crosscheck(
            Some("Mystery Model 9"),
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(
            rows.iter()
                .filter(|r| r.0 != "Mystery Model 9")
                .all(|r| r.2 < 100.0 && r.3 == TrustLevel::Api)
        );
        let hint = rows
            .iter()
            .find(|r| r.0 == "Mystery Model 9")
            .expect("hint row");
        assert_eq!((hint.2, hint.3), (100.0, TrustLevel::Estimated));
    }

    #[test]
    fn antigravity_exhausted_error_without_a_model_is_only_an_estimated_hint() {
        let rows = crosscheck(
            None,
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(
            rows.iter()
                .any(|r| r.0 == "antigravity" && r.3 == TrustLevel::Estimated)
        );
    }

    #[test]
    fn antigravity_exhausted_error_with_no_pool_data_marks_nothing_exact() {
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &[],
            &two_pools(Utc::now()),
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(rows.iter().any(|r| r.3 == TrustLevel::Estimated));
    }

    #[test]
    fn antigravity_exhausted_error_matching_two_pools_is_not_guessed() {
        let now = Utc::now();
        let pools = [
            pool("Claude A", FIVE, 0.0, now),
            pool("Claude B", FIVE, 0.0, now),
        ];
        let members = [("Claude A", "Claude Opus"), ("Claude B", "Claude Opus")];
        let rows = crosscheck(
            Some("Claude Opus 5.5"),
            chrono::Duration::minutes(1),
            &members,
            &pools,
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
    }

    #[test]
    fn antigravity_old_exhausted_error_leaves_readings_and_drops_the_hint() {
        let rows = crosscheck(
            Some("Mystery Model 9"),
            chrono::Duration::hours(6),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.3 == TrustLevel::Api));
    }

    #[test]
    fn pool_members_survive_an_empty_refresh_and_are_replaced_by_a_new_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut db = Database::open(&dir.path().join("store.db")).expect("open");
        let count = |db: &Database| -> i64 {
            db.conn
                .query_row("SELECT COUNT(*) FROM quota_pool_member", [], |row| {
                    row.get(0)
                })
                .expect("count")
        };
        let list = vec![
            ("A".to_owned(), "x".to_owned()),
            ("A".to_owned(), "y".to_owned()),
        ];
        db.replace_quota_pool_members("antigravity", &list)
            .expect("first");
        db.replace_quota_pool_members("antigravity", &[])
            .expect("empty");
        assert_eq!(count(&db), 2);
        db.replace_quota_pool_members("antigravity", &list[..1])
            .expect("replace");
        assert_eq!(count(&db), 1);
    }
}
