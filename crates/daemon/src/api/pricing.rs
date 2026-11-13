//! Custom pricing and pricing refresh handlers.

use super::*;

pub(super) async fn custom_pricing_get_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let pricings = db
        .get_custom_model_pricings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "custom_pricing": pricings })))
}

pub(super) async fn custom_pricing_post_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Json(pricing): Json<gnomcode_store::CustomModelPricing>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let mut db = state.db.lock().await;
    db.set_custom_model_pricing(&pricing)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pricings = db
        .get_custom_model_pricings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "custom_pricing": pricings })))
}

#[derive(serde::Deserialize)]
pub(super) struct DeletePricingQuery {
    pattern: String,
}

pub(super) async fn custom_pricing_delete_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(query): Query<DeletePricingQuery>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let mut db = state.db.lock().await;
    db.delete_custom_model_pricing(&query.pattern)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let pricings = db
        .get_custom_model_pricings()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "custom_pricing": pricings })))
}

pub(super) async fn pricing_models_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let models = gnomcode_core::pricing::all_model_pricings();
    let db = state.db.lock().await;
    let custom = db.get_custom_model_pricings().unwrap_or_default();

    Ok(Json(json!({
        "version": gnomcode_core::pricing::PRICING_VERSION,
        "source": "Official Provider Documentation & BerriAI/LiteLLM Open Source Pricing Registry",
        "source_url": "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json",
        "last_updated": "2026-10-26",
        "models": models,
        "custom_pricing": custom,
    })))
}

pub(super) async fn pricing_refresh_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let url = "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
    let Ok(resp) = client.get(url).send().await else {
        return Ok(Json(json!({
            "ok": false,
            "message": "Failed to reach remote LiteLLM pricing registry; using built-in catalog",
            "models_updated": 0,
        })));
    };

    let Ok(map) = resp.json::<serde_json::Value>().await else {
        return Ok(Json(json!({
            "ok": false,
            "message": "Malformed remote pricing JSON",
            "models_updated": 0,
        })));
    };

    let mut updated = 0;
    let mut db = state.db.lock().await;

    if let Some(obj) = map.as_object() {
        for (model_name, info) in obj {
            let in_cost = info.get("input_cost_per_token").and_then(|v| v.as_f64());
            let out_cost = info.get("output_cost_per_token").and_then(|v| v.as_f64());
            let cache_cost = info
                .get("cache_read_input_token_cost")
                .and_then(|v| v.as_f64());

            if let (Some(ic), Some(oc)) = (in_cost, out_cost) {
                // Convert per-token to per-1M-tokens
                let in_mtok = ic * 1_000_000.0;
                let out_mtok = oc * 1_000_000.0;
                let cache_read_mtok = cache_cost.map_or(in_mtok * 0.1, |c| c * 1_000_000.0);

                let custom = gnomcode_store::CustomModelPricing {
                    model_pattern: model_name.clone(),
                    input_per_mtok_usd: in_mtok,
                    output_per_mtok_usd: out_mtok,
                    cache_read_per_mtok_usd: cache_read_mtok,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                let _ = db.set_custom_model_pricing(&custom);
                updated += 1;
            }
        }
    }

    Ok(Json(json!({
        "ok": true,
        "message": format!("Successfully synchronized {updated} model rates from LiteLLM registry"),
        "models_updated": updated,
        "source": "BerriAI/LiteLLM Open Source Pricing Registry",
    })))
}
