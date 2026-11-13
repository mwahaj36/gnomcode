//! Provider overview handlers and view builders.

use super::*;

/// Recorded spend for one provider (SPEC 3.9); an estimate unless every turn was priced.
#[derive(Serialize)]
pub(crate) struct ProviderCost {
    today_usd: f64,
    last_30_days_usd: f64,
    basis: gnomcode_core::TrustLevel,
}

/// One provider card on the Limits view: windows, state and, when empty, how to turn them on.
#[derive(Serialize)]
pub(crate) struct ProviderOverview {
    /// Provider slug, e.g. `claude-code`.
    provider: String,
    display_name: String,
    /// First non-empty plan among the provider's windows.
    plan: Option<String>,
    /// Worst window by severity; unknown when there are no windows.
    state: gnomcode_core::quota::QuotaState,
    /// Sorted: session, weekly, monthly, then label.
    windows: Vec<QuotaPoolView>,
    /// Index into `windows` of the tightest window: highest severity, ties by lowest
    /// remaining percentage. `None` when there are no windows.
    headline_window: Option<usize>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    /// "Local logs", "Status line", "Provider API", "Native limits" or "Off".
    source_label: String,
    /// Shown when `windows` is empty.
    enable_hint: Option<String>,
    enable_command: Option<String>,
    /// `None` when nothing was recorded in the last 30 days.
    cost: Option<ProviderCost>,
    /// Why the opt-in live source is not refreshing this provider's readings.
    #[serde(skip_serializing_if = "Option::is_none")]
    source_issue: Option<String>,
}

pub(super) const CORE_PROVIDERS: [&str; 3] = ["claude-code", "codex", "antigravity"];

/// Sums recorded spend per agent slug. `ProviderReport.provider` is a display label
/// ("Claude Code", "OpenAI Codex"), so spend is keyed on `ModelReport.agent`, the same
/// slug quota snapshots use for `provider`.
pub(super) fn provider_costs(
    today: &gnomcode_store::TimelineReport,
    last_30_days: &gnomcode_store::TimelineReport,
) -> std::collections::HashMap<String, ProviderCost> {
    use std::collections::HashMap;
    fn per_agent(report: &gnomcode_store::TimelineReport) -> HashMap<String, (f64, u64)> {
        let mut sums: HashMap<String, (f64, u64)> = HashMap::new();
        for model in report.providers.iter().flat_map(|p| &p.models) {
            let entry = sums.entry(model.agent.to_lowercase()).or_default();
            entry.0 += model.cost_usd;
            entry.1 += model.unpriced_turns;
        }
        sums
    }
    let today = per_agent(today);
    per_agent(last_30_days)
        .into_iter()
        .filter(|(_, (spend, _))| *spend > 0.0)
        .map(|(agent, (month_usd, unpriced))| {
            let today_usd = today.get(&agent).map_or(0.0, |(spend, _)| *spend);
            let basis = if unpriced > 0 {
                gnomcode_core::TrustLevel::Unknown
            } else {
                gnomcode_core::TrustLevel::Estimated
            };
            (
                agent,
                ProviderCost {
                    today_usd,
                    last_30_days_usd: month_usd,
                    basis,
                },
            )
        })
        .collect()
}

/// Display name, hint and enable command for a provider without readings.
pub(super) fn provider_empty_state(
    slug: &str,
    claude_api: bool,
    antigravity_api: bool,
) -> (String, String, Option<String>) {
    let enable = |name: &str| Some(format!("gnomcode limits enable {name}"));
    match slug {
        "claude-code" if claude_api => (
            "Claude Code".into(),
            "Waiting for the first Claude reading.".into(),
            None,
        ),
        "claude-code" => (
            "Claude Code".into(),
            "Limits are off for Claude Code.".into(),
            enable("claude"),
        ),
        "codex" => (
            "Codex".into(),
            "Appears after your next Codex request.".into(),
            None,
        ),
        "antigravity" if antigravity_api => (
            "Antigravity".into(),
            "Start an Antigravity agent to read limits.".into(),
            None,
        ),
        "antigravity" => (
            "Antigravity".into(),
            "Limits are off for Antigravity.".into(),
            enable("antigravity"),
        ),
        other => (pool_display_name(other), "No reading yet.".into(), None),
    }
}

