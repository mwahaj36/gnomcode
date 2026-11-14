//! User profile, routing policy, update and feature settings.

use super::*;

/// User profile with per-provider active subscription plans.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    /// Unique user identifier.
    pub id: String,
    /// User handle.
    pub username: String,
    /// Display name shown in UI.
    pub display_name: String,
    /// Monthly plan price for OpenAI tools (Codex).
    pub openai_plan_usd: f64,
    /// Monthly plan price for Anthropic tools (Claude Code).
    pub claude_plan_usd: f64,
    /// Monthly plan price for Google tools (Antigravity).
    pub google_plan_usd: f64,
    /// Monthly plan price for other tools.
    pub other_plan_usd: f64,
    /// ISO timestamp of last update.
    pub updated_at: String,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            id: "local_user".to_owned(),
            username: "developer".to_owned(),
            display_name: "Local Developer".to_owned(),
            openai_plan_usd: 20.0,
            claude_plan_usd: 20.0,
            google_plan_usd: 0.0,
            other_plan_usd: 0.0,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

impl Database {
    /// Returns the current global routing and failover policy.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn get_routing_policy(&self) -> Result<RoutingPolicy, Error> {
        let cfg = self.get_context_relay_config()?;
        Ok(match cfg.policy.as_str() {
            "auto" | "automatic" => RoutingPolicy::Automatic,
            "deny_cross_provider" => RoutingPolicy::DenyCrossProvider,
            _ => RoutingPolicy::AskFirst,
        })
    }

    /// Sets and saves the global routing and failover policy.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite failure.
    pub fn save_routing_policy(&self, policy: RoutingPolicy) -> Result<(), Error> {
        let mut cfg = self.get_context_relay_config().unwrap_or_default();
        cfg.policy = policy.as_str().to_owned();
        cfg.updated_at = Utc::now().to_rfc3339();
        self.save_context_relay_config(&cfg)
    }

    /// Returns whether automatic background desktop update checks are enabled (default `true`).
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_auto_update_enabled(&self) -> Result<bool, Error> {
        let val: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM desktop_preferences WHERE key = 'auto_update_enabled'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(val.as_deref() != Some("0"))
    }

    /// Persists the opt-out automatic desktop update check preference.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write failure.
    pub fn set_auto_update_enabled(&self, enabled: bool) -> Result<(), Error> {
        self.conn.execute(
            "INSERT INTO desktop_preferences(key, value) VALUES ('auto_update_enabled', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [if enabled { "1" } else { "0" }],
        )?;
        Ok(())
    }

    /// Returns the E3 off-by-default feature toggles (all `false` when unset).
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite query failure.
    pub fn get_feature_toggles(&self) -> Result<gnomcode_core::FeatureToggles, Error> {
        let val: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM desktop_preferences WHERE key = 'feature_toggles'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(val
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default())
    }

    /// Persists the E3 feature toggles in `desktop_preferences`.
    ///
    /// # Errors
    /// Returns [`Error`] on SQLite write or JSON serialization failure.
    pub fn set_feature_toggles(
        &self,
        toggles: &gnomcode_core::FeatureToggles,
    ) -> Result<(), Error> {
        let json = serde_json::to_string(toggles)?;
        self.conn.execute(
            "INSERT INTO desktop_preferences(key, value) VALUES ('feature_toggles', ?1)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [json],
        )?;
        Ok(())
    }

    /// Queries the local user profile and preferences.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_user_profile(&self) -> Result<UserProfile, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at
            FROM user_profile
            WHERE id = 'local_user'
            LIMIT 1
            "#,
        )?;
        let res = stmt
            .query_row([], |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    openai_plan_usd: row.get(3)?,
                    claude_plan_usd: row.get(4)?,
                    google_plan_usd: row.get(5)?,
                    other_plan_usd: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .optional()?;

        Ok(res.unwrap_or_default())
    }

    /// Updates or inserts the local user profile and plan settings.
    ///
    /// # Errors
    /// Returns [`Error`] if database update fails.
    pub fn update_user_profile(&self, p: &UserProfile) -> Result<(), Error> {
        let mut stmt = self.conn.prepare(
            r#"
            INSERT INTO user_profile (id, username, display_name, openai_plan_usd, claude_plan_usd, google_plan_usd, other_plan_usd, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(id) DO UPDATE SET
                username = excluded.username,
                display_name = excluded.display_name,
                openai_plan_usd = excluded.openai_plan_usd,
                claude_plan_usd = excluded.claude_plan_usd,
                google_plan_usd = excluded.google_plan_usd,
                other_plan_usd = excluded.other_plan_usd,
                updated_at = excluded.updated_at
            "#,
        )?;
        stmt.execute(params![
            "local_user",
            p.username,
            p.display_name,
            p.openai_plan_usd,
            p.claude_plan_usd,
            p.google_plan_usd,
            p.other_plan_usd,
            Utc::now().to_rfc3339()
        ])?;
        Ok(())
    }
}
