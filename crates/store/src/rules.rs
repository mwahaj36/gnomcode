//! Guardrail rules, alerts and the resume queue.

use super::*;

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

impl Database {
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
        self.conn.execute("DELETE FROM resume_queue WHERE session_id=?1 OR session_id=?2 OR session_id IN (SELECT host_id FROM conversation_member WHERE conversation_id=?2 UNION SELECT native_id FROM conversation_member WHERE conversation_id=?2)",params![session_id,root])?;
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
}

#[cfg(test)]
mod tests {
    use super::Database;
    use chrono::Utc;
    use gnomcode_core::AgentId;
    use gnomcode_core::rules::{GuardrailRule, RuleAction};

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
}
