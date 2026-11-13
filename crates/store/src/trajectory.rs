use super::*;

/// Raw rows read from an Antigravity conversation database (`conversations/<id>.db`).
#[derive(Debug, Default)]
pub struct TrajectoryRows {
    /// `gen_metadata` rows `(idx, data)` with `idx` above the requested start.
    pub generations: Vec<(i64, Vec<u8>)>,
    /// `steps.metadata` blobs, used to date each model call.
    pub step_metadata: Vec<Vec<u8>>,
}

fn open_trajectory(path: &Path) -> Result<Connection, Error> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.busy_timeout(std::time::Duration::from_millis(500))?;
    Ok(conn)
}

/// Highest `gen_metadata` index of a conversation database, or `None` when it has no rows.
///
/// # Errors
/// Returns [`Error`] if the database cannot be opened or lacks the table.
pub fn trajectory_max_generation(path: &Path) -> Result<Option<i64>, Error> {
    let conn = open_trajectory(path)?;
    conn.query_row(
        obfstr::obfstr!("SELECT MAX(idx) FROM gen_metadata"),
        [],
        |row| row.get(0),
    )
    .map_err(Error::from)
}

/// Reads the model calls after `after` (exclusive) and the step metadata that dates them.
/// The file is opened read-only and closed before returning; nothing is written.
///
/// # Errors
/// Returns [`Error`] if the database cannot be opened or lacks the expected tables.
pub fn read_trajectory(path: &Path, after: i64) -> Result<TrajectoryRows, Error> {
    let conn = open_trajectory(path)?;
    let mut rows = TrajectoryRows::default();
    let mut statement = conn.prepare(obfstr::obfstr!(
        "SELECT idx, data FROM gen_metadata WHERE idx > ?1 AND data IS NOT NULL ORDER BY idx"
    ))?;
    for row in statement.query_map([after], |row| Ok((row.get(0)?, row.get(1)?)))? {
        rows.generations.push(row?);
    }
    if !rows.generations.is_empty() {
        let mut statement = conn.prepare(obfstr::obfstr!(
            "SELECT metadata FROM steps WHERE metadata IS NOT NULL"
        ))?;
        for row in statement.query_map([], |row| row.get(0))? {
            rows.step_metadata.push(row?);
        }
    }
    Ok(rows)
}

impl Database {
    /// Whether a session row exists.
    ///
    /// # Errors
    /// Returns [`Error`] if the query fails.
    pub fn session_exists(&self, id: &str) -> Result<bool, Error> {
        self.conn
            .query_row(
                obfstr::obfstr!("SELECT 1 FROM session WHERE id=?1"),
                [id],
                |_| Ok(()),
            )
            .optional()
            .map(|row| row.is_some())
            .map_err(Error::from)
    }

    /// Commits an Antigravity conversation's model-call usage with its checkpoint, atomically.
    ///
    /// The trajectory is the single usage source of a conversation, so usage a transcript recorded
    /// earlier under another key (`<id>:<step>`) is removed in the same transaction instead of
    /// being counted twice.
    ///
    /// # Errors
    /// Returns [`Error`] if any statement fails.
    pub fn commit_trajectory_usage(
        &mut self,
        session_id: &str,
        usage: &[UsageRecord],
        replaces_prefix: &str,
        checkpoint: &SourceCheckpoint,
    ) -> Result<(), Error> {
        let tx = self.conn.transaction()?;
        {
            let mut insert = tx.prepare_cached(
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
                insert.execute(params![
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
        if !usage.is_empty() {
            tx.execute(
                obfstr::obfstr!(
                    "DELETE FROM usage_record WHERE session_id=?1 AND substr(dedup_key,1,length(?2)) <> ?2"
                ),
                params![session_id, replaces_prefix],
            )?;
        }
        tx.execute(
            r#"
            INSERT INTO source_file (file_path, last_read_offset, file_size, fingerprint, last_ingested_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(file_path) DO UPDATE SET
                last_read_offset = excluded.last_read_offset,
                file_size = excluded.file_size,
                fingerprint = excluded.fingerprint,
                last_ingested_at = excluded.last_ingested_at
            "#,
            params![
                checkpoint.file_path,
                i64::try_from(checkpoint.last_read_offset).unwrap_or(i64::MAX),
                i64::try_from(checkpoint.file_size).unwrap_or(i64::MAX),
                checkpoint.fingerprint,
                checkpoint.last_ingested_at.to_rfc3339(),
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Sets the stored surface of existing sessions; absent sessions and unchanged rows are
    /// skipped. Returns how many rows changed.
    ///
    /// # Errors
    /// Returns [`Error`] if the update fails.
    pub fn set_session_surfaces(&mut self, surfaces: &[(String, Surface)]) -> Result<usize, Error> {
        let tx = self.conn.transaction()?;
        let mut changed = 0;
        {
            let mut update = tx.prepare_cached(obfstr::obfstr!(
                "UPDATE session SET surface=?2 WHERE id=?1 AND surface<>?2"
            ))?;
            for (id, surface) in surfaces {
                changed += update.execute(params![id, surface.as_str()])?;
            }
        }
        tx.commit()?;
        Ok(changed)
    }
}
