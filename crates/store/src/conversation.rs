use super::*;

/// Saved launch metadata for a conversation's current official agent.
pub struct ConversationLaunch {
    /// Official CLI identifier.
    pub agent: String,
    /// Original workspace directory.
    pub repo: String,
    /// Requested model, which is not proof of activation.
    pub model: Option<String>,
}

impl Database {
    /// Lists saved conversation identities for disconnected history discovery.
    ///
    /// # Errors
    /// Returns an error if the local mapping cannot be read.
    pub fn conversation_ids(&self) -> Result<Vec<String>, Error> {
        let mut statement = self.conn.prepare(obfstr::obfstr!(
            "SELECT DISTINCT conversation_id FROM conversation_member"
        ))?;
        let rows = statement.query_map([], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Error::from)
    }

    /// Saves launch metadata locally for restart and model selection.
    ///
    /// # Errors
    /// Returns an error if the metadata cannot be saved.
    pub fn save_conversation_launch(
        &self,
        host: &str,
        agent: &str,
        repo: &str,
        model: Option<&str>,
    ) -> Result<(), Error> {
        self.conn.execute(obfstr::obfstr!("INSERT OR IGNORE INTO conversation_member(host_id,conversation_id,position) VALUES (?1,?1,0)"),[host])?;
        self.conn.execute(obfstr::obfstr!("UPDATE conversation_member SET agent=?2,repo=?3,requested_model=?4 WHERE host_id=?1"),params![host,agent,repo,model])?;
        Ok(())
    }

    /// Reads the current saved launch metadata, without credentials or chat.
    ///
    /// # Errors
    /// Returns an error if the metadata cannot be read.
    pub fn conversation_launch(&self, id: &str) -> Result<Option<ConversationLaunch>, Error> {
        let root = self.conversation_id(id)?;
        Ok(self.conn.query_row(obfstr::obfstr!("SELECT agent,repo,requested_model FROM conversation_member WHERE conversation_id=?1 AND agent IS NOT NULL ORDER BY position DESC LIMIT 1"),[root],|row| Ok(ConversationLaunch {agent:row.get(0)?,repo:row.get(1)?,model:row.get(2)?})).optional()?)
    }

    /// Replaces a stopped host, preserving the conversation and native history.
    ///
    /// # Errors
    /// Returns an error if routing metadata cannot be saved.
    pub fn reattach_conversation(
        &self,
        id: &str,
        new_host: &str,
        native: Option<&str>,
    ) -> Result<(), Error> {
        let root = self.conversation_id(id)?;
        if self.conversation_target(id)?.is_some() {
            self.conn.execute(obfstr::obfstr!("UPDATE conversation_member SET host_id=?2 WHERE conversation_id=?1 AND position=(SELECT MAX(position) FROM conversation_member WHERE conversation_id=?1)"),params![root,new_host])?;
        } else {
            self.conn.execute(obfstr::obfstr!("INSERT INTO conversation_member(host_id,conversation_id,native_id,position) VALUES (?1,?2,?3,0)"),params![new_host,root,native])?;
        }
        Ok(())
    }

    /// Returns the first native session, for the original title and workspace.
    ///
    /// # Errors
    /// Returns an error if conversation metadata cannot be read.
    pub fn conversation_source(&self, id: &str) -> Result<String, Error> {
        let root = self.conversation_id(id)?;
        Ok(self.conn.query_row(obfstr::obfstr!("SELECT native_id FROM conversation_member WHERE conversation_id=?1 AND native_id IS NOT NULL ORDER BY position LIMIT 1"),[root],|row| row.get(0)).optional()?.unwrap_or_else(|| id.to_owned()))
    }

    /// Returns the durable conversation identity for a native session or host.
    ///
    /// # Errors
    /// Returns an error if the local mapping cannot be read.
    pub fn conversation_id(&self, id: &str) -> Result<String, Error> {
        Ok(self.conn.query_row(obfstr::obfstr!("SELECT conversation_id FROM conversation_member WHERE host_id=?1 OR native_id=?1 OR conversation_id=?1 LIMIT 1"), [id], |row| row.get(0)).optional()?.unwrap_or_else(|| id.to_owned()))
    }

