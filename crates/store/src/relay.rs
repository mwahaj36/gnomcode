//! Context relay configuration and ledger.

use super::*;

/// User-configured settings for the Pro Labs Context Relay Engine.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextRelayConfig {
    /// Relay execution policy ("ask_first", "auto", "disabled").
    pub policy: String,
    /// Relay routing strategy ("same_agent", "failover").
    pub strategy: String,
    /// Context window saturation percentage threshold to trigger relay (e.g. 85, 90, 95).
    pub threshold_pct: u32,
    /// Timestamp of last modification.
    pub updated_at: String,
}

impl Default for ContextRelayConfig {
    fn default() -> Self {
        Self {
            policy: "ask_first".to_owned(),
            strategy: "same_agent".to_owned(),
            threshold_pct: 90,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

/// Ledger item for a recorded or pending Context Relay event.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContextRelayEntry {
    /// Unique relay event ID.
    pub id: String,
    /// Saturated session ID.
    pub session_id: String,
    /// Source agent slug.
    pub source_agent: String,
    /// Target destination agent slug.
    pub target_agent: String,
    /// Repository project name.
    pub project_name: String,
    /// Working directory path where checkpoint and new session reside.
    pub cwd: String,
    /// Observed context fill percentage at trigger time.
    pub fill_pct: u32,
    /// Current status ("pending", "executed", "dismissed").
    pub status: String,
    /// Timestamp when relay was proposed/recorded.
    pub created_at: String,
    /// Timestamp when relay handoff was executed.
    pub executed_at: Option<String>,
}

impl Database {
    /// Returns current Context Relay Engine configuration.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn get_context_relay_config(&self) -> Result<ContextRelayConfig, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT policy, strategy, threshold_pct, updated_at FROM context_relay_config WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(ContextRelayConfig {
                policy: row.get(0)?,
                strategy: row.get(1)?,
                threshold_pct: row.get(2)?,
                updated_at: row.get(3)?,
            })
        } else {
            let def = ContextRelayConfig::default();
            let _ = self.conn.execute(
                "INSERT OR IGNORE INTO context_relay_config (id, policy, strategy, threshold_pct, updated_at) VALUES (1, ?1, ?2, ?3, ?4)",
                params![def.policy, def.strategy, def.threshold_pct, def.updated_at],
            );
            Ok(def)
        }
    }

    /// Saves updated Context Relay Engine configuration.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn save_context_relay_config(&self, cfg: &ContextRelayConfig) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO context_relay_config (id, policy, strategy, threshold_pct, updated_at)
            VALUES (1, ?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET
                policy = excluded.policy,
                strategy = excluded.strategy,
                threshold_pct = excluded.threshold_pct,
                updated_at = excluded.updated_at
            "#,
            params![
                cfg.policy,
                cfg.strategy,
                cfg.threshold_pct,
                Utc::now().to_rfc3339()
            ],
        )?;
        Ok(())
    }

    /// Lists recent recorded Context Relay events.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn list_context_relays(&self) -> Result<Vec<ContextRelayEntry>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, source_agent, target_agent, project_name, cwd, fill_pct, status, created_at, executed_at FROM context_relay ORDER BY created_at DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ContextRelayEntry {
                id: row.get(0)?,
                session_id: row.get(1)?,
                source_agent: row.get(2)?,
                target_agent: row.get(3)?,
                project_name: row.get(4)?,
                cwd: row.get(5)?,
                fill_pct: row.get(6)?,
                status: row.get(7)?,
                created_at: row.get(8)?,
                executed_at: row.get(9)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Records or upserts a Context Relay event.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn record_context_relay(&self, entry: &ContextRelayEntry) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO context_relay (id, session_id, source_agent, target_agent, project_name, cwd, fill_pct, status, created_at, executed_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                executed_at = excluded.executed_at
            "#,
            params![
                entry.id,
                entry.session_id,
                entry.source_agent,
                entry.target_agent,
                entry.project_name,
                entry.cwd,
                entry.fill_pct,
                entry.status,
                entry.created_at,
                entry.executed_at,
            ],
        )?;
        Ok(())
    }

    /// Updates status of a Context Relay event.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn update_context_relay_status(&self, id: &str, status: &str) -> Result<(), Error> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE context_relay SET status = ?1, executed_at = ?2 WHERE id = ?3",
            params![status, now, id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ContextRelayEntry, Database};
    use chrono::Utc;

    #[test]
    fn context_relay_config_and_ledger_persist() {
        use gnomcode_core::RoutingPolicy;
        let db = Database::open_in_memory().expect("in-memory db");
        let def = db.get_context_relay_config().expect("default config");
        assert_eq!(def.policy, "ask_first");
        assert_eq!(def.threshold_pct, 90);

        let mut updated = def.clone();
        updated.policy = "auto".to_owned();
        updated.threshold_pct = 85;
        db.save_context_relay_config(&updated).expect("save config");
        let loaded = db.get_context_relay_config().expect("loaded config");
        assert_eq!(loaded.policy, "auto");
        assert_eq!(loaded.threshold_pct, 85);

        // Test RoutingPolicy wrapper
        assert_eq!(
            db.get_routing_policy().expect("get policy"),
            RoutingPolicy::Automatic
        );
        db.save_routing_policy(RoutingPolicy::DenyCrossProvider)
            .expect("save routing policy");
        assert_eq!(
            db.get_routing_policy().expect("get policy 2"),
            RoutingPolicy::DenyCrossProvider
        );

        assert!(db.get_auto_update_enabled().expect("default auto update"));
        db.set_auto_update_enabled(false)
            .expect("disable auto update");
        assert!(!db.get_auto_update_enabled().expect("disabled auto update"));

        let entry = ContextRelayEntry {
            id: "relay-1".to_owned(),
            session_id: "claude-code:s1".to_owned(),
            source_agent: "claude-code".to_owned(),
            target_agent: "claude-code".to_owned(),
            project_name: "gnomcode".to_owned(),
            cwd: "/workspace/gnomcode".to_owned(),
            fill_pct: 92,
            status: "pending".to_owned(),
            created_at: Utc::now().to_rfc3339(),
            executed_at: None,
        };
        db.record_context_relay(&entry).expect("record relay");
        let list = db.list_context_relays().expect("list relays");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "relay-1");
        assert_eq!(list[0].status, "pending");

        db.update_context_relay_status("relay-1", "executed")
            .expect("update status");
        let list2 = db.list_context_relays().expect("list relays 2");
        assert_eq!(list2[0].status, "executed");
        assert!(list2[0].executed_at.is_some());
    }
}
