//! Console aggregations: usage buckets over time, per-project rollups and KPI summaries.
//!
//! All aggregation lives here (invariant: no business logic in the UI). Every figure
//! carries the weakest trust level among the rows that produced it.

use crate::{Database, Error};
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike, Utc};
use rusqlite::{params_from_iter, types::Value};
use serde::Serialize;

/// Maximum number of bucket labels returned for one query.
const MAX_LABELS: usize = 400;

/// Row filter shared by every aggregation.
#[derive(Debug, Clone, Default)]
pub struct UsageFilter {
    /// Inclusive RFC 3339 lower bound (UTC).
    pub from: Option<String>,
    /// Inclusive RFC 3339 upper bound (UTC).
    pub to: Option<String>,
    /// Agent slugs (empty means all).
    pub agents: Vec<String>,
    /// Model names (empty means all).
    pub models: Vec<String>,
    /// Project names (empty means all).
    pub projects: Vec<String>,
    /// Keep only rows whose cost basis is exact.
    pub exact_only: bool,
}

/// Dimension used to split buckets into series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GroupBy {
    /// One collective series named `total`.
    #[default]
    None,
    /// By agent slug.
    Agent,
    /// By model name.
    Model,
    /// By project name.
    Project,
}

impl GroupBy {
    /// Parses an API value; unknown values fall back to [`GroupBy::None`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "agent" => Self::Agent,
            "model" => Self::Model,
            "project" => Self::Project,
            _ => Self::None,
        }
    }

    const fn expr(self) -> &'static str {
        match self {
            Self::None => "'total'",
            Self::Agent => "s.agent",
            Self::Model => {
                "CASE WHEN u.model LIKE '%MODEL_PLACEHOLDER%' THEN 'Gemini 3.8 Flash (Default)' ELSE u.model END"
            }
            Self::Project => "p.name",
        }
    }
}

/// Time bucket size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interval {
    /// One bucket per hour.
    Hour,
    /// One bucket per local day.
    #[default]
    Day,
    /// One bucket per week starting Monday.
    Week,
}

impl Interval {
    /// Parses an API value; unknown values fall back to [`Interval::Day`].
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s {
            "hour" => Self::Hour,
            "week" => Self::Week,
            _ => Self::Day,
        }
    }
}

/// Token and cost sums with the weakest trust level of the contributing rows.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Totals {
    /// Fresh input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Reasoning tokens (reported separately by some agents).
    pub reasoning_tokens: u64,
    /// Tokens served from cache.
    pub cache_read_tokens: u64,
    /// Tokens written to cache.
    pub cache_write_tokens: u64,
    /// Sum of all token kinds above.
    pub total_tokens: u64,
    /// Spend in USD.
    pub cost_usd: f64,
    /// Distinct sessions with usage.
    pub sessions: u64,
    /// Number of usage records (turns).
    pub turns: u64,
    /// Share of prompt tokens served from cache, 0 to 100 (`None` without prompt tokens).
    pub cache_hit_pct: Option<f64>,
    /// Weakest trust level: `exact`, `api`, `estimated` or `unknown`.
    pub trust: &'static str,
}

/// One bucket and series value for the usage chart.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UsageBucket {
    /// Local bucket label (`YYYY-MM-DD`, or `YYYY-MM-DDTHH:00` for hours).
    pub bucket: String,
    /// Series name (agent, model, project or `total`).
    pub group: String,
    /// Sums for this bucket and series.
    #[serde(flatten)]
    pub totals: Totals,
}

/// Daily (or hourly, weekly) usage series.
#[derive(Debug, Clone, Serialize)]
pub struct UsageSeries {
    /// Every bucket label in range, in order (empty when no lower bound is given).
    pub labels: Vec<String>,
    /// Non-empty buckets.
    pub buckets: Vec<UsageBucket>,
}

/// Per-project rollup row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProjectRollup {
    /// Stable project id.
    pub id: String,
    /// Repository name.
    pub name: String,
    /// Agent slugs seen in this project.
    pub agents: Vec<String>,
    /// Last usage timestamp.
    pub last_active: Option<String>,
    /// Sums for this project.
    #[serde(flatten)]
    pub totals: Totals,
}