    /// Records a unique observed native-to-host binding without replacing history.
    ///
    /// # Errors
    /// Returns an error if the binding conflicts with an existing native identity.
    pub fn bind_conversation_host(&self, host: &str, native: &str) -> Result<(), Error> {
        self.conn.execute(obfstr::obfstr!("INSERT INTO conversation_member(host_id,conversation_id,native_id,position) VALUES (?1,?1,?2,0) ON CONFLICT(host_id) DO UPDATE SET native_id=excluded.native_id WHERE conversation_member.native_id IS NULL OR conversation_member.native_id=excluded.native_id"), params![host,native])?;
        Ok(())
    }

    /// Adds a new agent session to the same conversation. Existing members stay intact.
    ///
    /// # Errors
    /// Returns an error if the new member cannot be saved.
    pub fn continue_conversation(&self, source: &str, target_host: &str) -> Result<(), Error> {
        let root = self.conversation_id(source)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(obfstr::obfstr!("INSERT OR IGNORE INTO conversation_member(host_id,conversation_id,native_id,position) VALUES (?1,?1,?1,0)"), [&root])?;
        tx.execute(obfstr::obfstr!("INSERT INTO conversation_member(host_id,conversation_id,position,needs_context) SELECT ?1,?2,COALESCE(MAX(position),0)+1,1 FROM conversation_member WHERE conversation_id=?2"),params![target_host,root])?;
        tx.commit()?;
        Ok(())
    }

    /// Resolves the current host and optional observed native session.
    ///
    /// # Errors
    /// Returns an error if the mapping cannot be read.
    pub fn conversation_target(
        &self,
        id: &str,
    ) -> Result<Option<(String, Option<String>, bool)>, Error> {
        let root = self.conversation_id(id)?;
        Ok(self.conn.query_row(obfstr::obfstr!("SELECT host_id,native_id,needs_context FROM conversation_member WHERE conversation_id=?1 ORDER BY position DESC LIMIT 1"),[root],|row| Ok((row.get(0)?,row.get(1)?,row.get(2)?))).optional()?)
    }

    /// Hosts bound to a thinking, tool or approval session that are absent from `live_hosts`:
    /// the candidates [`Self::end_exited_hosted_sessions`] would end.
    ///
    /// # Errors
    /// Returns an error if the lookup fails.
    pub fn unlisted_bound_hosts(&self, live_hosts: &[String]) -> Result<Vec<String>, Error> {
        let mut statement = self.conn.prepare(obfstr::obfstr!(
            "SELECT DISTINCT m.host_id FROM conversation_member m JOIN session s ON s.id = m.native_id WHERE m.native_id <> m.host_id AND s.state IN ('thinking','tool','awaiting_approval')"
        ))?;
        let hosts = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(hosts
            .into_iter()
            .filter(|host| !live_hosts.contains(host))
            .collect())
    }

    /// Marks bound sessions ended when their Gnomcode host is not in `live_hosts`, so a killed
    /// host never leaves a thinking/tool/approval state behind. Returns the rows changed.
    ///
    /// `known_alive` holds hosts a fresh process check found running. Fail open: when any of
    /// them is missing from `live_hosts` the listing is incomplete (an unreadable record, a
    /// race with a starting host, or a wrongly empty list), so nothing is ended.
    ///
    /// # Errors
    /// Returns an error if the update fails.
    pub fn end_exited_hosted_sessions(
        &self,
        live_hosts: &[String],
        known_alive: &[String],
    ) -> Result<usize, Error> {
        if known_alive.iter().any(|host| !live_hosts.contains(host)) {
            return Ok(0);
        }
        let mut statement = self.conn.prepare(obfstr::obfstr!(
            "SELECT m.host_id, m.native_id FROM conversation_member m JOIN session s ON s.id = m.native_id WHERE m.native_id <> m.host_id AND s.state IN ('thinking','tool','awaiting_approval')"
        ))?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut ended = 0;
        for (host, native) in rows {
            if !live_hosts.contains(&host) {
                ended += self.conn.execute(
                    obfstr::obfstr!("UPDATE session SET state='ended' WHERE id=?1"),
                    [native],
                )?;
            }
        }
        Ok(ended)
    }

