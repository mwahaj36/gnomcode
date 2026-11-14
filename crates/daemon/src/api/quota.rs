//! Quota pool handlers and view builders.

use super::*;

pub(super) async fn quota_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let quota = db
        .get_latest_quota_snapshots()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(quota_views_with_issues(quota)))
}

#[derive(Serialize)]
pub(crate) struct QuotaPoolView {
    #[serde(flatten)]
    pub(super) snapshot: gnomcode_core::QuotaSnapshot,
    pub(super) remaining_pct: Option<f64>,
    /// The recorded window has reset; its old reading no longer describes capacity.
    pub(super) reset_passed: bool,
    /// Display state derived in Rust; interfaces never recompute it.
    pub(super) state: gnomcode_core::quota::QuotaState,
    /// Usage pace for exact or API readings, when it can be computed honestly.
    pub(super) pace: Option<gnomcode_core::quota::QuotaPace>,
    /// The reading is older than the stale threshold and is shown as last seen.
    pub(super) stale: bool,
    /// Display label: the window label, prefixed by the pool name (middle dot separator)
    /// when the provider has several pools.
    pub(super) label: String,
    /// Pool display name; only set when the provider has more than one distinct pool.
    pub(super) pool: Option<String>,
    /// Window label: "Session", "Weekly" or "Monthly".
    pub(super) window_label: String,
    /// Why the opt-in live source is not refreshing this reading (e.g. expired Claude sign-in).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) source_issue: Option<String>,
}

/// Quota views for the local API, annotated with why an opt-in live source has stopped updating.
pub(super) fn quota_views_with_issues(
    snapshots: Vec<gnomcode_core::QuotaSnapshot>,
) -> Vec<QuotaPoolView> {
    let mut views = quota_views(snapshots);
    for view in &mut views {
        view.source_issue = crate::vendor_limits::issue(&view.snapshot.provider);
    }
    views
}

