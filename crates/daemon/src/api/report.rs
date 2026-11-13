//! Reports, heatmap, stats and export handlers.

use super::*;

/// Query parameters for reports.
#[derive(Debug, Deserialize, Default)]
pub struct ReportQueryParams {
    /// Range filter (`today`, `7d`, `30d`, `all`, `custom`).
    pub range: Option<String>,
    /// Custom ISO start timestamp.
    pub from: Option<String>,
    /// Custom ISO end timestamp.
    pub to: Option<String>,
    /// Output format (`json` or `csv`).
    pub format: Option<String>,
}

pub(super) async fn export_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let range = params.range.as_deref().unwrap_or("30d");
    if !matches!(range, "today" | "7d" | "30d" | "all") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let db = state.db.lock().await;
    let report = db
        .generate_usage_report_timeline(range, params.from.as_deref(), params.to.as_deref())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if params.format.as_deref() == Some("csv") {
        let mut csv = String::from(
            "agent,model,turns,input_tokens,output_tokens,cache_read_tokens,cost_usd,cache_savings_usd\n",
        );
        for prov in &report.providers {
            for m in &prov.models {
                use std::fmt::Write as _;
                let _ = writeln!(
                    csv,
                    "{},{},{},{},{},{},{:.4},{:.4}",
                    prov.provider,
                    m.model,
                    m.turn_count,
                    m.input_tokens,
                    m.output_tokens,
                    m.cache_read_tokens,
                    m.cost_usd,
                    m.cache_savings_usd
                );
            }
        }
        Ok((
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=gnomcode-report.csv",
                ),
            ],
            csv,
        )
            .into_response())
    } else {
        Ok((
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=gnomcode-report.json",
                ),
            ],
            Json(report),
        )
            .into_response())
    }
}

