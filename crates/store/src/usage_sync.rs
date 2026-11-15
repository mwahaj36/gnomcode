//! Incremental usage history backup watermark and idempotent restore (`SPEC §3.15`, ADR 0065).

use super::*;
use serde::{Deserialize, Serialize};

const WATERMARK_PREF_KEY: &str = "usage_sync_watermark";

/// Persisted incremental sync cursor in `desktop_preferences`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageSyncWatermark {
    /// Highest `usage_record.rowid` acknowledged by Supabase.
    pub last_usage_rowid: i64,
    /// Highest `session.last_event_at` acknowledged by Supabase.
    pub last_session_event_at: Option<String>,
}

/// Privacy-bounded session metadata row for `gnomcode_session_meta`.
///
/// Never includes session titles (derived from user prompts), chat events, or full file paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudSessionMeta {
    /// Native or conversation session identifier.
    pub session_id: String,
    /// Canonical agent identifier (`claude-code`, `codex`, `antigravity`).
    pub agent: String,
    /// Originating surface (`cli`, `desktop_app`, `ide`, etc.).
    pub surface: String,
    /// Last observed model name, if any.
    pub model: Option<String>,
    /// Folder basename only (never full filesystem path).
    pub project_name: String,
    /// RFC 3339 UTC session start timestamp.
    pub started_at: String,
    /// RFC 3339 UTC last event timestamp.
    pub last_event_at: String,
}

/// Privacy-bounded per-turn usage row for `gnomcode_usage_record`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloudUsageRecord {
    /// Idempotent deduplication key (`(user_id, dedup_key)` primary key in Supabase).
    pub dedup_key: String,
    /// Session identifier this usage belongs to.
    pub session_id: String,
    /// RFC 3339 UTC turn timestamp.
    pub timestamp: String,
    /// Model identifier.
    pub model: String,
    /// Uncached input tokens.
    pub input_tokens: i64,
    /// Output tokens.
    pub output_tokens: i64,
    /// Reasoning tokens.
    pub reasoning_tokens: i64,
    /// Prompt cache read tokens.
    pub cache_read_tokens: i64,
    /// Prompt cache 5-minute write tokens.
    pub cache_write_5m_tokens: i64,
    /// Prompt cache 1-hour write tokens.
    pub cache_write_1h_tokens: i64,
    /// Context tokens at turn end.
    pub context_tokens: i64,
    /// Model context window limit, if known.
    pub context_window: Option<i64>,
    /// API-equivalent cost in USD.
    pub cost_usd: f64,
    /// Estimated cache savings in USD.
    pub cache_savings_usd: f64,
    /// Measurement trust level (`exact`, `estimated`, `api`, `unknown`).
    pub cost_basis: String,
    /// Pricing catalog version string.
    pub pricing_version: String,
}

