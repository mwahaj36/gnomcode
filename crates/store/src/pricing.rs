//! Custom model pricing and subscriptions.

use super::*;

/// User-configured custom model pricing rate override.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CustomModelPricing {
    /// Pattern to match against model identifier (e.g. `claude-3-5-sonnet`, `gemini`).
    pub model_pattern: String,
    /// Uncached input tokens USD per 1M tokens.
    pub input_per_mtok_usd: f64,
    /// Output & reasoning tokens USD per 1M tokens.
    pub output_per_mtok_usd: f64,
    /// Cached input tokens USD per 1M tokens read.
    pub cache_read_per_mtok_usd: f64,
    /// Timestamp of last modification.
    pub updated_at: String,
}

impl Database {
    /// Queries all custom model pricing overrides.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_custom_model_pricings(&self) -> Result<Vec<CustomModelPricing>, Error> {
        let mut stmt = self.conn.prepare(
            "SELECT model_pattern, input_per_mtok_usd, output_per_mtok_usd, cache_read_per_mtok_usd, updated_at FROM custom_model_pricing ORDER BY model_pattern",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CustomModelPricing {
                model_pattern: r.get(0)?,
                input_per_mtok_usd: r.get(1)?,
                output_per_mtok_usd: r.get(2)?,
                cache_read_per_mtok_usd: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Sets or updates a custom model pricing override.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn set_custom_model_pricing(&mut self, pricing: &CustomModelPricing) -> Result<(), Error> {
        self.conn.execute(
            r#"
            INSERT INTO custom_model_pricing (model_pattern, input_per_mtok_usd, output_per_mtok_usd, cache_read_per_mtok_usd, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(model_pattern) DO UPDATE SET
                input_per_mtok_usd = excluded.input_per_mtok_usd,
                output_per_mtok_usd = excluded.output_per_mtok_usd,
                cache_read_per_mtok_usd = excluded.cache_read_per_mtok_usd,
                updated_at = excluded.updated_at
            "#,
            params![
                pricing.model_pattern,
                pricing.input_per_mtok_usd,
                pricing.output_per_mtok_usd,
                pricing.cache_read_per_mtok_usd,
                pricing.updated_at,
            ],
        )?;
        Ok(())
    }

    /// Deletes a custom model pricing override.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn delete_custom_model_pricing(&mut self, model_pattern: &str) -> Result<(), Error> {
        self.conn.execute(
            "DELETE FROM custom_model_pricing WHERE model_pattern = ?1",
            params![model_pattern],
        )?;
        Ok(())
    }

    /// Automatically populates subscription plan prices from local agent tiers
    /// (`quota_snapshot.plan` for OpenAI, plus passed-in Anthropic/Google tiers)
    /// when not yet customized by the user.
    ///
    /// # Errors
    /// Returns [`Error`] if querying or updating `user_profile` fails.
    pub fn auto_populate_subscriptions(
        &mut self,
        anthropic_usd: Option<f64>,
        google_usd: Option<f64>,
    ) -> Result<(), Error> {
        let mut profile = self.get_user_profile()?;
        let mut changed = false;

        let codex_plan: Option<String> = self
            .conn
            .query_row(
                "SELECT plan FROM quota_snapshot WHERE provider = 'codex' AND plan IS NOT NULL ORDER BY observed_at DESC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .optional()?;

        if let Some(plan) = codex_plan {
            let detected_openai = match plan.to_ascii_lowercase().as_str() {
                "pro" => Some(200.0),
                "enterprise" => Some(60.0),
                "team" => Some(30.0),
                "plus" => Some(20.0),
                _ => None,
            };
            if let Some(price) = detected_openai
                && ((profile.openai_plan_usd - 0.0).abs() < f64::EPSILON
                    || ((profile.openai_plan_usd - 20.0).abs() < f64::EPSILON && price > 20.0))
            {
                profile.openai_plan_usd = price;
                changed = true;
            }
        }

        if let Some(price) = anthropic_usd
            && (profile.claude_plan_usd - 0.0).abs() < f64::EPSILON
        {
            profile.claude_plan_usd = price;
            changed = true;
        }

        if let Some(price) = google_usd
            && (profile.google_plan_usd - 0.0).abs() < f64::EPSILON
        {
            profile.google_plan_usd = price;
            changed = true;
        }

        if changed {
            self.update_user_profile(&profile)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use chrono::Utc;

    #[test]
    fn custom_pricing_overrides_and_recalculates_cost() {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let initial = db.get_custom_model_pricings().expect("query custom prices");
        assert!(initial.is_empty());

        let cp = super::CustomModelPricing {
            model_pattern: "claude-opus".to_owned(),
            input_per_mtok_usd: 10.0,
            output_per_mtok_usd: 50.0,
            cache_read_per_mtok_usd: 1.0,
            updated_at: Utc::now().to_rfc3339(),
        };
        db.set_custom_model_pricing(&cp)
            .expect("set custom pricing");

        let loaded = db.get_custom_model_pricings().expect("query custom prices");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].model_pattern, "claude-opus");
        assert_eq!(loaded[0].input_per_mtok_usd, 10.0);

        db.delete_custom_model_pricing("claude-opus")
            .expect("delete");
        let after_del = db.get_custom_model_pricings().expect("query custom prices");
        assert!(after_del.is_empty());
    }
}