/// Display name for a pool id. Snake or kebab ids (`gemini_pro`) are title-cased
/// (`Gemini Pro`); ids that are already readable (`Claude and GPT models`) are kept verbatim.
pub(super) fn pool_display_name(identifier: &str) -> String {
    let is_slug = !identifier.is_empty()
        && identifier
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
    if !is_slug {
        return identifier.to_owned();
    }
    identifier
        .split(['_', '-'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn quota_views(snapshots: Vec<gnomcode_core::QuotaSnapshot>) -> Vec<QuotaPoolView> {
    quota_views_at(snapshots, chrono::Utc::now())
}

pub(crate) fn quota_views_at(
    snapshots: Vec<gnomcode_core::QuotaSnapshot>,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<QuotaPoolView> {
    use gnomcode_core::quota;
    // Every source reports all of a provider's pools at once, so rows older than the provider's
    // latest snapshot belong to renamed or regrouped pools and are superseded.
    let mut latest = std::collections::HashMap::new();
    for snapshot in &snapshots {
        let seen = latest
            .entry(snapshot.provider.clone())
            .or_insert(snapshot.observed_at);
        *seen = (*seen).max(snapshot.observed_at);
    }
    let kept: Vec<_> = snapshots
        .into_iter()
        .filter(|snapshot| {
            latest
                .get(&snapshot.provider)
                .is_none_or(|at| *at - snapshot.observed_at <= chrono::Duration::minutes(10))
        })
        // A pool whose window reset over a week ago without a new reading is retired, not "awaiting".
        .filter(|snapshot| {
            snapshot
                .resets_at
                .is_none_or(|reset| now - reset < chrono::Duration::days(7))
        })
        .collect();
    let mut pools_by_provider: std::collections::HashMap<
        String,
        std::collections::HashSet<String>,
    > = std::collections::HashMap::new();
    for snapshot in &kept {
        pools_by_provider
            .entry(snapshot.provider.clone())
            .or_default()
            .insert(snapshot.limit_id.clone());
    }
    kept.into_iter()
        .map(|mut snapshot| {
            let window = match snapshot.window {
                gnomcode_core::QuotaWindow::FiveHour => "Session",
                gnomcode_core::QuotaWindow::Weekly => "Weekly",
                gnomcode_core::QuotaWindow::Monthly => "Monthly",
            };
            let pool = pools_by_provider
                .get(&snapshot.provider)
                .is_some_and(|pools| pools.len() > 1)
                .then(|| pool_display_name(&snapshot.limit_id));
            let label = pool.as_ref().map_or_else(
                || window.to_owned(),
                |name| format!("{name} \u{b7} {window}"),
            );
            let remaining_pct = quota::remaining_pct(&snapshot, now);
            let reset_passed = quota::reset_passed(&snapshot, now);
            let state = quota::quota_state(&snapshot, now);
            let pace = quota::quota_pace(&snapshot, now);
            let stale = quota::is_stale(&snapshot, now);
            snapshot.used_pct = quota::effective_used_pct(&snapshot, now);
            snapshot.basis = quota::effective_basis(&snapshot, now);
            QuotaPoolView {
                remaining_pct,
                reset_passed,
                state,
                pace,
                stale,
                label,
                pool,
                window_label: window.to_owned(),
                snapshot,
                source_issue: None,
            }
        })
        .collect()
}

pub(crate) fn conversation_quota(
    snapshots: &[gnomcode_core::QuotaSnapshot],
    agent: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> Vec<Value> {
    let provider = crate::agent_for_model("@default", Some(agent)).unwrap_or(agent);
    let pools: std::collections::HashSet<_> = snapshots
        .iter()
        .filter(|snapshot| snapshot.provider == provider)
        .map(|snapshot| snapshot.limit_id.as_str())
        .collect();
    quota_views_at(snapshots.iter().filter(|snapshot|snapshot.provider==provider).cloned().collect(), now).into_iter().map(|pool| {
        let window = match pool.snapshot.window {gnomcode_core::QuotaWindow::FiveHour=>"5h",gnomcode_core::QuotaWindow::Weekly=>"Weekly",gnomcode_core::QuotaWindow::Monthly=>"Monthly"};
        let label = if pools.len()>1 {format!("{} {window}",pool.snapshot.limit_id.replace(['-','_']," "))}else{window.to_owned()};
        let expired = pool.reset_passed;
        let stale = expired || pool.stale;
        json!({"label":label,"state":pool.state,"remaining_pct":pool.remaining_pct,"used_pct":pool.snapshot.used_pct,"basis":pool.snapshot.basis,"observed_at":pool.snapshot.observed_at,"stale":stale,"window":pool.snapshot.window,"window_label":pool.window_label,"pool":pool.pool,"resets_at":pool.snapshot.resets_at,"reset_passed":pool.reset_passed,"pace":pool.pace})
    }).collect()
}

/// Shared test fixture for quota and provider tests.
#[cfg(test)]
pub(super) fn pool(
    provider: &str,
    limit_id: &str,
    window: gnomcode_core::QuotaWindow,
    used_pct: f64,
    resets_at: Option<chrono::DateTime<chrono::Utc>>,
    basis: gnomcode_core::TrustLevel,
    observed_at: chrono::DateTime<chrono::Utc>,
) -> gnomcode_core::QuotaSnapshot {
    gnomcode_core::QuotaSnapshot {
        provider: provider.into(),
        limit_id: limit_id.into(),
        window,
        used_pct,
        resets_at,
        plan: None,
        basis,
        observed_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn quota_view_preserves_unknown_capacity() {
        let mut quota: gnomcode_core::QuotaSnapshot = serde_json::from_value(json!({
            "provider": "sample", "limit_id": "sample", "window": "weekly", "used_pct": 62.0,
            "resets_at": null, "plan": null, "basis": "estimated", "observed_at": "2026-10-28T00:00:00Z"
        })).expect("quota");
        assert_eq!(
            quota_views(vec![quota.clone()])[0].remaining_pct,
            Some(38.0)
        );
        quota.basis = gnomcode_core::TrustLevel::Unknown;
        assert_eq!(quota_views(vec![quota])[0].remaining_pct, None);
    }

    #[test]
    fn label_prefixes_pool_when_provider_has_several() {
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let views = super::quota_views_at(
            vec![
                pool(
                    "antigravity",
                    "gemini_pro",
                    QuotaWindow::Weekly,
                    10.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "antigravity",
                    "claude-opus",
                    QuotaWindow::FiveHour,
                    10.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
        );
        let labels: Vec<_> = views.iter().map(|view| view.label.as_str()).collect();
        assert_eq!(
            labels,
            ["Gemini Pro \u{b7} Weekly", "Claude Opus \u{b7} Session"]
        );
    }

    #[test]
    fn label_plain_for_single_pool() {
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let views = super::quota_views_at(
            vec![
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::FiveHour,
                    10.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Weekly,
                    10.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Monthly,
                    10.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
        );
        let labels: Vec<_> = views.iter().map(|view| view.label.as_str()).collect();
        assert_eq!(labels, ["Session", "Weekly", "Monthly"]);
    }

    #[test]
    fn readable_pool_names_stay_verbatim_and_split_into_pool_and_window() {
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let views = super::quota_views_at(
            vec![
                pool(
                    "antigravity",
                    "Claude and GPT models",
                    QuotaWindow::Weekly,
                    10.0,
                    reset,
                    TrustLevel::Api,
                    now,
                ),
                pool(
                    "antigravity",
                    "Gemini 4 Argon",
                    QuotaWindow::FiveHour,
                    10.0,
                    reset,
                    TrustLevel::Api,
                    now,
                ),
                pool(
                    "antigravity",
                    "gemini_pro",
                    QuotaWindow::Weekly,
                    10.0,
                    reset,
                    TrustLevel::Api,
                    now,
                ),
            ],
            now,
        );
        assert_eq!(views[0].pool.as_deref(), Some("Claude and GPT models"));
        assert_eq!(views[0].window_label, "Weekly");
        assert_eq!(views[0].label, "Claude and GPT models \u{b7} Weekly");
        assert_eq!(views[1].pool.as_deref(), Some("Gemini 4 Argon"));
        assert_eq!(views[2].pool.as_deref(), Some("Gemini Pro"));
        let single = super::quota_views_at(
            vec![pool(
                "codex",
                "codex",
                QuotaWindow::Weekly,
                10.0,
                reset,
                TrustLevel::Exact,
                now,
            )],
            now,
        );
        assert_eq!(single[0].pool, None);
        assert_eq!(single[0].window_label, "Weekly");
        let json = serde_json::to_value(&single[0]).expect("json");
        assert!(json["pool"].is_null());
        assert_eq!(json["window_label"], "Weekly");
    }

    #[test]
    fn view_carries_state_and_pace() {
        use gnomcode_core::quota::{PaceVerdict, QuotaState};
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        // Half of a weekly window elapsed with 80% used: ahead of pace, and low is not yet reached.
        let ahead = pool(
            "codex",
            "codex",
            QuotaWindow::Weekly,
            80.0,
            Some(now + chrono::Duration::hours(84)),
            TrustLevel::Exact,
            now,
        );
        let mut stale_estimate = ahead.clone();
        stale_estimate.basis = TrustLevel::Estimated;
        stale_estimate.observed_at = now - chrono::Duration::minutes(5);
        let views = super::quota_views_at(vec![ahead], now);
        assert_eq!(views[0].state, QuotaState::Ok);
        assert_eq!(
            views[0].pace.as_ref().map(|pace| pace.verdict),
            Some(PaceVerdict::Ahead)
        );
        assert!(!views[0].stale);
        let views = super::quota_views_at(vec![stale_estimate], now);
        assert!(views[0].pace.is_none(), "estimates never carry pace");
        assert!(views[0].stale);
        let json = serde_json::to_value(&views[0]).expect("json");
        assert_eq!(json["state"], "ok");
        assert_eq!(json["label"], "Weekly");
        assert_eq!(json["remaining_pct"], 20.0);
    }

    #[test]
    fn quota_pools_say_why_their_live_source_is_stale() {
        let quota: gnomcode_core::QuotaSnapshot = serde_json::from_value(json!({
            "provider": "issue-test", "limit_id": "claude", "window": "weekly", "used_pct": 2.0,
            "resets_at": null, "plan": null, "basis": "exact", "observed_at": "2026-11-12T03:37:17Z"
        }))
        .expect("quota");
        let plain = serde_json::to_value(quota_views(vec![quota.clone()])).expect("json");
        assert!(plain[0].get("source_issue").is_none());
        crate::vendor_limits::record_issue("issue-test", Some("Claude sign-in expired".to_owned()));
        let views = serde_json::to_value(quota_views_with_issues(vec![quota])).expect("json");
        crate::vendor_limits::record_issue("issue-test", None);
        assert_eq!(views[0]["source_issue"], "Claude sign-in expired");
        assert_eq!(views[0]["remaining_pct"], 98.0);
    }

    #[test]
    fn conversation_limits_follow_provider_observations_and_expire_after_reset() {
        let now = chrono::Utc::now();
        let mut snapshot = gnomcode_core::QuotaSnapshot {
            provider: "codex".into(),
            limit_id: "codex".into(),
            window: gnomcode_core::QuotaWindow::FiveHour,
            used_pct: 25.0,
            resets_at: Some(now + chrono::Duration::hours(1)),
            plan: None,
            basis: gnomcode_core::TrustLevel::Exact,
            observed_at: now,
        };
        let views = super::conversation_quota(std::slice::from_ref(&snapshot), "codex", now);
        assert_eq!(views[0]["remaining_pct"], 75.0);
        assert_eq!(views[0]["label"], "5h");
        assert_eq!(views[0]["window_label"], "Session");
        assert!(views[0]["resets_at"].is_string());
        assert!(views[0].get("pool").is_some() && views[0].get("pace").is_some());
        assert_eq!(views[0]["stale"], false);
        snapshot.used_pct = 40.0;
        assert_eq!(
            super::conversation_quota(std::slice::from_ref(&snapshot), "codex", now)[0]["remaining_pct"],
            60.0
        );
        assert!(
            super::conversation_quota(std::slice::from_ref(&snapshot), "claude-code", now)
                .is_empty()
        );
        assert_eq!(
            super::conversation_quota(
                std::slice::from_ref(&snapshot),
                "codex",
                now + chrono::Duration::minutes(3)
            )[0]["stale"],
            true
        );
        let after_reset = super::conversation_quota(
            std::slice::from_ref(&snapshot),
            "codex",
            now + chrono::Duration::hours(2),
        );
        assert_eq!(after_reset[0]["state"], "reset_unconfirmed");
        assert_eq!(after_reset[0]["remaining_pct"], 100.0);
        assert_eq!(after_reset[0]["used_pct"], 0.0);
        assert_eq!(after_reset[0]["basis"], "estimated");
        snapshot.basis = gnomcode_core::TrustLevel::Unknown;
        assert!(super::conversation_quota(&[snapshot], "codex", now)[0]["remaining_pct"].is_null());
    }

    #[test]
    fn quota_views_report_assumed_reset_and_omit_stale_runs_out() {
        let now = chrono::Utc::now();
        let mut stale_ahead = pool(
            "codex",
            "codex",
            gnomcode_core::QuotaWindow::FiveHour,
            99.0,
            Some(now + chrono::Duration::minutes(66)),
            gnomcode_core::TrustLevel::Exact,
            now - chrono::Duration::hours(2),
        );
        let stale_views = super::quota_views_at(vec![stale_ahead.clone()], now);
        assert!(stale_views[0].stale);
        let pace = stale_views[0].pace.as_ref().expect("pace");
        assert!(pace.stale);
        assert!(pace.stale_reason.is_some());
        assert_eq!(pace.runs_out_at, None);

        stale_ahead.resets_at = Some(now - chrono::Duration::minutes(5));
        let reset_views = super::quota_views_at(vec![stale_ahead], now);
        let json = serde_json::to_value(&reset_views[0]).expect("json");
        assert_eq!(json["state"], "reset_unconfirmed");
        assert_eq!(json["used_pct"], 0.0);
        assert_eq!(json["remaining_pct"], 100.0);
        assert_eq!(json["basis"], "estimated");
        assert_eq!(json["reset_passed"], true);
    }
}