/// Percent change versus the previous window (`None` when the previous value is zero).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Deltas {
    /// Spend change.
    pub cost_pct: Option<f64>,
    /// Token change.
    pub tokens_pct: Option<f64>,
    /// Session count change.
    pub sessions_pct: Option<f64>,
}

/// KPI summary for a window and the window before it.
#[derive(Debug, Clone, Serialize)]
pub struct UsageSummary {
    /// Current window.
    pub current: Totals,
    /// Previous window of equal length (needs a lower bound).
    pub previous: Option<Totals>,
    /// Change versus the previous window.
    pub deltas: Deltas,
}

/// Normalizes an RFC 3339 timestamp to UTC with the stored format.
#[must_use]
pub fn normalize_ts(s: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc).to_rfc3339())
}

fn trust_name(rank: i64) -> &'static str {
    match rank {
        0 => "exact",
        1 => "api",
        2 => "estimated",
        _ => "unknown",
    }
}

fn pct_change(cur: f64, prev: f64) -> Option<f64> {
    (prev > 0.0).then(|| (cur - prev) / prev * 100.0)
}

#[allow(clippy::cast_precision_loss)]
fn u64_f(v: u64) -> f64 {
    v as f64
}

fn to_u64(v: i64) -> u64 {
    u64::try_from(v.max(0)).unwrap_or(0)
}

fn push_in(sql: &mut String, params: &mut Vec<Value>, col: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    sql.push_str(" AND ");
    sql.push_str(col);
    sql.push_str(" IN (");
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            sql.push(',');
        }
        sql.push('?');
        params.push(Value::Text(item.clone()));
    }
    sql.push(')');
}

/// Builds the shared WHERE clause (anonymous placeholders only).
fn where_clause(f: &UsageFilter) -> (String, Vec<Value>) {
    let mut sql = String::from(" WHERE 1=1");
    let mut params = Vec::new();
    if let Some(from) = &f.from {
        sql.push_str(" AND u.timestamp >= ?");
        params.push(Value::Text(from.clone()));
    }
    if let Some(to) = &f.to {
        sql.push_str(" AND u.timestamp <= ?");
        params.push(Value::Text(to.clone()));
    }
    push_in(&mut sql, &mut params, "s.agent", &f.agents);
    push_in(&mut sql, &mut params, "u.model", &f.models);
    push_in(&mut sql, &mut params, "p.name", &f.projects);
    if f.exact_only {
        sql.push_str(" AND u.cost_basis = 'exact'");
    }
    (sql, params)
}

const SUM_COLUMNS: &str = "COALESCE(SUM(u.input_tokens),0), COALESCE(SUM(u.output_tokens),0), \
     COALESCE(SUM(u.reasoning_tokens),0), COALESCE(SUM(u.cache_read_tokens),0), \
     COALESCE(SUM(u.cache_write_5m_tokens + u.cache_write_1h_tokens),0), \
     COALESCE(SUM(u.cost_usd),0.0), COUNT(DISTINCT u.session_id), COUNT(*), \
     COALESCE(MAX(CASE u.cost_basis WHEN 'exact' THEN 0 WHEN 'api' THEN 1 \
     WHEN 'estimated' THEN 2 ELSE 3 END),0)";

const FROM_JOIN: &str = " FROM usage_record u JOIN session s ON s.id = u.session_id \
     JOIN project p ON p.id = s.project_id";

/// Reads `SUM_COLUMNS` starting at column `base`.
fn read_totals(row: &rusqlite::Row<'_>, base: usize) -> rusqlite::Result<Totals> {
    let input = to_u64(row.get(base)?);
    let output = to_u64(row.get(base + 1)?);
    let reasoning = to_u64(row.get(base + 2)?);
    let cache_read = to_u64(row.get(base + 3)?);
    let cache_write = to_u64(row.get(base + 4)?);
    let prompt = input.saturating_add(cache_read);
    Ok(Totals {
        input_tokens: input,
        output_tokens: output,
        reasoning_tokens: reasoning,
        cache_read_tokens: cache_read,
        cache_write_tokens: cache_write,
        total_tokens: input
            .saturating_add(output)
            .saturating_add(reasoning)
            .saturating_add(cache_read)
            .saturating_add(cache_write),
        cost_usd: row.get(base + 5)?,
        sessions: to_u64(row.get(base + 6)?),
        turns: to_u64(row.get(base + 7)?),
        cache_hit_pct: (prompt > 0).then(|| u64_f(cache_read) / u64_f(prompt) * 100.0),
        trust: trust_name(row.get(base + 8)?),
    })
}

