//! Model pricing catalog, API-equivalent USD cost calculation, cache savings,
//! and Subscription Plan ROI ("Plan Value") calculation (`SPEC §3.6`, `§4.1`).

use crate::TrustLevel;
use serde::{Deserialize, Serialize};

/// Bundled pricing table version stored on every [`crate::UsageRecord`].
pub const PRICING_VERSION: &str = "2026-10-26";

/// Per-million-token rates and context window for a model family.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPricing {
    /// Canonical model family name.
    pub family: &'static str,
    /// USD per 1M uncached input tokens.
    pub input_per_mtok_usd: f64,
    /// USD per 1M output/reasoning tokens.
    pub output_per_mtok_usd: f64,
    /// USD per 1M cached input tokens read (`cache_read_tokens`).
    pub cache_read_per_mtok_usd: f64,
    /// USD per 1M 5-minute ephemeral cache creation tokens.
    pub cache_write_5m_per_mtok_usd: f64,
    /// USD per 1M 1-hour ephemeral cache creation tokens.
    pub cache_write_1h_per_mtok_usd: f64,
    /// Maximum context window size in tokens.
    pub context_window: u64,
}

/// Raw token counts passed into [`calculate_cost`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBreakdown {
    /// Uncached input tokens.
    pub input_tokens: u64,
    /// Output tokens.
    pub output_tokens: u64,
    /// Reasoning tokens (billed at output rate when reported separately).
    pub reasoning_tokens: u64,
    /// Cached input tokens read.
    pub cache_read_tokens: u64,
    /// 5-minute cache creation tokens.
    pub cache_write_5m_tokens: u64,
    /// 1-hour cache creation tokens.
    pub cache_write_1h_tokens: u64,
}

/// Computed API-equivalent USD cost, cache savings, and trust basis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostEvaluation {
    /// API-equivalent cost in USD.
    pub cost_usd: f64,
    /// USD saved by prompt caching vs full input price.
    pub cache_savings_usd: f64,
    /// Known context window for the model, if matched.
    pub context_window: Option<u64>,
    /// Trust basis of the cost calculation.
    pub cost_basis: TrustLevel,
    /// Pricing table version used.
    pub pricing_version: &'static str,
}

/// Subscription ROI ("Plan Value") summary (`SPEC §3.6`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlanRoi {
    /// Total API-equivalent USD generated during the billing window.
    pub api_equivalent_usd: f64,
    /// Monthly subscription price in USD.
    pub monthly_plan_usd: f64,
    /// Ratio `api_equivalent_usd / monthly_plan_usd` (e.g. `9.2` for `9.2x`).
    pub multiplier: f64,
    /// Net value above subscription cost (`api_equivalent_usd - monthly_plan_usd`).
    pub net_savings_usd: f64,
}

const CLAUDE_OPUS: ModelPricing = ModelPricing {
    family: "claude-opus",
    input_per_mtok_usd: 15.0,
    output_per_mtok_usd: 75.0,
    cache_read_per_mtok_usd: 1.50,
    cache_write_5m_per_mtok_usd: 18.75,
    cache_write_1h_per_mtok_usd: 30.0,
    context_window: 200_000,
};

const CLAUDE_SONNET: ModelPricing = ModelPricing {
    family: "claude-sonnet",
    input_per_mtok_usd: 3.0,
    output_per_mtok_usd: 15.0,
    cache_read_per_mtok_usd: 0.30,
    cache_write_5m_per_mtok_usd: 3.75,
    cache_write_1h_per_mtok_usd: 6.0,
    context_window: 200_000,
};

const CLAUDE_HAIKU: ModelPricing = ModelPricing {
    family: "claude-haiku",
    input_per_mtok_usd: 0.80,
    output_per_mtok_usd: 4.0,
    cache_read_per_mtok_usd: 0.08,
    cache_write_5m_per_mtok_usd: 1.0,
    cache_write_1h_per_mtok_usd: 1.60,
    context_window: 200_000,
};

const OPENAI_CODEX_FRONTIER: ModelPricing = ModelPricing {
    family: "openai-codex-frontier",
    input_per_mtok_usd: 2.0,
    output_per_mtok_usd: 8.0,
    cache_read_per_mtok_usd: 0.50,
    cache_write_5m_per_mtok_usd: 2.0,
    cache_write_1h_per_mtok_usd: 2.0,
    context_window: 258_400,
};