pub(super) async fn report_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<ReportQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let range = params.range.as_deref().unwrap_or("all");
    let query_range = range.to_owned();
    let from = params.from.clone();
    let to = params.to.clone();
    let (report, user_profile) = tokio::task::spawn_blocking(move || {
        let db = state.db.blocking_lock();
        Ok::<_, gnomcode_store::Error>((
            db.generate_usage_report_timeline(&query_range, from.as_deref(), to.as_deref())?,
            db.get_user_profile()?,
        ))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let roi = report_roi(&report, &user_profile, range);
    Ok(Json(
        json!({"timeline":report,"user_profile":user_profile,"roi_by_provider":roi["roi_by_provider"],"combined_roi":roi["combined_roi"]}),
    ))
}

pub(super) async fn activity_heatmap_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let heatmap = tokio::task::spawn_blocking(move || {
        let db = state.db.blocking_lock();
        db.activity_heatmap(chrono::Utc::now().date_naive())
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(heatmap))
}

pub(crate) fn report_roi(
    report: &gnomcode_store::TimelineReport,
    user_profile: &UserProfile,
    range: &str,
) -> Value {
    #[derive(Serialize)]
    struct PlanRoiInfo {
        provider: String,
        plan_usd: f64,
        monthly_plan_usd: f64,
        spend_usd: f64,
        multiplier: f64,
        net_savings_usd: f64,
        period_label: String,
    }

    let (time_factor, period_label): (f64, &str) = match range {
        "today" | "24h" => (1.0 / 30.0, "day"),
        "7d" => (7.0 / 30.0, "7d"),
        "30d" => (1.0, "mo"),
        _ => {
            if let (Some(early_s), Some(late_s)) = (&report.earliest_at, &report.latest_at) {
                if let (Ok(e_dt), Ok(l_dt)) = (
                    chrono::DateTime::parse_from_rfc3339(early_s),
                    chrono::DateTime::parse_from_rfc3339(late_s),
                ) {
                    let days = (l_dt - e_dt).num_days().max(1) as f64;
                    (days / 30.0, "period")
                } else {
                    (1.0, "period")
                }
            } else {
                (1.0, "period")
            }
        }
    };

    let mut roi_by_provider = Vec::new();
    let mut combined_plan = 0.0;

    let get_spend = |prov_names: &[&str]| -> f64 {
        report
            .providers
            .iter()
            .find(|p| {
                prov_names
                    .iter()
                    .any(|name| p.provider.eq_ignore_ascii_case(name))
            })
            .map_or(0.0, |p| p.total_cost_usd)
    };

    let claude_spend = get_spend(&["Claude Code", "Anthropic", "Anthropic (Claude)"]);
    let openai_spend = get_spend(&["OpenAI Codex", "OpenAI", "OpenAI (Codex)"]);
    let google_spend = get_spend(&["Google Antigravity", "Google", "Google (Antigravity)"]);

    if user_profile.claude_plan_usd > 0.0 && claude_spend > 0.0 {
        let eff_plan = user_profile.claude_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(claude_spend, eff_plan);
        roi_by_provider.push(PlanRoiInfo {
            provider: "Claude Code".to_owned(),
            plan_usd: eff_plan,
            monthly_plan_usd: user_profile.claude_plan_usd,
            spend_usd: claude_spend,
            multiplier: roi.multiplier,
            net_savings_usd: roi.net_savings_usd,
            period_label: period_label.to_owned(),
        });
    }

    if user_profile.openai_plan_usd > 0.0 && openai_spend > 0.0 {
        let eff_plan = user_profile.openai_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(openai_spend, eff_plan);
        roi_by_provider.push(PlanRoiInfo {
            provider: "OpenAI Codex".to_owned(),
            plan_usd: eff_plan,
            monthly_plan_usd: user_profile.openai_plan_usd,
            spend_usd: openai_spend,
            multiplier: roi.multiplier,
            net_savings_usd: roi.net_savings_usd,
            period_label: period_label.to_owned(),
        });
    }

    if user_profile.google_plan_usd > 0.0 && google_spend > 0.0 {
        let eff_plan = user_profile.google_plan_usd * time_factor;
        combined_plan += eff_plan;
        let roi = gnomcode_core::pricing::calculate_plan_roi(google_spend, eff_plan);
        roi_by_provider.push(PlanRoiInfo {
            provider: "Google Antigravity".to_owned(),
            plan_usd: eff_plan,
            monthly_plan_usd: user_profile.google_plan_usd,
            spend_usd: google_spend,
            multiplier: roi.multiplier,
            net_savings_usd: roi.net_savings_usd,
            period_label: period_label.to_owned(),
        });
    }

    let combined_roi_calc =
        gnomcode_core::pricing::calculate_plan_roi(report.total_cost_usd, combined_plan);
    let combined_roi = PlanRoiInfo {
        provider: "All Plans (Combined)".to_owned(),
        plan_usd: combined_plan,
        monthly_plan_usd: user_profile.claude_plan_usd
            + user_profile.openai_plan_usd
            + user_profile.google_plan_usd,
        spend_usd: report.total_cost_usd,
        multiplier: combined_roi_calc.multiplier,
        net_savings_usd: combined_roi_calc.net_savings_usd,
        period_label: period_label.to_owned(),
    };

    json!({"roi_by_provider":roi_by_provider,"combined_roi":combined_roi})
}

pub(super) async fn stats_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let stats = db
        .get_database_stats()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use gnomcode_store::Database;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn activity_heatmap_requires_auth_and_returns_dense_days() {
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let request = |auth: &str| {
            Request::builder()
                .uri("/api/v1/activity/heatmap")
                .header("host", "127.0.0.1:41738")
                .header("authorization", auth)
                .body(Body::empty())
                .expect("request")
        };
        assert_eq!(
            app.clone()
                .oneshot(request("Bearer wrong"))
                .await
                .expect("response")
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let response = app
            .oneshot(request("Bearer local-test-token"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("body");
        let heatmap: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        let days = heatmap["days"].as_array().expect("days");
        assert!((365..=371).contains(&days.len()));
        assert!(heatmap["first_recorded"].is_null());
        assert_eq!(heatmap["stats"]["active_days"], 0);
    }
}
