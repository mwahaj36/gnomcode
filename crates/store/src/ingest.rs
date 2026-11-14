//! Source checkpoints and batch commits.

use super::*;

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

impl Database {
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
            let mut stmt = tx.prepare_cached(r#"
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
                    title = CASE WHEN session.agent = 'antigravity' AND coalesce(session.title,'') NOT IN ('','Subagent') THEN session.title ELSE coalesce(excluded.title, session.title) END,
                    state = excluded.state,
                    pid = coalesce(excluded.pid, session.pid),
                    last_event_at = excluded.last_event_at
                "#)?;
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
            tx.execute("INSERT OR IGNORE INTO chat_event (event_id, session_id, turn_index, role, content, tool_name, status, created_at) SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8 WHERE NOT EXISTS (SELECT 1 FROM deleted_chat_event WHERE event_id=?9)", params![event.event_id, event.session_id, event.turn_index, event.role.as_str(), content, event.tool_name.as_deref().map(gnomcode_core::redact::redact_chat_content), event.status.as_deref().map(gnomcode_core::redact::redact_chat_content), event.created_at.to_rfc3339(), event_marker(&event.event_id)])?;
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

    /// Finds the ingested source file path for a Codex rollout session id.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn find_codex_rollout_path(&self, raw_session_id: &str) -> Result<Option<String>, Error> {
        let needle = raw_session_id.trim();
        if needle.is_empty() {
            return Ok(None);
        }
        let pattern = format!("%{needle}%");
        self.conn
            .query_row(
                obfstr::obfstr!(
                    "SELECT file_path FROM source_file WHERE file_path LIKE ?1 ORDER BY last_ingested_at DESC LIMIT 1"
                ),
                params![pattern],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
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
}

#[cfg(test)]
mod tests {
    use super::{Database, SourceCheckpoint};
    use chrono::Utc;
    use gnomcode_core::{
        ActivityEvent, ActivityKind, AgentId, Project, QuotaSnapshot, QuotaWindow, Session,
        SessionState, Surface, TrustLevel, UsageRecord,
    };

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
}