impl Database {
    /// Returns the total number of local `usage_record` rows.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn local_usage_record_count(&self) -> Result<i64, Error> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM usage_record", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Reads the persisted incremental usage sync watermark.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_usage_sync_watermark(&self) -> Result<UsageSyncWatermark, Error> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM desktop_preferences WHERE key = ?1",
                [WATERMARK_PREF_KEY],
                |row| row.get(0),
            )
            .optional()?;
        Ok(raw
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default())
    }

    /// Persists the incremental usage sync watermark.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write or JSON serialization failure.
    pub fn set_usage_sync_watermark(&self, watermark: &UsageSyncWatermark) -> Result<(), Error> {
        let json = serde_json::to_string(watermark)?;
        self.conn.execute(
            "INSERT INTO desktop_preferences(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![WATERMARK_PREF_KEY, json],
        )?;
        Ok(())
    }

    /// Resets the incremental usage sync watermark to zero.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write failure.
    pub fn reset_usage_sync_watermark(&self) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM desktop_preferences WHERE key = ?1",
            [WATERMARK_PREF_KEY],
        )?;
        Ok(())
    }

    /// Reads up to `limit` local usage records strictly after `after_rowid`.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn read_usage_sync_batch(
        &self,
        after_rowid: i64,
        limit: usize,
    ) -> Result<Vec<(i64, CloudUsageRecord)>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT u.rowid, u.dedup_key, u.session_id, u.timestamp, u.model,
                    u.input_tokens, u.output_tokens, u.reasoning_tokens,
                    u.cache_read_tokens, u.cache_write_5m_tokens, u.cache_write_1h_tokens,
                    u.context_tokens, u.context_window, u.cost_usd, u.cache_savings_usd,
                    u.cost_basis, u.pricing_version
             FROM usage_record u
             JOIN session s ON s.id = u.session_id
             WHERE u.rowid > ?1 AND s.surface <> 'cloud'
             ORDER BY u.rowid ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![after_rowid, limit as i64], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                CloudUsageRecord {
                    dedup_key: row.get(1)?,
                    session_id: row.get(2)?,
                    timestamp: row.get(3)?,
                    model: row.get(4)?,
                    input_tokens: row.get(5)?,
                    output_tokens: row.get(6)?,
                    reasoning_tokens: row.get(7)?,
                    cache_read_tokens: row.get(8)?,
                    cache_write_5m_tokens: row.get(9)?,
                    cache_write_1h_tokens: row.get(10)?,
                    context_tokens: row.get(11)?,
                    context_window: row.get(12)?,
                    cost_usd: row.get(13)?,
                    cache_savings_usd: row.get(14)?,
                    cost_basis: row.get(15)?,
                    pricing_version: row.get(16)?,
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Reads up to `limit` session metadata rows at or after `since_last_event_at`.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn read_session_meta_batch(
        &self,
        since_last_event_at: Option<&str>,
        limit: usize,
    ) -> Result<Vec<CloudSessionMeta>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.agent, s.surface, s.model, p.name, s.started_at, s.last_event_at
             FROM session s
             JOIN project p ON p.id = s.project_id
             WHERE s.surface <> 'cloud'
               AND (?1 IS NULL OR s.last_event_at > ?1)
             ORDER BY s.last_event_at ASC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![since_last_event_at, limit as i64], |row| {
            Ok(CloudSessionMeta {
                session_id: row.get(0)?,
                agent: row.get(1)?,
                surface: row.get(2)?,
                model: row.get(3)?,
                project_name: row.get(4)?,
                started_at: row.get(5)?,
                last_event_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Idempotently restores cloud session metadata and usage records into the local database.
    ///
    /// Returns `(restored_records, restored_sessions)`. Safe to run multiple times.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite transaction failure.
    pub fn restore_cloud_usage_history(
        &mut self,
        sessions: &[CloudSessionMeta],
        records: &[CloudUsageRecord],
    ) -> Result<(usize, usize), Error> {
        let tx = self.conn.transaction()?;
        let mut restored_sessions = 0usize;
        let mut restored_records = 0usize;

        for meta in sessions {
            let project_name = if meta.project_name.trim().is_empty() {
                "restored".to_owned()
            } else {
                meta.project_name.trim().to_owned()
            };
            let canonical_key = format!("cloud/{}", project_name.to_ascii_lowercase());
            let project_id = format!("cloud:{}", sha256_hex(&canonical_key));
            tx.execute(
                "INSERT OR IGNORE INTO project(id, canonical_key, repo_root, name)
                 VALUES (?1, ?2, '', ?3)",
                params![project_id, canonical_key, project_name],
            )?;
            let changed = tx.execute(
                "INSERT OR IGNORE INTO session
                 (id, agent, surface, project_id, parent_session_id, model, title, state, pid, started_at, last_event_at)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, 'ended', NULL, ?6, ?7)",
                params![
                    meta.session_id,
                    meta.agent,
                    meta.surface,
                    project_id,
                    meta.model,
                    meta.started_at,
                    meta.last_event_at,
                ],
            )?;
            restored_sessions += changed;
        }

        if !records.is_empty() {
            let fallback_project_id = format!("cloud:{}", sha256_hex("cloud/restored"));
            tx.execute(
                "INSERT OR IGNORE INTO project(id, canonical_key, repo_root, name)
                 VALUES (?1, 'cloud/restored', '', 'restored')",
                params![fallback_project_id],
            )?;
            for record in records {
                let changed_session = tx.execute(
                    "INSERT OR IGNORE INTO session
                     (id, agent, surface, project_id, parent_session_id, model, title, state, pid, started_at, last_event_at)
                     VALUES (?1, 'unknown', 'cli', ?2, NULL, ?3, NULL, 'ended', NULL, ?4, ?4)",
                    params![
                        record.session_id,
                        fallback_project_id,
                        record.model,
                        record.timestamp,
                    ],
                )?;
                restored_sessions += changed_session;
                let changed = tx.execute(
                    "INSERT OR IGNORE INTO usage_record
                     (dedup_key, session_id, timestamp, model,
                      input_tokens, output_tokens, reasoning_tokens,
                      cache_read_tokens, cache_write_5m_tokens, cache_write_1h_tokens,
                      context_tokens, context_window, cost_usd, cache_savings_usd,
                      cost_basis, pricing_version)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                    params![
                        record.dedup_key,
                        record.session_id,
                        record.timestamp,
                        record.model,
                        record.input_tokens,
                        record.output_tokens,
                        record.reasoning_tokens,
                        record.cache_read_tokens,
                        record.cache_write_5m_tokens,
                        record.cache_write_1h_tokens,
                        record.context_tokens,
                        record.context_window,
                        record.cost_usd,
                        record.cache_savings_usd,
                        record.cost_basis,
                        record.pricing_version,
                    ],
                )?;
                restored_records += changed;
            }
        }

        tx.commit()?;
        Ok((restored_records, restored_sessions))
    }
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(16);
    for b in &digest[..8] {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_usage(db: &mut Database, count: usize) {
        let project = Project::from_origin(None, "C:\\work\\my-repo", None);
        let session = Session {
            id: "codex:sync-test".to_owned(),
            agent: AgentId::Codex,
            surface: Surface::DesktopApp,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("gpt-5".to_owned()),
            title: Some("Private prompt title must never sync".to_owned()),
            state: SessionState::Idle,
            pid: None,
            started_at: Utc::now(),
            last_event_at: Utc::now(),
        };
        let cp = SourceCheckpoint {
            file_path: "rollout.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fp".to_owned(),
            last_ingested_at: Utc::now(),
        };
        let records: Vec<UsageRecord> = (0..count)
            .map(|i| UsageRecord {
                dedup_key: format!("codex:sync-test:{i}"),
                session_id: "codex:sync-test".to_owned(),
                timestamp: Utc::now(),
                model: "gpt-5".to_owned(),
                input_tokens: 100 + i as u64,
                output_tokens: 50,
                reasoning_tokens: 0,
                cache_read_tokens: 25,
                cache_write_5m_tokens: 0,
                cache_write_1h_tokens: 0,
                context_tokens: 150,
                context_window: Some(200_000),
                cost_usd: 0.01,
                cache_savings_usd: 0.002,
                cost_basis: TrustLevel::Exact,
                pricing_version: "v1".to_owned(),
            })
            .collect();
        db.commit_batch(&[project], &[session], &records, &[], &[], &cp)
            .expect("commit");
    }

    #[test]
    fn watermark_persists_and_batches_incrementally_without_titles() {
        let mut db = Database::open_in_memory().expect("db");
        assert_eq!(
            db.get_usage_sync_watermark().expect("wm").last_usage_rowid,
            0
        );

        seed_usage(&mut db, 5);
        let first = db.read_usage_sync_batch(0, 3).expect("batch 1");
        assert_eq!(first.len(), 3);
        let last_rowid = first.last().expect("last").0;
        db.set_usage_sync_watermark(&UsageSyncWatermark {
            last_usage_rowid: last_rowid,
            last_session_event_at: None,
        })
        .expect("save wm");

        let reloaded = db.get_usage_sync_watermark().expect("reload");
        assert_eq!(reloaded.last_usage_rowid, last_rowid);

        let second = db
            .read_usage_sync_batch(reloaded.last_usage_rowid, 10)
            .expect("batch 2");
        assert_eq!(second.len(), 2);

        let metas = db.read_session_meta_batch(None, 10).expect("metas");
        assert_eq!(metas.len(), 1);
        let json = serde_json::to_string(&metas[0]).expect("json");
        assert!(!json.contains("Private prompt title"));
        assert!(!json.contains("title"));
    }

    #[test]
    fn restore_is_idempotent_on_repeated_runs() {
        let mut fresh = Database::open_in_memory().expect("fresh db");
        let sessions = vec![CloudSessionMeta {
            session_id: "codex:restored-1".to_owned(),
            agent: "codex".to_owned(),
            surface: "desktop_app".to_owned(),
            model: Some("gpt-5".to_owned()),
            project_name: "my-repo".to_owned(),
            started_at: "2026-11-15T10:00:00Z".to_owned(),
            last_event_at: "2026-11-15T10:05:00Z".to_owned(),
        }];
        let records = vec![CloudUsageRecord {
            dedup_key: "codex:restored-1:0".to_owned(),
            session_id: "codex:restored-1".to_owned(),
            timestamp: "2026-11-15T10:01:00Z".to_owned(),
            model: "gpt-5".to_owned(),
            input_tokens: 1000,
            output_tokens: 200,
            reasoning_tokens: 0,
            cache_read_tokens: 500,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
            context_tokens: 1200,
            context_window: Some(200_000),
            cost_usd: 0.05,
            cache_savings_usd: 0.01,
            cost_basis: "exact".to_owned(),
            pricing_version: "v1".to_owned(),
        }];

        let (rec1, ses1) = fresh
            .restore_cloud_usage_history(&sessions, &records)
            .expect("first restore");
        assert_eq!(rec1, 1);
        assert_eq!(ses1, 1);
        assert_eq!(fresh.local_usage_record_count().expect("count"), 1);

        let (rec2, ses2) = fresh
            .restore_cloud_usage_history(&sessions, &records)
            .expect("second restore");
        assert_eq!(rec2, 0);
        assert_eq!(ses2, 0);
        assert_eq!(fresh.local_usage_record_count().expect("count"), 1);
    }
}
