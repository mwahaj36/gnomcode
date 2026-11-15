use super::*;

/// A laptop-owned message; text is erased after a terminal outcome.
#[derive(serde::Serialize)]
pub struct ScheduledMessage {
    /// Client-generated idempotency key.
    pub id: String,
    /// Stable conversation identity.
    pub conversation_id: String,
    /// Redacted local message, never included in cloud snapshots.
    pub text: String,
    /// UTC Unix deadline in seconds.
    pub due_at: i64,
    /// Pending, delivering, delivered, failed, cancelled, or uncertain.
    pub status: String,
}

/// Generic durable lifecycle feedback, without conversation content.
#[derive(serde::Serialize)]
pub struct AutomationNotice {
    /// Deduplication key.
    pub id: String,
    /// Conversation to open.
    pub conversation_id: String,
    /// Generic lifecycle description.
    pub message: String,
    /// UTC Unix timestamp in seconds.
    pub created_at: i64,
}

/// Pending push notice metadata queued for opt-in background phone delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingPushNotice {
    /// Deduplication event ID.
    pub id: String,
    /// Redacted notification metadata (`event`, `agent`, `session_title`), if recorded.
    pub payload: Option<gnomcode_core::notifications::RedactedNotificationPayload>,
}

impl Database {
    /// Persists a redacted message once, returning false for an existing key.
    pub fn schedule_message(
        &self,
        id: &str,
        conversation: &str,
        text: &str,
        due: i64,
    ) -> Result<bool, Error> {
        let root = self.conversation_id(conversation)?;
        let redacted = gnomcode_core::redact::redact_chat_content(text);
        let existing: Option<(String, String, i64, String)> = self
            .conn
            .query_row(
                "SELECT conversation_id,text,due_at,status FROM scheduled_message WHERE id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        if let Some((owner, previous, deadline, status)) = existing {
            if owner != root || deadline != due || (status == "pending" && previous != redacted) {
                return Err(Error::Migration(
                    "Schedule key belongs to a different message".into(),
                ));
            }
            return Ok(false);
        }
        Ok(self.conn.execute("INSERT INTO scheduled_message(id,conversation_id,text,due_at,status,created_at) VALUES (?1,?2,?3,?4,'pending',?5)", params![id,root,redacted,due,Utc::now().timestamp()])? == 1)
    }

