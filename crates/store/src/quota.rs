//! Quota snapshots, pools and Antigravity cross-checks.

use super::*;

impl Database {
    /// Queries the latest quota pool snapshot for each distinct `(provider, limit_id, window)`.
    ///
    /// # Errors
    /// Returns [`Error`] if query execution fails.
    pub fn get_latest_quota_snapshots(&self) -> Result<Vec<QuotaSnapshot>, Error> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT q1.provider, q1.limit_id, q1.window, q1.used_pct, q1.resets_at, q1.plan, q1.basis, q1.observed_at
            FROM quota_snapshot q1
            JOIN (
                SELECT provider, limit_id, window, MAX(observed_at) AS max_obs
                FROM quota_snapshot
                GROUP BY provider, limit_id, window
            ) q2 ON q1.provider = q2.provider
                AND q1.limit_id = q2.limit_id
                AND q1.window = q2.window
                AND q1.observed_at = q2.max_obs
            ORDER BY q1.provider, q1.limit_id
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            let win_str: String = row.get(2)?;
            let window = match win_str.as_str() {
                "five_hour" => QuotaWindow::FiveHour,
                "monthly" => QuotaWindow::Monthly,
                _ => QuotaWindow::Weekly,
            };
            let resets_str: Option<String> = row.get(4)?;
            let resets_at = resets_str.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .ok()
            });
            let obs_str: String = row.get(7)?;
            let observed_at = DateTime::parse_from_rfc3339(&obs_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(QuotaSnapshot {
                provider: row.get(0)?,
                limit_id: row.get(1)?,
                window,
                used_pct: row.get(3)?,
                resets_at,
                plan: row.get(5)?,
                basis: match row.get::<_, String>(6)?.as_str() {
                    "exact" => TrustLevel::Exact,
                    "estimated" => TrustLevel::Estimated,
                    "api" => TrustLevel::Api,
                    _ => TrustLevel::Unknown,
                },
                observed_at,
            })
        })?;

        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }

        out.retain(|q| !q.limit_id.to_lowercase().contains("placeholder"));
        self.apply_antigravity_resource_exhausted_crosscheck(&mut out)?;
        Ok(out)
    }

    /// Replaces `provider`'s pool membership (`(pool, model name)` pairs) as reported by its limits
    /// source. An empty list keeps what is stored, since a failed read says nothing about pools.
    ///
    /// # Errors
    /// Returns [`Error`] if the transaction fails.
    pub fn replace_quota_pool_members(
        &mut self,
        provider: &str,
        members: &[(String, String)],
    ) -> Result<(), Error> {
        if members.is_empty() {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM quota_pool_member WHERE provider = ?1",
            params![provider],
        )?;
        for (pool, member) in members {
            tx.execute(
                "INSERT OR IGNORE INTO quota_pool_member (provider, limit_id, member) VALUES (?1, ?2, ?3)",
                params![provider, pool, member],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Applies each recorded `RESOURCE_EXHAUSTED` error to the pool of the model that raised it.
    ///
    /// The model is mapped to a pool from the limits source's own data (the pool name and its
    /// listed models). Exactly one pool: its unreset windows read 100% used, `exact`. No pool or
    /// several: no reading is changed and an `estimated` exhausted hint for that model is added.
    pub(crate) fn apply_antigravity_resource_exhausted_crosscheck(
        &self,
        snapshots: &mut Vec<QuotaSnapshot>,
    ) -> Result<(), Error> {
        let mut stmt = self.conn.prepare(
            "SELECT a.tool_name, MAX(a.timestamp) FROM activity_event a
             JOIN session s ON s.id = a.session_id
             WHERE s.agent = 'antigravity' AND a.kind = 'rate_limited'
             GROUP BY a.tool_name",
        )?;
        let errors: Vec<(Option<String>, DateTime<Utc>)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(Result::ok)
            .filter_map(|(model, at)| {
                let at = DateTime::parse_from_rfc3339(&at).ok()?.with_timezone(&Utc);
                Some((model, at))
            })
            .collect();
        if errors.is_empty() {
            return Ok(());
        }
        let mut members = self.conn.prepare(
            "SELECT limit_id, member FROM quota_pool_member WHERE provider = 'antigravity'",
        )?;
        let members: Vec<(String, String)> = members
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(Result::ok)
            .collect();
        // Added rows are never newer than the provider's newest reading, so the API does not
        // treat them as a fresher report and supersede the other pools.
        let newest = snapshots
            .iter()
            .filter(|snap| snap.provider == "antigravity")
            .map(|snap| snap.observed_at)
            .max();
        let now = Utc::now();

        for (model, rl_dt) in errors {
            let mut pools: Vec<&str> = Vec::new();
            if let Some(model) = model.as_deref() {
                for snap in snapshots
                    .iter()
                    .filter(|snap| snap.provider == "antigravity")
                {
                    let pool = snap.limit_id.as_str();
                    let covers = gnomcode_core::quota::model_matches_pool_name(model, pool)
                        || members.iter().any(|(id, member)| {
                            id == pool
                                && gnomcode_core::quota::model_matches_pool_name(model, member)
                        });
                    if covers && !pools.contains(&pool) {
                        pools.push(pool);
                    }
                }
            }
            if let [pool] = pools[..] {
                let pool = pool.to_owned();
                for snap in snapshots
                    .iter_mut()
                    .filter(|snap| snap.provider == "antigravity" && snap.limit_id == pool)
                {
                    let unreset = snap.resets_at.is_none_or(|reset| rl_dt < reset);
                    let recent = rl_dt >= snap.observed_at - chrono::Duration::hours(5);
                    if snap.used_pct < 100.0 && unreset && recent {
                        snap.used_pct = 100.0;
                        snap.basis = TrustLevel::Exact;
                        snap.observed_at =
                            snap.observed_at.max(newest.map_or(rl_dt, |n| rl_dt.min(n)));
                    }
                }
            } else if rl_dt > now - chrono::Duration::hours(5) {
                snapshots.push(QuotaSnapshot {
                    provider: "antigravity".to_owned(),
                    limit_id: model.unwrap_or_else(|| "antigravity".to_owned()),
                    window: QuotaWindow::FiveHour,
                    used_pct: 100.0,
                    resets_at: None,
                    plan: None,
                    basis: TrustLevel::Estimated,
                    observed_at: newest.map_or(rl_dt, |n| rl_dt.min(n)),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, SourceCheckpoint};
    use chrono::Utc;
    use gnomcode_core::TrustLevel;

    fn pool(
        id: &str,
        window: gnomcode_core::QuotaWindow,
        used: f64,
        now: chrono::DateTime<Utc>,
    ) -> gnomcode_core::QuotaSnapshot {
        gnomcode_core::QuotaSnapshot {
            provider: "antigravity".to_owned(),
            limit_id: id.to_owned(),
            window,
            used_pct: used,
            resets_at: Some(now + chrono::Duration::hours(4)),
            plan: None,
            basis: TrustLevel::Api,
            observed_at: now + chrono::Duration::seconds(30),
        }
    }

    /// Commits Antigravity pools plus one `RESOURCE_EXHAUSTED` error that named `model`, then
    /// returns the cross-checked latest readings as `(pool, window, used, basis)`.
    fn crosscheck(
        model: Option<&str>,
        age: chrono::Duration,
        members: &[(&str, &str)],
        pools: &[gnomcode_core::QuotaSnapshot],
    ) -> Vec<(String, gnomcode_core::QuotaWindow, f64, TrustLevel)> {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut db = Database::open(&dir.path().join("store.db")).expect("open");
        let now = Utc::now();
        let proj = gnomcode_core::Project::from_origin(None, "/repo", None);
        let sess = gnomcode_core::Session {
            id: "antigravity:ag-1".to_owned(),
            agent: gnomcode_core::AgentId::Antigravity,
            surface: gnomcode_core::Surface::Cli,
            project_id: proj.id.clone(),
            parent_session_id: None,
            model: model.map(str::to_owned),
            title: None,
            state: gnomcode_core::SessionState::Error,
            pid: None,
            started_at: now - age,
            last_event_at: now - age,
        };
        let act = gnomcode_core::ActivityEvent {
            session_id: sess.id.clone(),
            timestamp: now - age,
            kind: gnomcode_core::ActivityKind::RateLimited,
            tool_name: model.map(str::to_owned),
            summary_redacted: Some("RESOURCE_EXHAUSTED".to_owned()),
            duration_ms: None,
        };
        let cp = SourceCheckpoint {
            file_path: "ag.jsonl".to_owned(),
            last_read_offset: 1,
            file_size: 1,
            fingerprint: "fp".to_owned(),
            last_ingested_at: now,
        };
        db.commit_batch(&[proj], &[sess], &[], &[act], pools, &cp)
            .expect("commit");
        let members: Vec<_> = members
            .iter()
            .map(|(p, m)| ((*p).to_owned(), (*m).to_owned()))
            .collect();
        db.replace_quota_pool_members("antigravity", &members)
            .expect("members");
        db.get_latest_quota_snapshots()
            .expect("latest")
            .into_iter()
            .map(|q| (q.limit_id, q.window, q.used_pct, q.basis))
            .collect()
    }

    const FIVE: gnomcode_core::QuotaWindow = gnomcode_core::QuotaWindow::FiveHour;

    const WEEK: gnomcode_core::QuotaWindow = gnomcode_core::QuotaWindow::Weekly;

    const MEMBERS: [(&str, &str); 5] = [
        ("Gemini Models", "Gemini Models"),
        ("Gemini Models", "Gemini Pro"),
        ("Claude and GPT models", "Claude and GPT models"),
        ("Claude and GPT models", "Claude Opus"),
        ("Claude and GPT models", "GPT-OSS"),
    ];

    fn two_pools(now: chrono::DateTime<Utc>) -> Vec<gnomcode_core::QuotaSnapshot> {
        vec![
            pool("Claude and GPT models", FIVE, 0.0, now),
            pool("Claude and GPT models", WEEK, 10.0, now),
            pool("Gemini Models", FIVE, 20.0, now),
            pool("Gemini Models", WEEK, 30.0, now),
        ]
    }

    #[test]
    fn antigravity_exhausted_error_marks_only_the_pool_of_its_model() {
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 4, "no hint row when the model maps");
        for (id, window, used, basis) in rows {
            if id == "Claude and GPT models" {
                assert_eq!((used, basis), (100.0, TrustLevel::Exact), "{window:?}");
            } else {
                assert_eq!(basis, TrustLevel::Api, "{id} must stay untouched");
                assert!(used < 100.0);
            }
        }
    }

    #[test]
    fn antigravity_exhausted_error_maps_a_model_named_by_the_older_per_model_source() {
        let now = Utc::now();
        let pools = [
            pool("Claude Opus 5.5", FIVE, 0.0, now),
            pool("Gemini 3.1 Pro", FIVE, 0.0, now),
        ];
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &[],
            &pools,
        );
        assert_eq!(rows.len(), 2);
        assert!(
            rows.iter()
                .any(|r| r.0 == "Claude Opus 5.5" && r.2 == 100.0 && r.3 == TrustLevel::Exact)
        );
        assert!(
            rows.iter()
                .any(|r| r.0 == "Gemini 3.1 Pro" && r.2 == 0.0 && r.3 == TrustLevel::Api)
        );
    }

    #[test]
    fn antigravity_exhausted_error_for_an_unmapped_model_marks_nothing_exact() {
        let rows = crosscheck(
            Some("Mystery Model 9"),
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 5);
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(
            rows.iter()
                .filter(|r| r.0 != "Mystery Model 9")
                .all(|r| r.2 < 100.0 && r.3 == TrustLevel::Api)
        );
        let hint = rows
            .iter()
            .find(|r| r.0 == "Mystery Model 9")
            .expect("hint row");
        assert_eq!((hint.2, hint.3), (100.0, TrustLevel::Estimated));
    }

    #[test]
    fn antigravity_exhausted_error_without_a_model_is_only_an_estimated_hint() {
        let rows = crosscheck(
            None,
            chrono::Duration::minutes(1),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(
            rows.iter()
                .any(|r| r.0 == "antigravity" && r.3 == TrustLevel::Estimated)
        );
    }

    #[test]
    fn antigravity_exhausted_error_with_no_pool_data_marks_nothing_exact() {
        let rows = crosscheck(
            Some("Claude Opus 5.5 (High)"),
            chrono::Duration::minutes(1),
            &[],
            &two_pools(Utc::now()),
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
        assert!(rows.iter().any(|r| r.3 == TrustLevel::Estimated));
    }

    #[test]
    fn antigravity_exhausted_error_matching_two_pools_is_not_guessed() {
        let now = Utc::now();
        let pools = [
            pool("Claude A", FIVE, 0.0, now),
            pool("Claude B", FIVE, 0.0, now),
        ];
        let members = [("Claude A", "Claude Opus"), ("Claude B", "Claude Opus")];
        let rows = crosscheck(
            Some("Claude Opus 5.5"),
            chrono::Duration::minutes(1),
            &members,
            &pools,
        );
        assert!(rows.iter().all(|r| r.3 != TrustLevel::Exact));
    }

    #[test]
    fn antigravity_old_exhausted_error_leaves_readings_and_drops_the_hint() {
        let rows = crosscheck(
            Some("Mystery Model 9"),
            chrono::Duration::hours(6),
            &MEMBERS,
            &two_pools(Utc::now()),
        );
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.3 == TrustLevel::Api));
    }

    #[test]
    fn pool_members_survive_an_empty_refresh_and_are_replaced_by_a_new_list() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut db = Database::open(&dir.path().join("store.db")).expect("open");
        let count = |db: &Database| -> i64 {
            db.conn
                .query_row("SELECT COUNT(*) FROM quota_pool_member", [], |row| {
                    row.get(0)
                })
                .expect("count")
        };
        let list = vec![
            ("A".to_owned(), "x".to_owned()),
            ("A".to_owned(), "y".to_owned()),
        ];
        db.replace_quota_pool_members("antigravity", &list)
            .expect("first");
        db.replace_quota_pool_members("antigravity", &[])
            .expect("empty");
        assert_eq!(count(&db), 2);
        db.replace_quota_pool_members("antigravity", &list[..1])
            .expect("replace");
        assert_eq!(count(&db), 1);
    }
}