pub(super) fn provider_display_name(slug: &str) -> String {
    match slug {
        "claude-code" => "Claude Code".into(),
        "codex" => "Codex".into(),
        "antigravity" => "Antigravity".into(),
        other => pool_display_name(other),
    }
}

/// Builds one overview per provider from already-derived quota views.
pub(super) fn provider_overviews(
    views: Vec<QuotaPoolView>,
    mut costs: std::collections::HashMap<String, ProviderCost>,
    claude_api: bool,
    antigravity_api: bool,
) -> Vec<ProviderOverview> {
    use gnomcode_core::TrustLevel;
    let mut by_provider: std::collections::BTreeMap<String, Vec<QuotaPoolView>> =
        std::collections::BTreeMap::new();
    for provider in CORE_PROVIDERS {
        by_provider.entry(provider.to_owned()).or_default();
    }
    for view in views {
        by_provider
            .entry(view.snapshot.provider.clone())
            .or_default()
            .push(view);
    }
    let mut order: Vec<String> = CORE_PROVIDERS.iter().map(|p| (*p).to_owned()).collect();
    order.extend(
        by_provider
            .keys()
            .filter(|slug| !CORE_PROVIDERS.contains(&slug.as_str()))
            .cloned(),
    );
    order
        .into_iter()
        .filter_map(|slug| {
            let mut windows = by_provider.remove(&slug)?;
            windows.sort_by(|a, b| {
                (window_rank(a.snapshot.window), &a.label)
                    .cmp(&(window_rank(b.snapshot.window), &b.label))
            });
            let has_exact = windows
                .iter()
                .any(|w| w.snapshot.basis == TrustLevel::Exact);
            let source_label = match slug.as_str() {
                "claude-code" if claude_api => "Provider API",
                "claude-code" if has_exact => "Status line",
                "claude-code" => "Off",
                "antigravity" if antigravity_api => "Native limits",
                "antigravity" => "Off",
                _ => "Local logs",
            };
            let (display_name, enable_hint, enable_command) = if windows.is_empty() {
                let (name, hint, command) =
                    provider_empty_state(&slug, claude_api, antigravity_api);
                (name, Some(hint), command)
            } else {
                (provider_display_name(&slug), None, None)
            };
            Some(ProviderOverview {
                headline_window: headline_window(&windows),
                plan: windows.iter().find_map(|w| w.snapshot.plan.clone()),
                state: provider_state(&windows),
                updated_at: windows.iter().map(|w| w.snapshot.observed_at).max(),
                cost: costs.remove(&slug),
                source_issue: crate::vendor_limits::issue(&slug),
                provider: slug,
                display_name,
                windows,
                source_label: source_label.to_owned(),
                enable_hint,
                enable_command,
            })
        })
        .collect()
}

fn is_multi_pool(windows: &[QuotaPoolView]) -> bool {
    let mut distinct = std::collections::HashSet::new();
    for w in windows {
        distinct.insert(w.pool.as_deref().unwrap_or(&w.snapshot.limit_id));
        if distinct.len() > 1 {
            return true;
        }
    }
    false
}

/// Provider card state: for multi-pool providers, `Exhausted` only when all pools are exhausted
/// and `Low` when at least one pool is exhausted while another still has capacity; otherwise
/// highest window severity.
pub(super) fn provider_state(windows: &[QuotaPoolView]) -> gnomcode_core::quota::QuotaState {
    use gnomcode_core::quota::QuotaState;
    if windows.is_empty() {
        return QuotaState::Unknown;
    }
    if is_multi_pool(windows) {
        if windows.iter().all(|w| w.state == QuotaState::Exhausted) {
            return QuotaState::Exhausted;
        }
        let some_exhausted = windows.iter().any(|w| w.state == QuotaState::Exhausted);
        let some_available = windows
            .iter()
            .any(|w| matches!(w.state, QuotaState::Ok | QuotaState::Low));
        if some_exhausted && some_available {
            return QuotaState::Low;
        }
    }
    windows
        .iter()
        .map(|w| w.state)
        .max_by_key(|state| state.severity())
        .unwrap_or(QuotaState::Unknown)
}