    /// Marks the initial context as delivered after the host acknowledges the prompt.
    ///
    /// # Errors
    /// Returns an error if the delivery state cannot be saved.
    pub fn conversation_context_delivered(&self, host: &str) -> Result<(), Error> {
        self.conn.execute(
            obfstr::obfstr!("UPDATE conversation_member SET needs_context=0 WHERE host_id=?1"),
            [host],
        )?;
        Ok(())
    }

    /// Reads one continuous page across all provider sessions, preserving event cursors.
    ///
    /// # Errors
    /// Returns an error if history cannot be read.
    pub fn conversation_chat(
        &self,
        id: &str,
        limit: Option<usize>,
        before: Option<&str>,
    ) -> Result<Vec<ChatEvent>, Error> {
        let root = self.conversation_id(id)?;
        let mut stmt = self.conn.prepare(obfstr::obfstr!(r#"
            WITH members AS (
                SELECT native_id, host_id, position FROM conversation_member WHERE conversation_id=?1
                UNION ALL SELECT ?1, ?1, 0 WHERE NOT EXISTS (SELECT 1 FROM conversation_member WHERE conversation_id=?1)
            ), events AS (
                SELECT e.*, m.position AS segment FROM chat_event e JOIN members m ON e.session_id=m.native_id OR e.session_id=m.host_id
            )
            SELECT event_id,session_id,segment*1000000000+turn_index,role,content,tool_name,status,created_at
            FROM events WHERE (?3 IS NULL OR (segment,turn_index,created_at,event_id) < (SELECT segment,turn_index,created_at,event_id FROM events WHERE event_id=?3))
            ORDER BY segment DESC,turn_index DESC,created_at DESC,event_id DESC LIMIT ?2
        "#))?;
        let rows = stmt.query_map(
            params![root, limit.unwrap_or(1000).min(5000) as i64, before],
            handoff_chat_row,
        )?;
        let mut events = rows.collect::<Result<Vec<_>, _>>()?;
        events.reverse();
        Ok(events)
    }

    /// Reads the earliest available conversation title, independently of session-list pagination.
    ///
    /// # Errors
    /// Returns an error if conversation metadata cannot be read.
    pub fn conversation_title(&self, id: &str) -> Result<Option<String>, Error> {
        let root = self.conversation_id(id)?;
        let title: Option<String> = self.conn.query_row(obfstr::obfstr!(r#"
            WITH members AS (
                SELECT native_id,host_id,position FROM conversation_member WHERE conversation_id=?1
                UNION ALL SELECT ?1,?1,0 WHERE NOT EXISTS (SELECT 1 FROM conversation_member WHERE conversation_id=?1)
            )
            SELECT s.title FROM session s JOIN members m ON s.id=m.native_id OR s.id=m.host_id
            WHERE s.title IS NOT NULL AND trim(s.title)<>''
            ORDER BY m.position,s.started_at,s.id LIMIT 1
        "#), [root], |row| row.get(0)).optional()?;
        Ok(title.map(|title| gnomcode_core::redact::redact_chat_content(&title)))
    }

    /// Reads the first retained user request across a continuous conversation, without a page cap.
    /// Deleted/expired history is never recovered or copied into separate storage.
    ///
    /// # Errors
    /// Returns an error if the conversation history cannot be read.
    pub fn conversation_first_user_event(&self, id: &str) -> Result<Option<ChatEvent>, Error> {
        let root = self.conversation_id(id)?;
        let mut stmt = self.conn.prepare(obfstr::obfstr!(r#"
            WITH members AS (
                SELECT native_id, host_id, position FROM conversation_member WHERE conversation_id=?1
                UNION ALL SELECT ?1, ?1, 0 WHERE NOT EXISTS (SELECT 1 FROM conversation_member WHERE conversation_id=?1)
            ), events AS (
                SELECT e.*, m.position AS segment FROM chat_event e JOIN members m ON e.session_id=m.native_id OR e.session_id=m.host_id
            )
            SELECT event_id,session_id,segment*1000000000+turn_index,role,content,tool_name,status,created_at
            FROM events WHERE role='user'
            ORDER BY segment ASC,turn_index ASC,created_at ASC,event_id ASC
        "#))?;
        let rows = stmt.query_map([root], handoff_chat_row)?;
        for row in rows {
            let mut event = row?;
            event.content = gnomcode_core::redact::clean_user_message(&event.content);
            if !event.content.trim().is_empty() {
                return Ok(Some(event));
            }
        }
        Ok(None)
    }

    /// Deletes all Gnomcode-owned chat in a conversation, retaining routing metadata.
    ///
    /// # Errors
    /// Returns an error if deletion fails.
    pub fn delete_conversation_chat(&self, id: &str) -> Result<usize, Error> {
        self.cancel_conversation_schedules(id)?;
        let root = self.conversation_id(id)?;
        let tx = self.conn.unchecked_transaction()?;
        mark_deleted(
            &tx,
            obfstr::obfstr!(
                "SELECT event_id FROM chat_event WHERE session_id=?1 OR session_id IN (SELECT native_id FROM conversation_member WHERE conversation_id=?1) OR session_id IN (SELECT host_id FROM conversation_member WHERE conversation_id=?1)"
            ),
            [&root],
        )?;
        let deleted = tx.execute(obfstr::obfstr!("DELETE FROM chat_event WHERE session_id=?1 OR session_id IN (SELECT native_id FROM conversation_member WHERE conversation_id=?1) OR session_id IN (SELECT host_id FROM conversation_member WHERE conversation_id=?1)"), [&root])?;
        tx.commit()?;
        Ok(deleted)
    }
}

fn handoff_chat_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChatEvent> {
    let role: String = row.get(3)?;
    let created: String = row.get(7)?;
    Ok(ChatEvent {
        event_id: row.get(0)?,
        session_id: row.get(1)?,
        turn_index: row.get(2)?,
        role: ChatRole::parse_role(&role).unwrap_or(ChatRole::System),
        content: if role == "tool" {
            String::new()
        } else {
            gnomcode_core::redact::redact_chat_content(&row.get::<_, String>(4)?)
        },
        tool_name: row.get(5)?,
        status: row.get(6)?,
        created_at: DateTime::parse_from_rfc3339(&created)
            .map(|time| time.with_timezone(&Utc))
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_message(db: &Database, session: &str, event: &str, turn: i64) {
        db.conn.execute("INSERT OR IGNORE INTO project(id,canonical_key,repo_root,name) VALUES ('p','p','fixture','p')",[]).expect("project");
        db.conn.execute("INSERT OR IGNORE INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES (?1,'codex','cli','p','idle','2026-10-30T00:00:00Z','2026-10-30T00:00:00Z')",[session]).expect("session");
        db.insert_chat_event(&ChatEvent {
            event_id: event.into(),
            session_id: session.into(),
            turn_index: turn,
            role: ChatRole::User,
            content: event.into(),
            created_at: Utc::now(),
            tool_name: None,
            status: None,
        })
        .expect("message");
    }

    #[test]
    fn original_request_survives_more_than_a_page_and_switches_but_not_deletion() {
        let db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "original-goal", 1);
        db.conn
            .execute(
                "UPDATE chat_event SET content=?1 WHERE event_id='original-goal'",
                ["Preserve leap days. STAGING_API_KEY=sk-live-evalfixture0123456789"],
            )
            .expect("content");
        for turn in 2..=5002 {
            insert_message(&db, "codex:first", &format!("later-{turn}"), turn);
        }
        db.bind_conversation_host("first-host", "codex:first")
            .expect("source");
        insert_message(&db, "claude-code:second", "latest-request", 1);
        db.continue_conversation("first-host", "second-host")
            .expect("switch");
        db.bind_conversation_host("second-host", "claude-code:second")
            .expect("target");
        db.conn
            .execute(
                "UPDATE session SET title='Original calendar task' WHERE id='codex:first'",
                [],
            )
            .expect("title");
        assert_eq!(
            db.conversation_title("second-host")
                .expect("title")
                .as_deref(),
            Some("Original calendar task")
        );
        let recent = db
            .conversation_chat("second-host", Some(50), None)
            .expect("recent");
        assert!(!recent.iter().any(|event| event.event_id == "original-goal"));
        let first = db
            .conversation_first_user_event("second-host")
            .expect("query")
            .expect("first");
        assert_eq!(first.event_id, "original-goal");
        assert!(first.content.contains("Preserve leap days"));
        assert!(!first.content.contains("sk-live-evalfixture0123456789"));
        let packet = db
            .prepare_handoff("codex:first", "target", vec![])
            .expect("packet")
            .expect("session");
        assert!(packet.prompt_summary.contains("Preserve leap days"));
        assert!(
            !packet
                .prompt_summary
                .contains("sk-live-evalfixture0123456789")
        );
        db.delete_conversation_chat("second-host").expect("delete");
        assert!(
            db.conversation_first_user_event("second-host")
                .expect("query")
                .is_none()
        );
    }

    #[test]
    fn retention_compares_rfc3339_timestamps_by_instant() {
        let db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "expired", 1);
        insert_message(&db, "codex:first", "recent", 2);
        let mut cutoff = Utc::now() - chrono::Duration::days(1);
        let mut elapsed = cutoff.time().signed_duration_since(chrono::NaiveTime::MIN);
        if elapsed < chrono::Duration::seconds(2) {
            std::thread::sleep(std::time::Duration::from_secs(2));
            cutoff = Utc::now() - chrono::Duration::days(1);
            elapsed = cutoff.time().signed_duration_since(chrono::NaiveTime::MIN);
        }
        let expired = cutoff - elapsed / 2;
        db.conn
            .execute(
                "UPDATE chat_event SET created_at=?1 WHERE event_id='expired'",
                [expired.to_rfc3339()],
            )
            .expect("timestamp");
        assert_eq!(db.purge_chat_events_older_than(1).expect("purge"), 1);
        let remaining = db.get_session_chat("codex:first", None).expect("chat");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].event_id, "recent");
        insert_message(&db, "codex:first", "expired", 1);
        assert_eq!(
            db.get_session_chat("codex:first", None)
                .expect("chat")
                .len(),
            1
        );
    }

    #[test]
    fn deleted_ids_survive_restart_without_blocking_new_events() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("state.db");
        {
            let db = Database::open(&path).expect("db");
            insert_message(&db, "codex:first", "deleted", 1);
            db.bind_conversation_host("host", "codex:first")
                .expect("bind");
            assert_eq!(db.delete_conversation_chat("host").expect("delete"), 1);
        }
        let db = Database::open(&path).expect("reopen");
        insert_message(&db, "codex:first", "deleted", 1);
        insert_message(&db, "codex:first", "new", 2);
        let events = db.get_session_chat("codex:first", None).expect("chat");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "new");
    }

    #[test]
    fn deletion_marker_does_not_store_the_raw_identifier() {
        let db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "sensitive-identifier", 1);
        db.delete_session_chat("codex:first").expect("delete");
        let marker: Vec<u8> = db
            .conn
            .query_row(
                "SELECT CAST(event_id AS BLOB) FROM deleted_chat_event",
                [],
                |row| row.get(0),
            )
            .expect("marker");
        assert_ne!(marker, b"sensitive-identifier");
    }

    #[test]
    fn failed_delete_rolls_back_deletion_markers() {
        let db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "preserved", 1);
        db.conn.execute_batch("CREATE TRIGGER reject_chat_delete BEFORE DELETE ON chat_event BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").expect("trigger");
        assert!(db.delete_session_chat("codex:first").is_err());
        assert_eq!(
            db.get_session_chat("codex:first", None)
                .expect("chat")
                .len(),
            1
        );
        let markers: i64 = db
            .conn
            .query_row("SELECT count(*) FROM deleted_chat_event", [], |row| {
                row.get(0)
            })
            .expect("markers");
        assert_eq!(markers, 0);
    }

