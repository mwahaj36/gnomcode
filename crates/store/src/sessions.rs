//! Session listing, focus and metadata queries.

use super::*;

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
    /// Rolled-up label while subagents run, for example `Working · 2 subagents running`.
    #[serde(default)]
    pub state_label: Option<String>,
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
    /// Display name derived in Rust: the recorded description or `Subagent N`.
    #[serde(default)]
    pub name: String,
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
        self.list_sessions_filtered_with_hosts(
            agent_filter,
            status_filter,
            project_filter,
            &[],
            offset,
            limit,
        )
    }

    /// Lists sessions with optional filtering and live Gnomcode-hosted native session IDs.
    ///
    /// # Errors
    /// Returns an error on SQLite failure.
    pub fn list_sessions_paginated_with_hosts(
        &self,
        agent_filter: Option<&str>,
        status_filter: Option<&str>,
        live_hosted_ids: &[String],
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SessionListItem>, usize), Error> {
        self.list_sessions_filtered_with_hosts(
            agent_filter,
            status_filter,
            None,
            live_hosted_ids,
            offset,
            limit,
        )
    }

    /// Lists sessions with an optional exact local project ID and live hosted native session IDs.
    ///
    /// # Errors
    /// Returns an error on SQLite failure.
    pub fn list_sessions_filtered_with_hosts(
        &self,
        agent_filter: Option<&str>,
        status_filter: Option<&str>,
        project_filter: Option<&str>,
        live_hosted_ids: &[String],
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SessionListItem>, usize), Error> {
        self.list_sessions_filtered_at(
            agent_filter,
            status_filter,
            project_filter,
            live_hosted_ids,
            Utc::now(),
            offset,
            limit,
        )
    }

    /// Lists sessions at a caller-supplied timestamp `now` with live hosted native session IDs.
    ///
    /// # Errors
    /// Returns an error on SQLite failure.
    #[allow(clippy::too_many_arguments)]
    pub fn list_sessions_filtered_at(
        &self,
        agent_filter: Option<&str>,
        status_filter: Option<&str>,
        project_filter: Option<&str>,
        live_hosted_ids: &[String],
        now: DateTime<Utc>,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<SessionListItem>, usize), Error> {
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
        let hosted_tool_clause = |start_param: usize| {
            if live_hosted_ids.is_empty() {
                "0".to_owned()
            } else {
                let marks = (0..live_hosted_ids.len())
                    .map(|idx| format!("?{}", start_param + idx))
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    "{} ({marks}))",
                    obfstr::obfstr!(
                        "(s.state = 'tool' AND (SELECT kind FROM activity_event ae WHERE ae.session_id = s.id ORDER BY julianday(ae.timestamp) DESC, ae.id DESC LIMIT 1) = 'tool_start' AND s.id IN"
                    )
                )
            }
        };
        let child_count = child_clause("?5", "?6");
        let child_page = child_clause("?7", "?8");
        let hosted_tool_count = hosted_tool_clause(7);
        let hosted_tool_page = hosted_tool_clause(9);
        let mut count_params: Vec<rusqlite::types::Value> = vec![
            agent.to_owned().into(),
            status.to_owned().into(),
            cutoff.clone().into(),
            project_filter.map(str::to_owned).into(),
            stale_cutoff.clone().into(),
            execution_cutoff.clone().into(),
        ];
        count_params.extend(live_hosted_ids.iter().cloned().map(Into::into));
        let total_count: i64 = self.conn.query_row(
            &format!(r#"SELECT COUNT(*) FROM session s
               WHERE (?4 IS NULL OR s.project_id = ?4) AND s.surface <> 'cloud' AND s.parent_session_id IS NULL AND s.id NOT GLOB 'codex:ctc_*' AND s.id NOT GLOB 'codex:ctco_*' AND s.id NOT GLOB 'codex:fc_*' AND s.id NOT GLOB 'codex:call_*' AND s.id NOT GLOB 'codex:rs_*' AND s.id NOT GLOB 'codex:msg_*' AND (?1 = 'all' OR s.agent = ?1)
                 AND (?2 = 'all'
                   OR (?2 = 'ended' AND s.state = 'ended')
                   OR (?2 = 'awaiting_approval' AND s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5))
                   OR (?2 = 'ongoing' AND ({child_count} OR {hosted_tool_count} OR (s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?6))) OR (s.state = 'idle' AND julianday(s.last_event_at) >= julianday(?3))))
                   OR (?2 = 'active' AND ({child_count} OR {hosted_tool_count} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?5) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?6))))
                   OR (?2 = 'idle' AND s.state = 'idle'))"#),
            rusqlite::params_from_iter(count_params),
            |row| row.get(0),
        )?;
        let mut stmt = self.conn.prepare(
            &format!(r#"
            WITH page AS (
                SELECT s.*, p.name AS project_name,
                    CASE WHEN ({child_page} OR {hosted_tool_page} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))) THEN 1 ELSE 0 END AS active_rank
                FROM session s JOIN project p ON s.project_id = p.id
                WHERE (?4 IS NULL OR s.project_id = ?4) AND s.surface <> 'cloud' AND s.parent_session_id IS NULL AND s.id NOT GLOB 'codex:ctc_*' AND s.id NOT GLOB 'codex:ctco_*' AND s.id NOT GLOB 'codex:fc_*' AND s.id NOT GLOB 'codex:call_*' AND s.id NOT GLOB 'codex:rs_*' AND s.id NOT GLOB 'codex:msg_*' AND (?1 = 'all' OR s.agent = ?1)
                  AND (?2 = 'all'
                    OR (?2 = 'ended' AND s.state = 'ended')
                   OR (?2 = 'awaiting_approval' AND s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7))
                   OR (?2 = 'ongoing' AND ({child_page} OR {hosted_tool_page} OR (s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))) OR (s.state = 'idle' AND julianday(s.last_event_at) >= julianday(?3))))
                   OR (?2 = 'active' AND ({child_page} OR {hosted_tool_page} OR s.state = 'awaiting_approval' AND julianday(s.last_event_at) >= julianday(?7) OR (s.state IN ('thinking','tool') AND julianday(s.last_event_at) >= julianday(?8))))
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

        let mut page_params: Vec<rusqlite::types::Value> = vec![
            agent.to_owned().into(),
            status.to_owned().into(),
            cutoff.into(),
            project_filter.map(str::to_owned).into(),
            (limit as i64).into(),
            (offset as i64).into(),
            stale_cutoff.into(),
            execution_cutoff.into(),
        ];
        page_params.extend(live_hosted_ids.iter().cloned().map(Into::into));
        let rows = stmt.query_map(rusqlite::params_from_iter(page_params), |row| {
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
            let host_alive = live_hosted_ids.iter().any(|live_id| live_id == &id);
            let presence = gnomcode_core::presence::classify_presence_with_host(
                &state_str,
                age,
                open_execution,
                host_alive,
            );
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
                surface_label: gnomcode_core::surface_label(&agent, &surface).map(str::to_owned),
                id,
                agent,
                surface: surface.clone(),
                project_name,
                model,
                state,
                state_label: None,
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
        })?;

        let mut filtered = Vec::new();
        for r in rows {
            filtered.push(r?);
        }
        self.attach_subagents(&mut filtered, now)?;
        Ok((filtered, total_count.max(0) as usize))
    }

    /// Nests each session's subagents and lifts a parent with a live subagent into the
    /// ongoing groups, since the parent's own log is quiet while a subagent works.
    pub(crate) fn attach_subagents(
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
                        name: String::new(),
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
        let mut running_children: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for row in rows {
            let (parent, child) = row?;
            let Some(item) = items.iter_mut().find(|item| item.id == parent) else {
                continue;
            };
            item.subagent_count += 1;
            item.subagents_ongoing += usize::from(child.is_ongoing);
            running_children
                .entry(item.id.clone())
                .and_modify(|count| *count += usize::from(child.is_active))
                .or_insert_with(|| usize::from(child.is_active));
            if item.subagents.len() < SUBAGENT_LIST_LIMIT {
                item.subagents.push(child);
            }
        }
        for item in items.iter_mut() {
            // Names count from the oldest sibling so `Subagent N` stays stable as new ones start.
            let mut order: Vec<usize> = (0..item.subagents.len()).collect();
            order.sort_by(|a, b| {
                item.subagents[*a]
                    .started_at
                    .cmp(&item.subagents[*b].started_at)
                    .then_with(|| item.subagents[*a].id.cmp(&item.subagents[*b].id))
            });
            for (ordinal, index) in order.into_iter().enumerate() {
                let child = &mut item.subagents[index];
                child.name =
                    gnomcode_core::subagent::subagent_name(child.title.as_deref(), ordinal + 1);
            }
            let running = running_children.get(&item.id).copied().unwrap_or(0);
            if running > 0 && item.state != "awaiting_approval" {
                item.state_label = Some(gnomcode_core::subagent::working_label(running));
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
        assert_eq!(quiet.subagents[0].name, "Subagent 1");
        assert_eq!(
            quiet.state_label.as_deref(),
            Some("Working · 1 subagent running"),
            "an idle parent reflects its running child"
        );
        let (all, total) = db.list_sessions_paginated(None, None, 0, 10).expect("all");
        assert_eq!(total, 2, "subagents are nested, never top-level rows");
        let done = all
            .iter()
            .find(|row| row.id == "claude-code:done")
            .expect("done");
        assert!(!done.is_ongoing);
        assert_eq!(done.state_label, None, "no running child, no rollup");
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
        for agent in [AgentId::ClaudeCode, AgentId::Codex, AgentId::Antigravity] {
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
    fn rows_from_removed_agents_stay_readable_and_never_crash_queries() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(None, "/workspace/legacy", None);
        let session = Session {
            id: "gemini-cli:legacy".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gemini-2.5-pro".to_owned()),
            title: Some("Old chat".to_owned()),
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let usage = UsageRecord {
            dedup_key: "legacy:1".to_owned(),
            session_id: session.id.clone(),
            timestamp: Utc::now(),
            model: "gemini-2.5-pro".to_owned(),
            input_tokens: 10,
            output_tokens: 5,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 15,
            context_window: None,
            cost_usd: 0.01,
            cache_savings_usd: 0.0,
            cost_basis: TrustLevel::Estimated,
            pricing_version: "test".to_owned(),
        };
        let checkpoint = SourceCheckpoint {
            file_path: "/legacy/session.json".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "legacy".to_owned(),
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
        .expect("store session");
        // Simulate rows written by a removed agent before the 1.0 scope cut.
        db.conn
            .execute("UPDATE session SET agent = 'gemini-cli'", [])
            .expect("relabel as removed agent");

        assert!(AgentId::parse("gemini-cli").is_none());
        assert!(db.list_active_sessions(10).expect("active").is_empty());
        let (rows, count) = db
            .list_sessions_paginated(None, None, 0, 10)
            .expect("paginated");
        assert_eq!((rows.len(), count), (1, 1));
        db.generate_usage_report().expect("report");
        assert_eq!(
            gnomcode_core::pricing::format_agent_display("gemini-cli"),
            "Unsupported agent"
        );
        let remaining: i64 = db
            .conn
            .query_row(
                "SELECT COUNT(*) FROM session WHERE agent = 'gemini-cli'",
                [],
                |r| r.get(0),
            )
            .expect("count");
        assert_eq!(remaining, 1, "no user data is deleted");
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

    #[test]
    fn hosted_alive_open_tool_stays_running_past_cutoff_and_counts_match() {
        let db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(None, "/work/hosted-presence", None);
        db.conn
            .execute(
                "INSERT INTO project(id,canonical_key,repo_root,name) VALUES (?1,?2,?3,?4)",
                rusqlite::params![
                    project.id,
                    project.canonical_key,
                    project.repo_root,
                    project.name
                ],
            )
            .expect("project");
        let now = DateTime::parse_from_rfc3339("2026-11-13T12:00:00Z")
            .expect("now")
            .with_timezone(&Utc);
        let old = (now - chrono::Duration::minutes(25)).to_rfc3339();
        let insert_tool = |id: &str, kind: &str| {
            db.conn
                .execute(
                    "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES (?1,'claude-code','cli',?2,'tool',?3,?3)",
                    rusqlite::params![id, project.id, old],
                )
                .expect("session");
            db.conn
                .execute(
                    "INSERT INTO activity_event(session_id,timestamp,kind) VALUES (?1,?2,?3)",
                    rusqlite::params![id, old, kind],
                )
                .expect("event");
        };
        insert_tool("hosted-alive", "tool_start");
        insert_tool("hosted-dead", "tool_start");
        insert_tool("non-hosted", "tool_start");

        db.bind_conversation_host("h-alive", "hosted-alive")
            .expect("bind");
        let live_hosted = db
            .native_ids_for_hosts(&["h-alive".to_owned()])
            .expect("live native ids");
        assert_eq!(live_hosted, vec!["hosted-alive".to_owned()]);

        let (all_rows, all_total) = db
            .list_sessions_filtered_at(None, None, None, &live_hosted, now, 0, 10)
            .expect("all");
        assert_eq!(all_total, 3);
        assert_eq!(all_rows.len(), 3);
        let by_id = |id: &str| all_rows.iter().find(|r| r.id == id).expect("row");
        assert_eq!(
            by_id("hosted-alive").presence,
            gnomcode_core::presence::SessionPresence::Running
        );
        assert!(by_id("hosted-alive").is_active);
        assert_eq!(
            by_id("hosted-dead").presence,
            gnomcode_core::presence::SessionPresence::Inactive
        );
        assert_eq!(by_id("hosted-dead").state, "unknown");
        assert_eq!(
            by_id("non-hosted").presence,
            gnomcode_core::presence::SessionPresence::Inactive
        );
        assert_eq!(by_id("non-hosted").state, "unknown");

        for filter in ["active", "ongoing"] {
            let (rows, total) = db
                .list_sessions_filtered_at(None, Some(filter), None, &live_hosted, now, 0, 10)
                .expect("filtered");
            assert_eq!(total, 1, "filter {filter} count");
            assert_eq!(rows.len(), 1, "filter {filter} rows");
            assert_eq!(rows[0].id, "hosted-alive");
        }
    }
}