fn bucket_expr(interval: Interval, tz_offset_minutes: i32) -> String {
    let off = format!("'{tz_offset_minutes:+} minutes'");
    match interval {
        Interval::Hour => format!("strftime('%Y-%m-%dT%H:00', u.timestamp, {off})"),
        Interval::Day => format!("strftime('%Y-%m-%d', u.timestamp, {off})"),
        Interval::Week => {
            format!("strftime('%Y-%m-%d', u.timestamp, {off}, '-6 days', 'weekday 1')")
        }
    }
}

/// Every bucket label between `from` and `to` in local time.
fn bucket_labels(
    from: &str,
    to: Option<&str>,
    interval: Interval,
    tz_offset_minutes: i32,
) -> Vec<String> {
    let off = Duration::minutes(i64::from(tz_offset_minutes));
    let Ok(start) = DateTime::parse_from_rfc3339(from) else {
        return Vec::new();
    };
    let end = to
        .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
        .map_or_else(Utc::now, |d| d.with_timezone(&Utc));
    let start = start.with_timezone(&Utc) + off;
    let end = end + off;
    let mut labels = Vec::new();
    match interval {
        Interval::Hour => {
            let mut cur = start.date_naive().and_hms_opt(start.hour(), 0, 0);
            let last = end.naive_utc();
            while let Some(c) = cur {
                if c > last || labels.len() >= MAX_LABELS {
                    break;
                }
                labels.push(c.format("%Y-%m-%dT%H:00").to_string());
                cur = c.checked_add_signed(Duration::hours(1));
            }
        }
        Interval::Day | Interval::Week => {
            let step = if interval == Interval::Week { 7 } else { 1 };
            let mut cur: NaiveDate = start.date_naive();
            if interval == Interval::Week {
                cur -= Duration::days(i64::from(cur.weekday().num_days_from_monday()));
            }
            let last = end.date_naive();
            while cur <= last && labels.len() < MAX_LABELS {
                labels.push(cur.format("%Y-%m-%d").to_string());
                cur += Duration::days(step);
            }
        }
    }
    labels
}