    #[test]
    fn v9_upgrade_preserves_chat_and_creates_deletion_markers() {
        let mut db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "preserved", 1);
        db.conn
            .execute_batch(
                "DROP TABLE deleted_chat_event; DELETE FROM schema_migrations WHERE version=10;",
            )
            .expect("v9 state");
        db.migrate().expect("upgrade");
        db.migrate().expect("repeat");
        assert_eq!(
            db.get_session_chat("codex:first", None)
                .expect("chat")
                .len(),
            1
        );
        db.delete_session_chat("codex:first").expect("delete");
        insert_message(&db, "codex:first", "preserved", 1);
        assert!(
            db.get_session_chat("codex:first", None)
                .expect("chat")
                .is_empty()
        );
    }

    #[test]
    fn switches_page_across_sessions_and_delete_all_owned_history() {
        let db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "first", 1);
        insert_message(&db, "claude-code:second", "second", 1);
        insert_message(&db, "codex:third", "third", 1);
        db.bind_conversation_host("first-host", "codex:first")
            .expect("source");
        db.continue_conversation("first-host", "second-host")
            .expect("switch");
        db.bind_conversation_host("second-host", "claude-code:second")
            .expect("bind second");
        db.continue_conversation("claude-code:second", "third-host")
            .expect("second switch");
        db.bind_conversation_host("third-host", "codex:third")
            .expect("bind third");
        let recent = db
            .conversation_chat("first-host", Some(2), None)
            .expect("recent");
        assert_eq!(
            recent
                .iter()
                .map(|event| event.event_id.as_str())
                .collect::<Vec<_>>(),
            ["second", "third"]
        );
        assert!(recent[0].turn_index < recent[1].turn_index);
        let earlier = db
            .conversation_chat("codex:third", Some(2), Some("second"))
            .expect("page across switch");
        assert_eq!(earlier[0].event_id, "first");
        assert_eq!(
            db.conversation_id("claude-code:second").expect("root"),
            "first-host"
        );
        assert_eq!(
            db.delete_conversation_chat("second-host").expect("delete"),
            3
        );
        assert!(
            db.conversation_chat("first-host", None, None)
                .expect("empty")
                .is_empty()
        );
    }

    #[test]
    fn routing_and_pending_context_survive_restart_and_host_replacement() {
        let dir = tempfile::tempdir().expect("temp");
        let path = dir.path().join("state.db");
        {
            let db = Database::open(&path).expect("db");
            insert_message(&db, "codex:first", "saved-message", 1);
            db.bind_conversation_host("original", "codex:first")
                .expect("source");
            db.continue_conversation("original", "target")
                .expect("switch");
            db.save_conversation_launch(
                "target",
                "claude-code",
                "workspace",
                Some("claude-sonnet-4"),
            )
            .expect("metadata");
        }
        let db = Database::open(&path).expect("reopen");
        assert_eq!(
            db.conversation_target("original").expect("target"),
            Some(("target".into(), None, true))
        );
        assert_eq!(
            db.conversation_launch("original")
                .expect("launch")
                .expect("metadata")
                .agent,
            "claude-code"
        );
        db.reattach_conversation("original", "restarted", None)
            .expect("reattach");
        db.bind_conversation_host("restarted", "claude-code:saved")
            .expect("native binding");
        db.conversation_context_delivered("restarted")
            .expect("delivery");
        assert_eq!(
            db.conversation_target("original").expect("target"),
            Some(("restarted".into(), Some("claude-code:saved".into()), false))
        );
        assert_eq!(
            db.conversation_chat("original", None, None)
                .expect("history")[0]
                .content,
            "saved-message"
        );
    }

    #[test]
    fn upgrade_preserves_v8_chat_and_is_idempotent() {
        let mut db = Database::open_in_memory().expect("db");
        insert_message(&db, "codex:first", "preserved", 1);
        db.conn
            .execute_batch(
                "DROP TABLE conversation_member; DROP TABLE deleted_chat_event; DELETE FROM schema_migrations WHERE version IN (9, 10);",
            )
            .expect("v8 state");
        db.migrate().expect("upgrade");
        db.migrate().expect("repeat");
        assert_eq!(
            db.conversation_chat("codex:first", None, None)
                .expect("chat")[0]
                .content,
            "preserved"
        );
    }

    fn two_bound_sessions() -> Database {
        let db = Database::open_in_memory().expect("db");
        for (host, native) in [("h-dead", "antigravity:a"), ("h-live", "antigravity:b")] {
            db.conn
                .execute(
                    "INSERT OR IGNORE INTO project(id,canonical_key,repo_root,name) VALUES ('p','/w','/w','w')",
                    [],
                )
                .expect("project");
            db.conn
                .execute(
                    "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES (?1,'antigravity','cli','p','tool','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                    [native],
                )
                .expect("session");
            db.bind_conversation_host(host, native).expect("bind");
        }
        db
    }

    fn state_of(db: &Database, id: &str) -> String {
        db.conn
            .query_row("SELECT state FROM session WHERE id=?1", [id], |r| r.get(0))
            .expect("state")
    }

    #[test]
    fn sweep_ends_nothing_when_a_host_known_alive_is_missing_from_the_listing() {
        let db = two_bound_sessions();
        // Wrongly empty listing while a process check found h-dead running.
        assert_eq!(
            db.end_exited_hosted_sessions(&[], &["h-dead".into()])
                .expect("sweep"),
            0
        );
        // Partial listing that dropped a known-alive host.
        assert_eq!(
            db.end_exited_hosted_sessions(&["h-live".into()], &["h-dead".into()])
                .expect("sweep"),
            0
        );
        assert_eq!(state_of(&db, "antigravity:a"), "tool");
        assert_eq!(state_of(&db, "antigravity:b"), "tool");
        // A complete listing that agrees with the process check still ends the dead host.
        assert_eq!(
            db.end_exited_hosted_sessions(&["h-live".into()], &["h-live".into()])
                .expect("sweep"),
            1
        );
        assert_eq!(state_of(&db, "antigravity:a"), "ended");
        assert_eq!(state_of(&db, "antigravity:b"), "tool");
    }

    #[test]
    fn unlisted_bound_hosts_are_the_sweep_candidates() {
        let db = two_bound_sessions();
        let mut candidates = db.unlisted_bound_hosts(&[]).expect("candidates");
        candidates.sort();
        assert_eq!(candidates, ["h-dead", "h-live"]);
        assert_eq!(
            db.unlisted_bound_hosts(&["h-live".into()])
                .expect("candidates"),
            ["h-dead"]
        );
    }

    #[test]
    fn exited_host_ends_its_bound_session_but_not_a_live_one() {
        let db = Database::open_in_memory().expect("db");
        for (host, native) in [("h-dead", "antigravity:a"), ("h-live", "antigravity:b")] {
            db.conn
                .execute(
                    "INSERT OR IGNORE INTO project(id,canonical_key,repo_root,name) VALUES ('p','/w','/w','w')",
                    [],
                )
                .expect("project");
            db.conn
                .execute(
                    "INSERT INTO session(id,agent,surface,project_id,state,started_at,last_event_at) VALUES (?1,'antigravity','cli','p','tool','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z')",
                    [native],
                )
                .expect("session");
            db.bind_conversation_host(host, native).expect("bind");
        }
        assert_eq!(
            db.end_exited_hosted_sessions(&["h-live".into()], &[])
                .expect("sweep"),
            1
        );
        let state = |id: &str| -> String {
            db.conn
                .query_row("SELECT state FROM session WHERE id=?1", [id], |r| r.get(0))
                .expect("state")
        };
        assert_eq!(state("antigravity:a"), "ended");
        assert_eq!(state("antigravity:b"), "tool");
    }
}
