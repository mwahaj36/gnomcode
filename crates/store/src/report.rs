//! Usage reports, activity heatmap and database statistics.

use super::*;

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

pub(crate) fn group_activity(
    rows: Vec<DailyUsage>,
    range: &str,
) -> (Vec<DailyUsage>, &'static str) {
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

pub(crate) fn unknown_cost_basis() -> TrustLevel {
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

impl Database {
    /// Aggregates usage report grouped by project, agent, and model for `gnomcode report`.
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
            WHERE s.surface <> 'cloud'
            GROUP BY p.id, s.agent, u.model
            ORDER BY total_cost DESC
            "#,
        )?;

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

    /// Per-bucket usage rows shared by the timeline report and the activity grid:
    /// tokens including cache exactly once, recorded cost and turns per UTC day
    /// (per UTC hour when `range_kind` is `today`).
    pub(crate) fn daily_usage_rows(
        &self,
        range_kind: &str,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<DailyUsage>, Error> {
        let mut statement = self.conn.prepare(r#"
            SELECT CASE WHEN ?3 = 'today' THEN strftime('%Y-%m-%dT%H:00:00Z', u.timestamp) ELSE date(u.timestamp) END,
                SUM(u.input_tokens + u.output_tokens + CASE WHEN s.agent IN ('codex', 'gemini-cli') THEN 0
                    ELSE u.cache_read_tokens + u.cache_write_5m_tokens + u.cache_write_1h_tokens END),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 0 ELSE u.cost_usd END), COUNT(*),
                SUM(CASE WHEN u.cost_basis = 'unknown' THEN 1 ELSE 0 END)
            FROM usage_record u JOIN session s ON s.id = u.session_id
            WHERE s.surface <> 'cloud' AND (?1 IS NULL OR julianday(u.timestamp) >= julianday(?1)) AND (?2 IS NULL OR julianday(u.timestamp) <= julianday(?2))
            GROUP BY 1 ORDER BY 1
        "#)?;
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
                "SELECT MIN(date(u.timestamp)) FROM usage_record u JOIN session s ON s.id = u.session_id WHERE s.surface <> 'cloud'",
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

        let mut stmt = self.conn.prepare(r#"
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
            "#)?;

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

/// Shared test fixture for report, project and session tests.
#[cfg(test)]
pub(crate) fn heatmap_db(timestamps: &[&str]) -> Database {
    let mut db = Database::open_in_memory().expect("in-memory db");
    let project = Project::from_origin(
        Some("https://github.com/mwahaj36/gnomcode"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::{Database, SourceCheckpoint};
    use chrono::Utc;
    use gnomcode_core::{
        AgentId, Project, Session, SessionState, Surface, TrustLevel, UsageRecord,
    };

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
    fn usage_report_timeline_honors_explicit_from_ts_and_local_start_of_day() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let p = Project::from_origin(
            Some("https://github.com/mwahaj36/gnomcode"),
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
}