impl Database {
    /// Usage totals for the filtered rows.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn usage_totals(&self, filter: &UsageFilter) -> Result<Totals, Error> {
        let (wh, params) = where_clause(filter);
        let sql = format!("SELECT {SUM_COLUMNS}{FROM_JOIN}{wh}");
        let totals = self
            .conn
            .query_row(&sql, params_from_iter(params), |row| read_totals(row, 0))?;
        Ok(totals)
    }

    /// Usage over time, split by `group`, bucketed by `interval` in the caller's time zone.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn usage_series(
        &self,
        filter: &UsageFilter,
        group: GroupBy,
        interval: Interval,
        tz_offset_minutes: i32,
    ) -> Result<UsageSeries, Error> {
        let tz = tz_offset_minutes.clamp(-14 * 60, 14 * 60);
        let (wh, params) = where_clause(filter);
        let sql = format!(
            "SELECT {b} AS bucket, {g} AS grp, {SUM_COLUMNS}{FROM_JOIN}{wh} \
             GROUP BY bucket, grp ORDER BY bucket, grp",
            b = bucket_expr(interval, tz),
            g = group.expr(),
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(UsageBucket {
                bucket: row.get(0)?,
                group: row.get(1)?,
                totals: read_totals(row, 2)?,
            })
        })?;
        let mut buckets = Vec::new();
        for r in rows {
            buckets.push(r?);
        }
        let labels = filter
            .from
            .as_deref()
            .map(|f| bucket_labels(f, filter.to.as_deref(), interval, tz))
            .unwrap_or_default();
        Ok(UsageSeries { labels, buckets })
    }

    /// Per-project rollup, highest spend first.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn project_rollups(&self, filter: &UsageFilter) -> Result<Vec<ProjectRollup>, Error> {
        let (wh, params) = where_clause(filter);
        let sql = format!(
            "SELECT p.id, p.name, GROUP_CONCAT(DISTINCT s.agent), MAX(u.timestamp), \
             {SUM_COLUMNS}{FROM_JOIN}{wh} GROUP BY p.id ORDER BY SUM(u.cost_usd) DESC, p.name"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            let agents: Option<String> = row.get(2)?;
            Ok(ProjectRollup {
                id: row.get(0)?,
                name: row.get(1)?,
                agents: agents
                    .map(|a| a.split(',').map(str::to_owned).collect())
                    .unwrap_or_default(),
                last_active: row.get(3)?,
                totals: read_totals(row, 4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// KPI totals for the window plus the previous window of equal length.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn usage_summary(&self, filter: &UsageFilter) -> Result<UsageSummary, Error> {
        let current = self.usage_totals(filter)?;
        let previous = match filter.from.as_deref().and_then(|f| {
            DateTime::parse_from_rfc3339(f)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }) {
            Some(from) => {
                let to = filter
                    .to
                    .as_deref()
                    .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                    .map_or_else(Utc::now, |d| d.with_timezone(&Utc));
                let len = (to - from).max(Duration::seconds(1));
                let prev_to = from - Duration::microseconds(1);
                let mut pf = filter.clone();
                pf.from = Some((from - len).to_rfc3339());
                pf.to = Some(prev_to.to_rfc3339());
                Some(self.usage_totals(&pf)?)
            }
            None => None,
        };
        let deltas = previous.as_ref().map_or_else(Deltas::default, |p| Deltas {
            cost_pct: pct_change(current.cost_usd, p.cost_usd),
            tokens_pct: pct_change(u64_f(current.total_tokens), u64_f(p.total_tokens)),
            sessions_pct: pct_change(u64_f(current.sessions), u64_f(p.sessions)),
        });
        Ok(UsageSummary {
            current,
            previous,
            deltas,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{GroupBy, Interval, UsageFilter, bucket_labels, normalize_ts};
    use crate::Database;
    use chrono::{Duration, Utc};
    use fleethud_core::{
        AgentId, Project, Session, SessionState, Surface, TrustLevel, UsageRecord,
    };

    fn seed(
        db: &mut Database,
        name: &str,
        agent: AgentId,
        days_ago: i64,
        tokens: u64,
        basis: TrustLevel,
    ) {
        let p = Project::from_origin(None, &format!("/work/{name}"), None);
        let ts = Utc::now() - Duration::days(days_ago);
        let sid = format!("{}:{name}:{days_ago}", agent.as_str());
        let s = Session {
            id: sid.clone(),
            agent,
            surface: Surface::Ide,
            project_id: p.id.clone(),
            parent_session_id: None,
            model: Some("m1".to_owned()),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: ts,
            last_event_at: ts,
        };
        let u = UsageRecord {
            dedup_key: format!("k:{sid}"),
            session_id: sid,
            timestamp: ts,
            model: "m1".to_owned(),
            input_tokens: tokens,
            output_tokens: tokens / 2,
            reasoning_tokens: 0,
            cache_read_tokens: tokens,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 0,
            context_window: None,
            cost_usd: 1.0,
            cache_savings_usd: 0.0,
            cost_basis: basis,
            pricing_version: "t".to_owned(),
        };
        let cp = crate::SourceCheckpoint {
            file_path: format!("/log/{name}/{days_ago}"),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "h".to_owned(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch(&[p], &[s], &[u], &[], &[], &cp)
            .expect("commit");
    }

    fn filter_days(days: i64) -> UsageFilter {
        UsageFilter {
            from: Some((Utc::now() - Duration::days(days)).to_rfc3339()),
            ..UsageFilter::default()
        }
    }

    #[test]
    fn series_groups_by_agent_and_fills_labels() {
        let mut db = Database::open_in_memory().expect("db");
        seed(
            &mut db,
            "api",
            AgentId::ClaudeCode,
            1,
            100,
            TrustLevel::Exact,
        );
        seed(&mut db, "api", AgentId::Codex, 1, 300, TrustLevel::Exact);
        seed(&mut db, "web", AgentId::Codex, 10, 50, TrustLevel::Exact);

        let s = db
            .usage_series(&filter_days(7), GroupBy::Agent, Interval::Day, 0)
            .expect("series");
        assert_eq!(s.labels.len(), 8);
        assert_eq!(s.buckets.len(), 2);
        let codex = s
            .buckets
            .iter()
            .find(|b| b.group == "codex")
            .expect("codex");
        assert_eq!(codex.totals.input_tokens, 300);
        assert_eq!(codex.totals.total_tokens, 300 + 150 + 300);

        let total = db
            .usage_series(&filter_days(30), GroupBy::None, Interval::Week, 0)
            .expect("weekly");
        assert!(total.buckets.iter().all(|b| b.group == "total"));
        assert_eq!(total.buckets.iter().map(|b| b.totals.turns).sum::<u64>(), 3);
    }

    #[test]
    fn filters_and_trust_level_apply() {
        let mut db = Database::open_in_memory().expect("db");
        seed(
            &mut db,
            "api",
            AgentId::ClaudeCode,
            1,
            100,
            TrustLevel::Exact,
        );
        seed(
            &mut db,
            "web",
            AgentId::Codex,
            2,
            200,
            TrustLevel::Estimated,
        );

        let all = db.usage_totals(&UsageFilter::default()).expect("all");
        assert_eq!(all.turns, 2);
        assert_eq!(all.trust, "estimated");

        let exact = UsageFilter {
            exact_only: true,
            ..UsageFilter::default()
        };
        let t = db.usage_totals(&exact).expect("exact");
        assert_eq!(t.turns, 1);
        assert_eq!(t.trust, "exact");

        let only_web = UsageFilter {
            projects: vec!["web".to_owned()],
            ..UsageFilter::default()
        };
        assert_eq!(db.usage_totals(&only_web).expect("web").input_tokens, 200);

        let rollup = db.project_rollups(&UsageFilter::default()).expect("rollup");
        assert_eq!(rollup.len(), 2);
        assert!(rollup.iter().all(|r| r.agents.len() == 1));
    }

    #[test]
    fn summary_computes_previous_window_delta() {
        let mut db = Database::open_in_memory().expect("db");
        seed(
            &mut db,
            "api",
            AgentId::ClaudeCode,
            1,
            100,
            TrustLevel::Exact,
        );
        seed(
            &mut db,
            "api",
            AgentId::ClaudeCode,
            2,
            100,
            TrustLevel::Exact,
        );
        seed(
            &mut db,
            "api",
            AgentId::ClaudeCode,
            9,
            100,
            TrustLevel::Exact,
        );
        let sum = db.usage_summary(&filter_days(7)).expect("summary");
        assert_eq!(sum.current.turns, 2);
        let prev = sum.previous.expect("previous");
        assert_eq!(prev.turns, 1);
        assert_eq!(sum.deltas.cost_pct, Some(100.0));
        let none = db.usage_summary(&UsageFilter::default()).expect("no range");
        assert!(none.previous.is_none());
        assert_eq!(none.deltas.cost_pct, None);
    }

    #[test]
    fn timestamps_normalize_and_labels_respect_offset() {
        assert_eq!(
            normalize_ts("2026-10-26T05:00:00+05:00").as_deref(),
            Some("2026-10-26T00:00:00+00:00")
        );
        assert!(normalize_ts("not a date").is_none());
        let l = bucket_labels(
            "2026-10-26T22:00:00+00:00",
            Some("2026-10-27T01:00:00+00:00"),
            Interval::Day,
            300,
        );
        assert_eq!(l, vec!["2026-10-27".to_owned()]);
        let h = bucket_labels(
            "2026-10-26T22:30:00+00:00",
            Some("2026-10-27T00:10:00+00:00"),
            Interval::Hour,
            0,
        );
        assert_eq!(h.len(), 3);
        assert_eq!(Interval::parse("week"), Interval::Week);
        assert_eq!(GroupBy::parse("x"), GroupBy::None);
    }
}