/// Index of the window with the highest severity, preferring non-exhausted windows on multi-pool
/// providers when capacity remains; ties broken by the lowest remaining percentage.
pub(super) fn headline_window(windows: &[QuotaPoolView]) -> Option<usize> {
    use gnomcode_core::quota::QuotaState;
    let skip_exhausted = is_multi_pool(windows)
        && windows
            .iter()
            .any(|w| matches!(w.state, QuotaState::Ok | QuotaState::Low));
    windows
        .iter()
        .enumerate()
        .filter(|(_, w)| !skip_exhausted || w.state != QuotaState::Exhausted)
        .min_by(|(_, a), (_, b)| {
            b.state.severity().cmp(&a.state.severity()).then_with(|| {
                let left = |w: &QuotaPoolView| w.remaining_pct.unwrap_or(f64::INFINITY);
                left(a).total_cmp(&left(b))
            })
        })
        .map(|(index, _)| index)
}

pub(super) fn window_rank(window: gnomcode_core::QuotaWindow) -> u8 {
    match window {
        gnomcode_core::QuotaWindow::FiveHour => 0,
        gnomcode_core::QuotaWindow::Weekly => 1,
        gnomcode_core::QuotaWindow::Monthly => 2,
    }
}

pub(super) async fn providers_overview_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let (quota, today, last_30_days) = tokio::task::spawn_blocking(move || {
        let db = state.db.blocking_lock();
        Ok::<_, gnomcode_store::Error>((
            db.get_latest_quota_snapshots()?,
            db.generate_usage_report_timeline("today", None, None)?,
            db.generate_usage_report_timeline("30d", None, None)?,
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let views = quota_views_at(quota, chrono::Utc::now());
    Ok(Json(provider_overviews(
        views,
        provider_costs(&today, &last_30_days),
        crate::vendor_limits::enabled("claude"),
        crate::vendor_limits::enabled("antigravity"),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use gnomcode_store::Database;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt as _;

    fn overviews(
        snapshots: Vec<gnomcode_core::QuotaSnapshot>,
        now: chrono::DateTime<chrono::Utc>,
        claude_api: bool,
        antigravity_api: bool,
    ) -> Vec<super::ProviderOverview> {
        super::provider_overviews(
            super::quota_views_at(snapshots, now),
            std::collections::HashMap::new(),
            claude_api,
            antigravity_api,
        )
    }

    #[test]
    fn overview_cards_carry_the_source_issue() {
        let quota: gnomcode_core::QuotaSnapshot = serde_json::from_value(json!({
            "provider": "overview-issue-test", "limit_id": "claude", "window": "weekly", "used_pct": 2.0,
            "resets_at": null, "plan": null, "basis": "exact", "observed_at": "2026-11-12T03:37:17Z"
        })).expect("quota");
        crate::vendor_limits::record_issue(
            "overview-issue-test",
            Some("Claude sign-in expired".to_owned()),
        );
        let list = overviews(vec![quota], chrono::Utc::now(), true, false);
        crate::vendor_limits::record_issue("overview-issue-test", None);
        let card = list
            .iter()
            .find(|o| o.provider == "overview-issue-test")
            .expect("card");
        assert_eq!(card.source_issue.as_deref(), Some("Claude sign-in expired"));
        let json = serde_json::to_value(card).expect("json");
        assert_eq!(json["source_issue"], "Claude sign-in expired");
        assert!(
            serde_json::to_value(&list[1])
                .expect("json")
                .get("source_issue")
                .is_none()
        );
    }

    #[test]
    fn overview_lists_core_providers_without_data() {
        let list = overviews(Vec::new(), chrono::Utc::now(), false, false);
        let slugs: Vec<_> = list.iter().map(|o| o.provider.as_str()).collect();
        assert_eq!(slugs, ["claude-code", "codex", "antigravity"]);
        assert!(list.iter().all(|o| o.windows.is_empty()));
        assert!(
            list.iter()
                .all(|o| o.state == gnomcode_core::quota::QuotaState::Unknown)
        );
        assert!(
            list.iter()
                .all(|o| o.cost.is_none() && o.updated_at.is_none())
        );
        assert_eq!(list[0].display_name, "Claude Code");
        assert_eq!(
            list[0].enable_hint.as_deref(),
            Some("Limits are off for Claude Code.")
        );
        assert_eq!(
            list[0].enable_command.as_deref(),
            Some("gnomcode limits enable claude")
        );
        assert_eq!(list[0].source_label, "Off");
        assert_eq!(list[1].enable_command, None);
        assert_eq!(list[1].source_label, "Local logs");
        assert_eq!(
            list[2].enable_command.as_deref(),
            Some("gnomcode limits enable antigravity")
        );
    }

    #[test]
    fn overview_state_is_worst_window() {
        use gnomcode_core::quota::QuotaState;
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let list = overviews(
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
                    90.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        assert_eq!(list[1].state, QuotaState::Low);
        let list = overviews(
            vec![
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::FiveHour,
                    100.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Weekly,
                    90.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        assert_eq!(list[1].state, QuotaState::Exhausted);
        assert_eq!(list[1].updated_at, Some(now));
    }

    #[test]
    fn overview_headline_window_is_tightest_by_severity_then_remaining() {
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let at = |list: &[super::ProviderOverview]| list[1].headline_window;
        // No windows: none.
        assert_eq!(
            overviews(Vec::new(), now, false, false)[1].headline_window,
            None
        );
        // Windows sort session, weekly, monthly. Weekly (index 1) is low; the others are ok.
        let list = overviews(
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
                    90.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Monthly,
                    20.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        assert_eq!(at(&list), Some(1));
        // Two low windows: the one with less left wins.
        let list = overviews(
            vec![
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::FiveHour,
                    85.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Weekly,
                    95.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        assert_eq!(at(&list), Some(1));
        // Exhausted outranks low even when listed later.
        let list = overviews(
            vec![
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::FiveHour,
                    95.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "codex",
                    "codex",
                    QuotaWindow::Weekly,
                    100.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        assert_eq!(at(&list), Some(1));
    }

    #[test]
    fn overview_hint_changes_when_source_enabled() {
        let now = chrono::Utc::now();
        let list = overviews(Vec::new(), now, true, true);
        assert_eq!(
            list[0].enable_hint.as_deref(),
            Some("Waiting for the first Claude reading.")
        );
        assert_eq!(list[0].enable_command, None);
        assert_eq!(list[0].source_label, "Provider API");
        assert_eq!(
            list[2].enable_hint.as_deref(),
            Some("Start an Antigravity agent to read limits.")
        );
        assert_eq!(list[2].enable_command, None);
        assert_eq!(list[2].source_label, "Native limits");
    }

    #[test]
    fn overview_windows_sorted_session_first() {
        use gnomcode_core::{QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let reset = Some(now + chrono::Duration::days(3));
        let list = overviews(
            vec![
                pool(
                    "claude-code",
                    "claude",
                    QuotaWindow::Monthly,
                    1.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "claude-code",
                    "claude",
                    QuotaWindow::Weekly,
                    1.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "claude-code",
                    "claude",
                    QuotaWindow::FiveHour,
                    1.0,
                    reset,
                    TrustLevel::Exact,
                    now,
                ),
                pool(
                    "zed",
                    "zed",
                    QuotaWindow::Weekly,
                    1.0,
                    reset,
                    TrustLevel::Estimated,
                    now,
                ),
            ],
            now,
            false,
            false,
        );
        let labels: Vec<_> = list[0].windows.iter().map(|w| w.label.as_str()).collect();
        assert_eq!(labels, ["Session", "Weekly", "Monthly"]);
        assert_eq!(list[0].source_label, "Status line");
        // Unlisted providers with readings follow the core three, titled from their slug.
        let slugs: Vec<_> = list.iter().map(|o| o.provider.as_str()).collect();
        assert_eq!(slugs, ["claude-code", "codex", "antigravity", "zed"]);
        assert_eq!(list[3].display_name, "Zed");
        assert_eq!(list[3].source_label, "Local logs");
    }

    #[test]
    fn overview_costs_key_on_agent_slug_not_display_label() {
        // `ProviderReport.provider` is a display label ("Claude Code"); `ModelReport.agent`
        // is the slug that matches `QuotaSnapshot.provider`.
        let db = Database::open_in_memory().expect("in-memory db");
        let mut today = db
            .generate_usage_report_timeline("today", None, None)
            .expect("report");
        let mut month = db
            .generate_usage_report_timeline("30d", None, None)
            .expect("report");
        let model = |agent: &str, cost: f64, unpriced: u64| gnomcode_store::ModelReport {
            priced_turns: 1,
            unpriced_turns: unpriced,
            model: "m".into(),
            agent: agent.into(),
            input_tokens: 0,
            cache_read_tokens: 0,
            output_tokens: 0,
            cost_usd: cost,
            cache_savings_usd: 0.0,
            turn_count: 1,
        };
        let provider = |label: &str, models| gnomcode_store::ProviderReport {
            priced_turns: 1,
            unpriced_turns: 0,
            provider: label.into(),
            total_cost_usd: 0.0,
            total_cache_savings_usd: 0.0,
            pct_of_spend: 0.0,
            models,
        };
        today.providers = vec![provider("Claude Code", vec![model("claude-code", 1.5, 0)])];
        month.providers = vec![
            provider("Claude Code", vec![model("claude-code", 12.0, 0)]),
            provider("OpenAI Codex", vec![model("codex", 4.0, 2)]),
            provider("Google Antigravity", vec![model("antigravity", 0.0, 0)]),
        ];
        assert_ne!(month.providers[0].provider, "claude-code");
        let costs = super::provider_costs(&today, &month);
        let claude = costs.get("claude-code").expect("claude cost");
        assert_eq!(claude.today_usd, 1.5);
        assert_eq!(claude.last_30_days_usd, 12.0);
        assert_eq!(claude.basis, gnomcode_core::TrustLevel::Estimated);
        let codex = costs.get("codex").expect("codex cost");
        assert_eq!(codex.today_usd, 0.0);
        assert_eq!(codex.basis, gnomcode_core::TrustLevel::Unknown);
        assert!(!costs.contains_key("antigravity"), "no spend means no cost");
        let list = super::provider_overviews(Vec::new(), costs, false, false);
        assert!(list[0].cost.is_some() && list[1].cost.is_some() && list[2].cost.is_none());
    }

    #[tokio::test]
    async fn providers_overview_requires_authentication() {
        let db = Database::open_in_memory().expect("in-memory db");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "secret-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = Request::builder()
            .uri("/api/v1/providers/overview")
            .header("Host", "127.0.0.1:41738")
            .body(Body::empty())
            .expect("request");
        let response = app.oneshot(request).await.expect("response");
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn multi_pool_provider_is_low_when_capacity_remains_and_exhausted_only_when_all_out() {
        use gnomcode_core::{QuotaSnapshot, QuotaWindow, TrustLevel};
        let now = chrono::Utc::now();
        let make_pool = |pool: &str, used_pct: f64| {
            super::super::quota::quota_views_at(
                vec![QuotaSnapshot {
                    provider: "antigravity".into(),
                    limit_id: format!("antigravity:{pool}"),
                    window: QuotaWindow::FiveHour,
                    used_pct,
                    resets_at: Some(now + chrono::Duration::hours(2)),
                    observed_at: now,
                    basis: TrustLevel::Exact,
                    plan: None,
                }],
                now,
            )
            .into_iter()
            .next()
            .expect("view")
        };

        let mixed = vec![
            make_pool("Claude Models", 100.0),
            make_pool("Gemini Models", 87.0),
        ];
        let mixed_cards =
            super::provider_overviews(mixed, std::collections::HashMap::new(), false, true);
        let ag_mixed = mixed_cards
            .iter()
            .find(|c| c.provider == "antigravity")
            .expect("antigravity");
        assert_eq!(ag_mixed.state, gnomcode_core::quota::QuotaState::Low);
        assert_eq!(ag_mixed.headline_window, Some(1));

        let all_out = vec![
            make_pool("Claude Models", 100.0),
            make_pool("Gemini Models", 100.0),
        ];
        let all_out_cards =
            super::provider_overviews(all_out, std::collections::HashMap::new(), false, true);
        let ag_out = all_out_cards
            .iter()
            .find(|c| c.provider == "antigravity")
            .expect("antigravity");
        assert_eq!(ag_out.state, gnomcode_core::quota::QuotaState::Exhausted);
    }
}