    /// Reads local schedules, bounded to the most recent 200 entries.
    pub fn scheduled_messages(&self, conversation: &str) -> Result<Vec<ScheduledMessage>, Error> {
        let root = self.conversation_id(conversation)?;
        let mut query = self.conn.prepare("SELECT id,conversation_id,text,due_at,status FROM scheduled_message WHERE conversation_id=?1 ORDER BY due_at DESC LIMIT 200")?;
        Ok(query
            .query_map([root], |r| {
                Ok(ScheduledMessage {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    text: r.get(2)?,
                    due_at: r.get(3)?,
                    status: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Returns due pending messages in deadline order.
    pub fn due_messages(&self, now: i64) -> Result<Vec<ScheduledMessage>, Error> {
        let mut query = self.conn.prepare("SELECT id,conversation_id,text,due_at,status FROM scheduled_message WHERE status='pending' AND due_at<=?1 ORDER BY due_at,id LIMIT 100")?;
        Ok(query
            .query_map([now], |r| {
                Ok(ScheduledMessage {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    text: r.get(2)?,
                    due_at: r.get(3)?,
                    status: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Atomically claims delivery; a second worker cannot deliver again.
    pub fn claim_message(&self, id: &str) -> Result<bool, Error> {
        Ok(self.conn.execute(
            "UPDATE scheduled_message SET status='delivering',attempted_at=?2 WHERE id=?1 AND status='pending'",
            params![id,Utc::now().timestamp_millis()],
        )? == 1)
    }

    /// Erases message text and records delivery, failure, or uncertainty.
    pub fn finish_message(&self, id: &str, status: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE scheduled_message SET status=?2,text='',finished_at=unixepoch() WHERE id=?1 AND status='delivering'",
            params![id, status],
        )?;
        Ok(())
    }

    /// Returns a claimed message to the queue after the agent refused to take it, so the
    /// next pass tries again. Only for refusals that happen before any text is sent (the
    /// agent is busy or not at its prompt); a claim whose delivery may have started is
    /// finished as uncertain instead and never replayed.
    ///
    /// # Errors
    /// Returns [`Error`] if the row cannot be updated.
    pub fn release_message(&self, id: &str) -> Result<(), Error> {
        self.conn.execute(
            "UPDATE scheduled_message SET status='pending' WHERE id=?1 AND status='delivering'",
            [id],
        )?;
        Ok(())
    }

    /// Cancels a pending message only within its owning conversation.
    pub fn cancel_message(&self, conversation: &str, id: &str) -> Result<bool, Error> {
        let root = self.conversation_id(conversation)?;
        Ok(self.conn.execute("UPDATE scheduled_message SET status='cancelled',text='',finished_at=unixepoch() WHERE id=?1 AND conversation_id=?2 AND status='pending'",params![id,root])? == 1)
    }

    /// Updates the text of a pending queued message within its owning conversation.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn update_scheduled_message(
        &self,
        conversation: &str,
        id: &str,
        text: &str,
    ) -> Result<bool, Error> {
        let root = self.conversation_id(conversation)?;
        let redacted = gnomcode_core::redact::redact_chat_content(text);
        Ok(self.conn.execute(
            "UPDATE scheduled_message SET text=?3 WHERE id=?1 AND conversation_id=?2 AND status='pending'",
            params![id, root, redacted],
        )? == 1)
    }

    /// Looks up a single scheduled/queued message by id within its owning conversation.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn get_scheduled_message(
        &self,
        conversation: &str,
        id: &str,
    ) -> Result<Option<ScheduledMessage>, Error> {
        let root = self.conversation_id(conversation)?;
        self.conn
            .query_row(
                "SELECT id,conversation_id,text,due_at,status FROM scheduled_message WHERE id=?1 AND conversation_id=?2",
                params![id, root],
                |r| {
                    Ok(ScheduledMessage {
                        id: r.get(0)?,
                        conversation_id: r.get(1)?,
                        text: r.get(2)?,
                        due_at: r.get(3)?,
                        status: r.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    /// Cancels and erases pending messages when chat history is deleted.
    pub fn cancel_conversation_schedules(&self, conversation: &str) -> Result<(), Error> {
        let root = self.conversation_id(conversation)?;
        self.conn.execute("UPDATE scheduled_message SET status='cancelled',text='',finished_at=unixepoch() WHERE conversation_id=?1 AND status='pending'",[root])?;
        Ok(())
    }

    /// Recovers interrupted delivery without replay and expires terminal metadata.
    pub fn recover_scheduled_messages(&self) -> Result<(), Error> {
        self.conn.execute("INSERT OR IGNORE INTO automation_notice(id,conversation_id,message,created_at) SELECT 'interrupted:'||id,conversation_id,'Scheduled delivery was interrupted. Check the terminal before retrying; it will not be replayed.',unixepoch() FROM scheduled_message WHERE status='delivering'",[])?;
        self.conn.execute(
            "UPDATE scheduled_message SET status='uncertain',text='',finished_at=unixepoch() WHERE status='delivering'",
            [],
        )?;
        self.conn.execute("DELETE FROM scheduled_message WHERE status NOT IN ('pending','delivering') AND COALESCE(finished_at,created_at)<?1",[Utc::now().timestamp()-30*86400])?;
        Ok(())
    }

    /// Records generic lifecycle feedback once; returns whether it was new.
    pub fn automation_notice(
        &self,
        id: &str,
        conversation: &str,
        message: &str,
    ) -> Result<bool, Error> {
        Ok(self.conn.execute("INSERT OR IGNORE INTO automation_notice(id,conversation_id,message,created_at) VALUES (?1,?2,?3,?4)",params![id,self.conversation_id(conversation)?,message,Utc::now().timestamp()])? == 1)
    }

    /// Records a redacted notification once alongside structured push metadata.
    ///
    /// # Errors
    /// Returns [`Error`] if serialization or SQLite insert fails.
    pub fn automation_notice_with_push(
        &self,
        id: &str,
        conversation: &str,
        message: &str,
        payload: &gnomcode_core::notifications::RedactedNotificationPayload,
    ) -> Result<bool, Error> {
        let encoded = serde_json::to_string(payload)?;
        Ok(self.conn.execute("INSERT OR IGNORE INTO automation_notice(id,conversation_id,message,created_at,push_payload) VALUES (?1,?2,?3,?4,?5)",params![id,self.conversation_id(conversation)?,message,Utc::now().timestamp(),encoded])? == 1)
    }

    /// Returns recent generic lifecycle notifications for desktop and Remote.
    pub fn automation_notices(&self) -> Result<Vec<AutomationNotice>, Error> {
        let mut query = self.conn.prepare("SELECT id,conversation_id,message,created_at FROM automation_notice WHERE id NOT LIKE 'reset-ready:%' ORDER BY created_at DESC,rowid DESC LIMIT 100")?;
        Ok(query
            .query_map([], |r| {
                Ok(AutomationNotice {
                    id: r.get(0)?,
                    conversation_id: r.get(1)?,
                    message: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

impl Database {
    /// Returns persisted notification preferences or [`NotificationSettings::default`].
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_notification_settings(
        &self,
    ) -> Result<gnomcode_core::notifications::NotificationSettings, Error> {
        let mut query = self.conn.prepare("SELECT approval_waiting, turn_finished, turn_finished_min_minutes, limit_low, limit_reset FROM notification_settings WHERE id = 1 LIMIT 1")?;
        let row = query
            .query_row([], |row| {
                Ok(gnomcode_core::notifications::NotificationSettings {
                    approval_waiting: row.get::<_, i64>(0)? != 0,
                    turn_finished: row.get::<_, i64>(1)? != 0,
                    turn_finished_min_minutes: row
                        .get::<_, i64>(2)?
                        .clamp(0, 1440)
                        .try_into()
                        .unwrap_or(2),
                    limit_low: row.get::<_, i64>(3)? != 0,
                    limit_reset: row.get::<_, i64>(4)? != 0,
                })
            })
            .optional()?;
        Ok(row.unwrap_or_default())
    }

    /// Saves notification preferences atomically.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite update failure.
    pub fn save_notification_settings(
        &self,
        settings: &gnomcode_core::notifications::NotificationSettings,
    ) -> Result<(), Error> {
        let s = settings.clone().sanitized();
        self.conn.execute(
            r#"INSERT INTO notification_settings(id, approval_waiting, turn_finished, turn_finished_min_minutes, limit_low, limit_reset, updated_at)
                   VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
                   ON CONFLICT(id) DO UPDATE SET
                       approval_waiting = excluded.approval_waiting,
                       turn_finished = excluded.turn_finished,
                       turn_finished_min_minutes = excluded.turn_finished_min_minutes,
                       limit_low = excluded.limit_low,
                       limit_reset = excluded.limit_reset,
                       updated_at = excluded.updated_at"#,
            params![
                i64::from(s.approval_waiting),
                i64::from(s.turn_finished),
                i64::from(s.turn_finished_min_minutes),
                i64::from(s.limit_low),
                i64::from(s.limit_reset),
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Lists recent generic notices awaiting opt-in background delivery.
    ///
    /// # Errors
    /// Returns an error if pending notices cannot be read.
    pub fn pending_push_notices(&self) -> Result<Vec<String>, Error> {
        Ok(self
            .pending_push_notices_with_meta()?
            .into_iter()
            .map(|n| n.id)
            .collect())
    }

    /// Lists recent notices with redacted structured metadata awaiting Web Push delivery.
    ///
    /// # Errors
    /// Returns an error if pending notices cannot be read.
    pub fn pending_push_notices_with_meta(&self) -> Result<Vec<PendingPushNotice>, Error> {
        let mut query = self.conn.prepare("SELECT id, push_payload FROM automation_notice WHERE pushed=0 AND id NOT LIKE 'reset-ready:%' AND created_at>=?1 ORDER BY rowid LIMIT 5")?;
        Ok(query
            .query_map([Utc::now().timestamp() - 120], |row| {
                let id: String = row.get(0)?;
                let raw_payload: Option<String> = row.get(1)?;
                let payload = raw_payload.and_then(|json| {
                    serde_json::from_str::<
                            gnomcode_core::notifications::RedactedNotificationPayload,
                        >(&json)
                        .ok()
                });
                Ok(PendingPushNotice { id, payload })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Marks a background notice accepted after the authenticated server acknowledges it.
    ///
    /// # Errors
    /// Returns an error if delivery state cannot be saved.
    pub fn mark_notice_pushed(&self, id: &str) -> Result<(), Error> {
        self.conn
            .execute("UPDATE automation_notice SET pushed=1 WHERE id=?1", [id])?;
        Ok(())
    }

    /// Claims a bounded batch of recent OS notifications exactly once.
    pub fn claim_automation_notices(&self) -> Result<Vec<String>, Error> {
        let mut query=self.conn.prepare("UPDATE automation_notice SET notified=1 WHERE id IN (SELECT id FROM automation_notice WHERE notified=0 AND id NOT LIKE 'reset-ready:%' AND created_at>=?1 ORDER BY rowid LIMIT 5) RETURNING message")?;
        Ok(query
            .query_map([Utc::now().timestamp() - 120], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Last claimed scheduled delivery, used to wait for fresh native idle evidence.
    pub fn last_scheduled_attempt(&self, conversation: &str) -> Result<Option<i64>, Error> {
        let root = self.conversation_id(conversation)?;
        Ok(self.conn.query_row(
            "SELECT MAX(attempted_at) FROM scheduled_message WHERE conversation_id=?1",
            [root],
            |r| r.get(0),
        )?)
    }

    /// Looks up a lifecycle key without a bounded UI list affecting deduplication.
    pub fn has_automation_notice(&self, id: &str) -> Result<bool, Error> {
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM automation_notice WHERE id=?1)",
            [id],
            |r| r.get(0),
        )?)
    }

    /// Expires terminal records and generic lifecycle feedback after 30 days.
    pub fn expire_automation_records(&self) -> Result<(), Error> {
        let cutoff = Utc::now().timestamp() - 30 * 86400;
        self.conn.execute("DELETE FROM scheduled_message WHERE status NOT IN ('pending','delivering') AND COALESCE(finished_at,created_at)<?1",[cutoff])?;
        self.conn.execute(
            "DELETE FROM automation_notice WHERE created_at<?1",
            [cutoff],
        )?;
        Ok(())
    }

    /// Whether a confirmed reset permits retry after this exact observed error.
    pub fn quota_reset_ready(&self, native: &str, event_at: DateTime<Utc>) -> Result<bool, Error> {
        let key = format!("reset-ready:{native}:{}", event_at.to_rfc3339());
        Ok(self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM automation_notice WHERE id=?1)",
            [key],
            |r| r.get(0),
        )?)
    }

    /// Recent lifecycle events, excluding text and tool output.
    pub fn automation_events(&self) -> Result<Vec<(i64, String, String, String)>, Error> {
        let mut query=self.conn.prepare("SELECT id,session_id,kind,timestamp FROM activity_event WHERE kind IN ('rate_limited','approval_wait','error') AND julianday(timestamp)>=julianday('now','-30 minutes') ORDER BY id DESC LIMIT 100")?;
        Ok(query
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
            .collect::<Result<Vec<_>, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn background_delivery_is_independent_of_desktop_claim_and_can_retry() {
        let db = Database::open_in_memory().expect("db");
        db.automation_notice("completed:test", "", "Ready")
            .expect("notice");
        assert_eq!(db.claim_automation_notices().expect("desktop").len(), 1);
        assert_eq!(db.pending_push_notices().expect("push").len(), 1);
        assert_eq!(db.pending_push_notices().expect("retry").len(), 1);
        db.mark_notice_pushed("completed:test").expect("accepted");
        assert!(db.pending_push_notices().expect("once").is_empty());
    }
    #[test]
    fn notification_settings_persist_and_push_metadata_roundtrips() {
        let db = Database::open_in_memory().expect("db");
        let defaults = db.get_notification_settings().expect("defaults");
        assert!(defaults.approval_waiting);
        assert_eq!(defaults.turn_finished_min_minutes, 2);

        let updated = gnomcode_core::notifications::NotificationSettings {
            approval_waiting: false,
            turn_finished: true,
            turn_finished_min_minutes: 5,
            limit_low: false,
            limit_reset: true,
        };
        db.save_notification_settings(&updated).expect("save");
        assert_eq!(db.get_notification_settings().expect("reload"), updated);

        let payload = gnomcode_core::notifications::RedactedNotificationPayload {
            event: gnomcode_core::notifications::NotificationEventKind::TurnFinished,
            agent: "Codex".into(),
            session_title: "Build suite".into(),
        };
        assert!(
            db.automation_notice_with_push(
                "completed:codex:s1",
                "",
                "[Codex] Build suite: Turn finished",
                &payload,
            )
            .expect("insert")
        );
        let pending = db.pending_push_notices_with_meta().expect("pending");
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].payload, Some(payload));
    }
    #[test]
    fn native_notices_are_claimed_once_and_internal_reset_markers_stay_hidden() {
        let db = Database::open_in_memory().expect("db");
        assert!(
            db.automation_notice("saved", "conversation", "Saved on laptop.")
                .expect("notice")
        );
        assert!(
            !db.automation_notice("saved", "conversation", "Duplicate.")
                .expect("duplicate")
        );
        db.automation_notice("reset-ready:fixture", "conversation", "Internal marker.")
            .expect("marker");
        assert_eq!(
            db.claim_automation_notices().expect("claim"),
            vec!["Saved on laptop."]
        );
        assert!(
            db.claim_automation_notices()
                .expect("claimed once")
                .is_empty()
        );
        assert_eq!(db.automation_notices().expect("visible").len(), 1);
    }
    #[test]
    fn cancelled_future_schedule_expires_after_its_outcome_not_its_deadline() {
        let db = Database::open_in_memory().expect("db");
        db.schedule_message(
            "future",
            "conversation",
            "hello",
            Utc::now().timestamp() + 90 * 86400,
        )
        .expect("save");
        db.cancel_message("conversation", "future").expect("cancel");
        db.expire_automation_records().expect("recent outcome");
        assert_eq!(
            db.scheduled_messages("conversation").expect("recent").len(),
            1
        );
        db.conn
            .execute(
                "UPDATE scheduled_message SET finished_at=?1",
                [Utc::now().timestamp() - 31 * 86400],
            )
            .expect("advance retention age");
        db.expire_automation_records().expect("expire");
        assert!(
            db.scheduled_messages("conversation")
                .expect("expired")
                .is_empty()
        );
    }
    #[test]
    fn reused_schedule_key_cannot_acknowledge_a_different_message() {
        let db = Database::open_in_memory().expect("db");
        db.schedule_message("one", "first", "original", 1)
            .expect("save");
        assert!(
            db.schedule_message("one", "second", "different", 2)
                .is_err()
        );
        assert_eq!(
            db.scheduled_messages("first").expect("list")[0].text,
            "original"
        );
    }
    #[test]
    fn schedules_survive_restart_and_interrupted_delivery_is_not_replayed() {
        let dir = tempfile::tempdir().expect("dir");
        let path = dir.path().join("state.db");
        let now = Utc::now().timestamp();
        {
            let db = Database::open(&path).expect("db");
            assert!(
                db.schedule_message("one", "conversation", "hello", now)
                    .expect("save")
            );
            assert!(
                !db.schedule_message("one", "conversation", "hello", now)
                    .expect("retry")
            );
        }
        let db = Database::open(&path).expect("reopen");
        assert_eq!(db.due_messages(now - 1).expect("future").len(), 0);
        let due = db.due_messages(now).expect("due");
        assert_eq!(due[0].text, "hello");
        assert!(db.claim_message("one").expect("claim"));
        db.recover_scheduled_messages().expect("recover");
        assert!(db.due_messages(now + 1).expect("no replay").is_empty());
        let messages = db.scheduled_messages("conversation").expect("list");
        assert_eq!(messages[0].status, "uncertain");
        assert!(messages[0].text.is_empty());
    }
    #[test]
    fn cancellation_is_scoped_and_chat_deletion_erases_scheduled_text() {
        let db = Database::open_in_memory().expect("db");
        db.schedule_message("one", "conversation", "hello", 1)
            .expect("save");
        assert!(!db.cancel_message("other", "one").expect("wrong owner"));
        db.delete_conversation_chat("conversation").expect("delete");
        let messages = db.scheduled_messages("conversation").expect("list");
        assert_eq!(messages[0].status, "cancelled");
        assert!(messages[0].text.is_empty());
    }
}