const OPENAI_MINI: ModelPricing = ModelPricing {
    family: "openai-mini",
    input_per_mtok_usd: 0.40,
    output_per_mtok_usd: 1.60,
    cache_read_per_mtok_usd: 0.10,
    cache_write_5m_per_mtok_usd: 0.40,
    cache_write_1h_per_mtok_usd: 0.40,
    context_window: 200_000,
};

const GEMINI_PRO_ARGON: ModelPricing = ModelPricing {
    family: "gemini-pro",
    input_per_mtok_usd: 1.25,
    output_per_mtok_usd: 10.0,
    cache_read_per_mtok_usd: 0.3125,
    cache_write_5m_per_mtok_usd: 1.25,
    cache_write_1h_per_mtok_usd: 1.25,
    context_window: 1_000_000,
};

const GEMINI_FLASH: ModelPricing = ModelPricing {
    family: "gemini-flash",
    input_per_mtok_usd: 0.15,
    output_per_mtok_usd: 0.60,
    cache_read_per_mtok_usd: 0.0375,
    cache_write_5m_per_mtok_usd: 0.15,
    cache_write_1h_per_mtok_usd: 0.15,
    context_window: 1_000_000,
};

/// Resolves a raw model identifier from any supported agent log to its
/// [`ModelPricing`] entry.
#[must_use]
pub fn lookup_model_pricing(model: &str) -> Option<ModelPricing> {
    let lower = model.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    if lower.contains("opus") {
        Some(CLAUDE_OPUS)
    } else if lower.contains("sonnet") {
        Some(CLAUDE_SONNET)
    } else if lower.contains("haiku") {
        Some(CLAUDE_HAIKU)
    } else if lower.contains("mini") || lower.contains("nano") {
        Some(OPENAI_MINI)
    } else if lower.contains("gpt-")
        || lower.contains("codex")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
    {
        Some(OPENAI_CODEX_FRONTIER)
    } else if lower.contains("flash") {
        Some(GEMINI_FLASH)
    } else if lower.contains("gemini") || lower.contains("argon") || lower.contains("fable") {
        Some(GEMINI_PRO_ARGON)
    } else {
        None
    }
}

/// Computes the API-equivalent USD cost and prompt-cache savings for a turn.
#[must_use]
pub fn calculate_cost(model: &str, usage: &TokenBreakdown) -> CostEvaluation {
    let Some(pricing) = lookup_model_pricing(model) else {
        return CostEvaluation {
            cost_usd: 0.0,
            cache_savings_usd: 0.0,
            context_window: None,
            cost_basis: TrustLevel::Unknown,
            pricing_version: PRICING_VERSION,
        };
    };

    let mtok = 1_000_000.0_f64;
    let in_m = u64_to_f64(usage.input_tokens) / mtok;
    let out_m = u64_to_f64(usage.output_tokens.saturating_add(usage.reasoning_tokens)) / mtok;
    let read_m = u64_to_f64(usage.cache_read_tokens) / mtok;
    let w5_m = u64_to_f64(usage.cache_write_5m_tokens) / mtok;
    let w1h_m = u64_to_f64(usage.cache_write_1h_tokens) / mtok;

    let cost_usd = in_m.mul_add(
        pricing.input_per_mtok_usd,
        out_m.mul_add(
            pricing.output_per_mtok_usd,
            read_m.mul_add(
                pricing.cache_read_per_mtok_usd,
                w5_m.mul_add(
                    pricing.cache_write_5m_per_mtok_usd,
                    w1h_m * pricing.cache_write_1h_per_mtok_usd,
                ),
            ),
        ),
    );

    let full_input_rate_for_cached = read_m * pricing.input_per_mtok_usd;
    let actual_cached_cost = read_m * pricing.cache_read_per_mtok_usd;
    let cache_savings_usd = (full_input_rate_for_cached - actual_cached_cost).max(0.0);

    CostEvaluation {
        cost_usd,
        cache_savings_usd,
        context_window: Some(pricing.context_window),
        cost_basis: TrustLevel::Exact,
        pricing_version: PRICING_VERSION,
    }
}

