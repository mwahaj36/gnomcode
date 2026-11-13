//! Chat event storage, repair, retrieval and deletion.

use super::*;

impl Database {
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
}

#[cfg(test)]
mod tests {
    use super::{Database, SourceCheckpoint};
    use chrono::Utc;
    use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};

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
}