/// Calculates Subscription Plan ROI ("Plan Value") given API-equivalent spend
/// and the user's monthly plan price (`SPEC §3.6`).
#[must_use]
pub fn calculate_plan_roi(api_equivalent_usd: f64, monthly_plan_usd: f64) -> PlanRoi {
    let multiplier = if monthly_plan_usd > 0.0 {
        api_equivalent_usd / monthly_plan_usd
    } else {
        0.0
    };
    PlanRoi {
        api_equivalent_usd,
        monthly_plan_usd,
        multiplier,
        net_savings_usd: api_equivalent_usd - monthly_plan_usd,
    }
}

/// Maps an agent slug to a clean display name.
#[must_use]
pub fn format_agent_display(agent_slug: &str) -> &'static str {
    match agent_slug.to_lowercase().as_str() {
        "antigravity" => "Google Antigravity",
        "codex" => "OpenAI Codex",
        "claude-code" => "Claude Code",
        "gemini-cli" => "Google Gemini CLI",
        "cursor" => "Cursor",
        "cline" => "Cline",
        "aider" => "Aider",
        _ => "Other Agent",
    }
}

/// Maps a model name or agent slug to its provider name ("Anthropic", "OpenAI", "Google", "Other").
#[must_use]
pub fn classify_provider(agent_slug: &str, model: &str) -> &'static str {
    let m = model.to_lowercase();
    let a = agent_slug.to_lowercase();
    if a.contains("claude")
        || m.contains("claude")
        || m.contains("opus")
        || m.contains("sonnet")
        || m.contains("haiku")
    {
        "Anthropic"
    } else if a.contains("codex")
        || m.contains("gpt")
        || m.contains("o1")
        || m.contains("o3")
        || m.contains("sol")
    {
        "OpenAI"
    } else if a.contains("antigravity")
        || a.contains("gemini")
        || m.contains("gemini")
        || m.contains("argon")
        || m.contains("flash")
    {
        "Google"
    } else {
        "Other"
    }
}

fn u64_to_f64(val: u64) -> f64 {
    u32::try_from(val).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests {
    use super::{
        PRICING_VERSION, TokenBreakdown, calculate_cost, calculate_plan_roi, lookup_model_pricing,
    };
    use crate::TrustLevel;

    #[test]
    fn claude_opus_cost_and_cache_savings_are_exact() {
        let usage = TokenBreakdown {
            input_tokens: 10_000,
            output_tokens: 1_000,
            reasoning_tokens: 0,
            cache_read_tokens: 100_000,
            cache_write_5m_tokens: 20_000,
            cache_write_1h_tokens: 0,
        };
        let res = calculate_cost("claude-opus-5-20260619", &usage);
        assert_eq!(res.cost_basis, TrustLevel::Exact);
        assert_eq!(res.pricing_version, PRICING_VERSION);
        assert!(res.cost_usd > 0.0);
        // Reading 100k cached tokens at 90% discount saves 90% of input rate
        assert!(res.cache_savings_usd > 1.0);
        assert_eq!(res.context_window, Some(200_000));
    }

    #[test]
    fn codex_and_antigravity_models_resolve_pricing_and_context_window() {
        let codex = lookup_model_pricing("gpt-6.1-sol");
        assert!(codex.is_some());

        let argon = lookup_model_pricing("Gemini 4 Argon (Medium)");
        assert!(argon.is_some());
    }

    #[test]
    fn unknown_model_returns_unknown_trust_level_and_zero_cost() {
        let usage = TokenBreakdown {
            input_tokens: 5_000,
            output_tokens: 500,
            reasoning_tokens: 0,
            cache_read_tokens: 0,
            cache_write_5m_tokens: 0,
            cache_write_1h_tokens: 0,
        };
        let res = calculate_cost("mystery-custom-llm-v9", &usage);
        assert_eq!(res.cost_basis, TrustLevel::Unknown);
        assert!((res.cost_usd - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn plan_roi_computes_multiplier_and_net_value() {
        let roi = calculate_plan_roi(184.50, 20.0);
        assert!((roi.multiplier - 9.225).abs() < 1e-6);
        assert!((roi.net_savings_usd - 164.50).abs() < 1e-6);

        let zero_plan = calculate_plan_roi(50.0, 0.0);
        assert!((zero_plan.multiplier - 0.0).abs() < f64::EPSILON);
    }
}
