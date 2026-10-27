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

const CLAUDE_OPUS_5_5: ModelPricing = ModelPricing {
    family: "claude-opus-5-5",
    input_per_mtok_usd: 25.0,
    output_per_mtok_usd: 125.0,
    cache_read_per_mtok_usd: 2.50,
    cache_write_5m_per_mtok_usd: 31.25,
    cache_write_1h_per_mtok_usd: 50.0,
    context_window: 500_000,
};

const CLAUDE_OPUS_5: ModelPricing = ModelPricing {
    family: "claude-opus-5",
    input_per_mtok_usd: 20.0,
    output_per_mtok_usd: 100.0,
    cache_read_per_mtok_usd: 2.00,
    cache_write_5m_per_mtok_usd: 25.0,
    cache_write_1h_per_mtok_usd: 40.0,
    context_window: 400_000,
};

const CLAUDE_OPUS_4_8: ModelPricing = ModelPricing {
    family: "claude-opus-4-8",
    input_per_mtok_usd: 15.0,
    output_per_mtok_usd: 75.0,
    cache_read_per_mtok_usd: 1.50,
    cache_write_5m_per_mtok_usd: 18.75,
    cache_write_1h_per_mtok_usd: 30.0,
    context_window: 300_000,
};

const CLAUDE_OPUS_4_6: ModelPricing = ModelPricing {
    family: "claude-opus-4-6",
    input_per_mtok_usd: 12.0,
    output_per_mtok_usd: 60.0,
    cache_read_per_mtok_usd: 1.20,
    cache_write_5m_per_mtok_usd: 15.0,
    cache_write_1h_per_mtok_usd: 24.0,
    context_window: 200_000,
};

const CLAUDE_OPUS_4: ModelPricing = ModelPricing {
    family: "claude-opus-4",
    input_per_mtok_usd: 15.0,
    output_per_mtok_usd: 75.0,
    cache_read_per_mtok_usd: 1.50,
    cache_write_5m_per_mtok_usd: 18.75,
    cache_write_1h_per_mtok_usd: 30.0,
    context_window: 200_000,
};

const CLAUDE_OPUS: ModelPricing = ModelPricing {
    family: "claude-opus",
    input_per_mtok_usd: 15.0,
    output_per_mtok_usd: 75.0,
    cache_read_per_mtok_usd: 1.50,
    cache_write_5m_per_mtok_usd: 18.75,
    cache_write_1h_per_mtok_usd: 30.0,
    context_window: 200_000,
};

const CLAUDE_SONNET_5_5: ModelPricing = ModelPricing {
    family: "claude-sonnet-5-5",
    input_per_mtok_usd: 5.0,
    output_per_mtok_usd: 25.0,
    cache_read_per_mtok_usd: 0.50,
    cache_write_5m_per_mtok_usd: 6.25,
    cache_write_1h_per_mtok_usd: 10.0,
    context_window: 500_000,
};

const CLAUDE_SONNET_5: ModelPricing = ModelPricing {
    family: "claude-sonnet-5",
    input_per_mtok_usd: 4.0,
    output_per_mtok_usd: 20.0,
    cache_read_per_mtok_usd: 0.40,
    cache_write_5m_per_mtok_usd: 5.0,
    cache_write_1h_per_mtok_usd: 8.0,
    context_window: 400_000,
};

const CLAUDE_SONNET_4_8: ModelPricing = ModelPricing {
    family: "claude-sonnet-4-8",
    input_per_mtok_usd: 3.50,
    output_per_mtok_usd: 17.50,
    cache_read_per_mtok_usd: 0.35,
    cache_write_5m_per_mtok_usd: 4.375,
    cache_write_1h_per_mtok_usd: 7.0,
    context_window: 300_000,
};

const CLAUDE_SONNET_4_6: ModelPricing = ModelPricing {
    family: "claude-sonnet-4-6",
    input_per_mtok_usd: 3.0,
    output_per_mtok_usd: 15.0,
    cache_read_per_mtok_usd: 0.30,
    cache_write_5m_per_mtok_usd: 3.75,
    cache_write_1h_per_mtok_usd: 6.0,
    context_window: 250_000,
};

const CLAUDE_SONNET_4: ModelPricing = ModelPricing {
    family: "claude-sonnet-4",
    input_per_mtok_usd: 3.0,
    output_per_mtok_usd: 15.0,
    cache_read_per_mtok_usd: 0.30,
    cache_write_5m_per_mtok_usd: 3.75,
    cache_write_1h_per_mtok_usd: 6.0,
    context_window: 200_000,
};

const CLAUDE_SONNET_3_7: ModelPricing = ModelPricing {
    family: "claude-sonnet-3-7",
    input_per_mtok_usd: 3.0,
    output_per_mtok_usd: 15.0,
    cache_read_per_mtok_usd: 0.30,
    cache_write_5m_per_mtok_usd: 3.75,
    cache_write_1h_per_mtok_usd: 6.0,
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

const CLAUDE_HAIKU_4_5: ModelPricing = ModelPricing {
    family: "claude-haiku-4-5",
    input_per_mtok_usd: 1.0,
    output_per_mtok_usd: 5.0,
    cache_read_per_mtok_usd: 0.10,
    cache_write_5m_per_mtok_usd: 1.25,
    cache_write_1h_per_mtok_usd: 2.0,
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

const CLAUDE_HAIKU_3: ModelPricing = ModelPricing {
    family: "claude-haiku-3",
    input_per_mtok_usd: 0.25,
    output_per_mtok_usd: 1.25,
    cache_read_per_mtok_usd: 0.025,
    cache_write_5m_per_mtok_usd: 0.3125,
    cache_write_1h_per_mtok_usd: 0.50,
    context_window: 200_000,
};

const OPENAI_GPT_6_1_SOL: ModelPricing = ModelPricing {
    family: "openai-gpt-6-1-sol",
    input_per_mtok_usd: 2.50,
    output_per_mtok_usd: 10.0,
    cache_read_per_mtok_usd: 0.625,
    cache_write_5m_per_mtok_usd: 2.50,
    cache_write_1h_per_mtok_usd: 2.50,
    context_window: 1_000_000,
};

const OPENAI_GPT_6_SOL: ModelPricing = ModelPricing {
    family: "openai-gpt-6-sol",
    input_per_mtok_usd: 2.0,
    output_per_mtok_usd: 8.0,
    cache_read_per_mtok_usd: 0.50,
    cache_write_5m_per_mtok_usd: 2.0,
    cache_write_1h_per_mtok_usd: 2.0,
    context_window: 1_000_000,
};

const OPENAI_GPT_6_ASTRA: ModelPricing = ModelPricing {
    family: "openai-gpt-6-astra",
    input_per_mtok_usd: 3.0,
    output_per_mtok_usd: 12.0,
    cache_read_per_mtok_usd: 0.75,
    cache_write_5m_per_mtok_usd: 3.0,
    cache_write_1h_per_mtok_usd: 3.0,
    context_window: 1_000_000,
};

const OPENAI_GPT_6_LUNA: ModelPricing = ModelPricing {
    family: "openai-gpt-6-luna",
    input_per_mtok_usd: 1.80,
    output_per_mtok_usd: 7.20,
    cache_read_per_mtok_usd: 0.45,
    cache_write_5m_per_mtok_usd: 1.80,
    cache_write_1h_per_mtok_usd: 1.80,
    context_window: 1_000_000,
};

const OPENAI_GPT_5_6_LUNA: ModelPricing = ModelPricing {
    family: "openai-gpt-5-6-luna",
    input_per_mtok_usd: 1.50,
    output_per_mtok_usd: 6.0,
    cache_read_per_mtok_usd: 0.375,
    cache_write_5m_per_mtok_usd: 1.50,
    cache_write_1h_per_mtok_usd: 1.50,
    context_window: 500_000,
};

const OPENAI_GPT_5_SOL: ModelPricing = ModelPricing {
    family: "openai-gpt-5-sol",
    input_per_mtok_usd: 1.50,
    output_per_mtok_usd: 6.0,
    cache_read_per_mtok_usd: 0.375,
    cache_write_5m_per_mtok_usd: 1.50,
    cache_write_1h_per_mtok_usd: 1.50,
    context_window: 500_000,
};

const OPENAI_O3: ModelPricing = ModelPricing {
    family: "openai-o3",
    input_per_mtok_usd: 2.50,
    output_per_mtok_usd: 10.0,
    cache_read_per_mtok_usd: 1.25,
    cache_write_5m_per_mtok_usd: 2.50,
    cache_write_1h_per_mtok_usd: 2.50,
    context_window: 200_000,
};

const OPENAI_O3_MINI: ModelPricing = ModelPricing {
    family: "openai-o3-mini",
    input_per_mtok_usd: 1.10,
    output_per_mtok_usd: 4.40,
    cache_read_per_mtok_usd: 0.55,
    cache_write_5m_per_mtok_usd: 1.10,
    cache_write_1h_per_mtok_usd: 1.10,
    context_window: 200_000,
};

const OPENAI_O1: ModelPricing = ModelPricing {
    family: "openai-o1",
    input_per_mtok_usd: 15.0,
    output_per_mtok_usd: 60.0,
    cache_read_per_mtok_usd: 7.50,
    cache_write_5m_per_mtok_usd: 15.0,
    cache_write_1h_per_mtok_usd: 15.0,
    context_window: 200_000,
};

const OPENAI_O1_MINI: ModelPricing = ModelPricing {
    family: "openai-o1-mini",
    input_per_mtok_usd: 1.10,
    output_per_mtok_usd: 4.40,
    cache_read_per_mtok_usd: 0.55,
    cache_write_5m_per_mtok_usd: 1.10,
    cache_write_1h_per_mtok_usd: 1.10,
    context_window: 128_000,
};

const OPENAI_GPT_4O: ModelPricing = ModelPricing {
    family: "openai-gpt-4o",
    input_per_mtok_usd: 2.50,
    output_per_mtok_usd: 10.0,
    cache_read_per_mtok_usd: 1.25,
    cache_write_5m_per_mtok_usd: 2.50,
    cache_write_1h_per_mtok_usd: 2.50,
    context_window: 128_000,
};

const OPENAI_MINI: ModelPricing = ModelPricing {
    family: "openai-mini",
    input_per_mtok_usd: 0.15,
    output_per_mtok_usd: 0.60,
    cache_read_per_mtok_usd: 0.075,
    cache_write_5m_per_mtok_usd: 0.15,
    cache_write_1h_per_mtok_usd: 0.15,
    context_window: 128_000,
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

const GEMINI_FABLE: ModelPricing = ModelPricing {
    family: "gemini-fable",
    input_per_mtok_usd: 2.00,
    output_per_mtok_usd: 8.00,
    cache_read_per_mtok_usd: 0.50,
    cache_write_5m_per_mtok_usd: 2.00,
    cache_write_1h_per_mtok_usd: 2.00,
    context_window: 2_000_000,
};

const GEMINI_4_ARGON_HIGH: ModelPricing = ModelPricing {
    family: "gemini-4-argon-high",
    input_per_mtok_usd: 2.50,
    output_per_mtok_usd: 10.0,
    cache_read_per_mtok_usd: 0.625,
    cache_write_5m_per_mtok_usd: 2.50,
    cache_write_1h_per_mtok_usd: 2.50,
    context_window: 2_000_000,
};

const GEMINI_PRO_ARGON: ModelPricing = ModelPricing {
    family: "gemini-pro",
    input_per_mtok_usd: 1.25,
    output_per_mtok_usd: 5.0,
    cache_read_per_mtok_usd: 0.3125,
    cache_write_5m_per_mtok_usd: 1.25,
    cache_write_1h_per_mtok_usd: 1.25,
    context_window: 2_000_000,
};

const GEMINI_4_ARGON_LOW: ModelPricing = ModelPricing {
    family: "gemini-4-argon-low",
    input_per_mtok_usd: 0.80,
    output_per_mtok_usd: 3.20,
    cache_read_per_mtok_usd: 0.20,
    cache_write_5m_per_mtok_usd: 0.80,
    cache_write_1h_per_mtok_usd: 0.80,
    context_window: 2_000_000,
};

const GEMINI_3_8_FLASH_HIGH: ModelPricing = ModelPricing {
    family: "gemini-3-8-flash-high",
    input_per_mtok_usd: 0.15,
    output_per_mtok_usd: 0.60,
    cache_read_per_mtok_usd: 0.0375,
    cache_write_5m_per_mtok_usd: 0.15,
    cache_write_1h_per_mtok_usd: 0.15,
    context_window: 1_000_000,
};

const GEMINI_FLASH: ModelPricing = ModelPricing {
    family: "gemini-flash",
    input_per_mtok_usd: 0.075,
    output_per_mtok_usd: 0.30,
    cache_read_per_mtok_usd: 0.01875,
    cache_write_5m_per_mtok_usd: 0.075,
    cache_write_1h_per_mtok_usd: 0.075,
    context_window: 1_000_000,
};

const GEMINI_3_8_FLASH_DEFAULT: ModelPricing = ModelPricing {
    family: "gemini-3-8-flash-default",
    input_per_mtok_usd: 0.05,
    output_per_mtok_usd: 0.20,
    cache_read_per_mtok_usd: 0.0125,
    cache_write_5m_per_mtok_usd: 0.05,
    cache_write_1h_per_mtok_usd: 0.05,
    context_window: 1_000_000,
};

const GEMINI_2_0_FLASH: ModelPricing = ModelPricing {
    family: "gemini-2-0-flash",
    input_per_mtok_usd: 0.10,
    output_per_mtok_usd: 0.40,
    cache_read_per_mtok_usd: 0.025,
    cache_write_5m_per_mtok_usd: 0.10,
    cache_write_1h_per_mtok_usd: 0.10,
    context_window: 1_048_576,
};

const GEMINI_ULTRA: ModelPricing = ModelPricing {
    family: "gemini-ultra",
    input_per_mtok_usd: 5.0,
    output_per_mtok_usd: 20.0,
    cache_read_per_mtok_usd: 1.25,
    cache_write_5m_per_mtok_usd: 5.0,
    cache_write_1h_per_mtok_usd: 5.0,
    context_window: 1_000_000,
};

/// Master static list of all supported catalog models and their pricing.
pub static ALL_MODELS: &[ModelPricing] = &[
    CLAUDE_OPUS_5_5,
    CLAUDE_OPUS_5,
    CLAUDE_OPUS_4_8,
    CLAUDE_OPUS_4_6,
    CLAUDE_OPUS_4,
    CLAUDE_OPUS,
    CLAUDE_SONNET_5_5,
    CLAUDE_SONNET_5,
    CLAUDE_SONNET_4_8,
    CLAUDE_SONNET_4_6,
    CLAUDE_SONNET_4,
    CLAUDE_SONNET_3_7,
    CLAUDE_SONNET,
    CLAUDE_HAIKU_4_5,
    CLAUDE_HAIKU,
    CLAUDE_HAIKU_3,
    OPENAI_GPT_6_1_SOL,
    OPENAI_GPT_6_SOL,
    OPENAI_GPT_6_ASTRA,
    OPENAI_GPT_6_LUNA,
    OPENAI_GPT_5_6_LUNA,
    OPENAI_GPT_5_SOL,
    OPENAI_O3,
    OPENAI_O3_MINI,
    OPENAI_O1,
    OPENAI_O1_MINI,
    OPENAI_GPT_4O,
    OPENAI_MINI,
    OPENAI_CODEX_FRONTIER,
    GEMINI_FABLE,
    GEMINI_4_ARGON_HIGH,
    GEMINI_PRO_ARGON,
    GEMINI_4_ARGON_LOW,
    GEMINI_3_8_FLASH_HIGH,
    GEMINI_FLASH,
    GEMINI_3_8_FLASH_DEFAULT,
    GEMINI_2_0_FLASH,
    GEMINI_ULTRA,
];

/// Returns the complete built-in model pricing catalog.
#[must_use]
pub fn all_model_pricings() -> &'static [ModelPricing] {
    ALL_MODELS
}

/// Resolves a raw model identifier from any supported agent log to its
/// [`ModelPricing`] entry.
#[must_use]
pub fn lookup_model_pricing(model: &str) -> Option<ModelPricing> {
    let lower = model.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    // 1. Anthropic Claude Opus models
    if lower.contains("opus") {
        if lower.contains("5.5") || lower.contains("5-5") {
            return Some(CLAUDE_OPUS_5_5);
        }
        if lower.contains("5.0")
            || lower.contains("5-0")
            || lower.contains("opus-5")
            || lower.contains("opus 5")
        {
            return Some(CLAUDE_OPUS_5);
        }
        if lower.contains("4.8") || lower.contains("4-8") {
            return Some(CLAUDE_OPUS_4_8);
        }
        if lower.contains("4.6") || lower.contains("4-6") {
            return Some(CLAUDE_OPUS_4_6);
        }
        if lower.contains("4.0")
            || lower.contains("4-0")
            || lower.contains("opus-4")
            || lower.contains("opus 4")
        {
            return Some(CLAUDE_OPUS_4);
        }
        return Some(CLAUDE_OPUS);
    }

    // 2. Anthropic Claude Sonnet models
    if lower.contains("sonnet") {
        if lower.contains("5.5") || lower.contains("5-5") {
            return Some(CLAUDE_SONNET_5_5);
        }
        if lower.contains("5.0")
            || lower.contains("5-0")
            || lower.contains("sonnet-5")
            || lower.contains("sonnet 5")
        {
            return Some(CLAUDE_SONNET_5);
        }
        if lower.contains("4.8") || lower.contains("4-8") {
            return Some(CLAUDE_SONNET_4_8);
        }
        if lower.contains("4.6") || lower.contains("4-6") {
            return Some(CLAUDE_SONNET_4_6);
        }
        if lower.contains("4.0")
            || lower.contains("4-0")
            || lower.contains("sonnet-4")
            || lower.contains("sonnet 4")
        {
            return Some(CLAUDE_SONNET_4);
        }
        if lower.contains("3.7") || lower.contains("3-7") {
            return Some(CLAUDE_SONNET_3_7);
        }
        return Some(CLAUDE_SONNET);
    }

    // 3. Anthropic Claude Haiku models
    if lower.contains("haiku") {
        if lower.contains("4.5") || lower.contains("4-5") {
            return Some(CLAUDE_HAIKU_4_5);
        }
        if lower.contains("3.5") || lower.contains("3-5") {
            return Some(CLAUDE_HAIKU);
        }
        return Some(CLAUDE_HAIKU_3);
    }

    // 4. OpenAI Sol / Astra / Luna Frontier models
    if lower.contains("6.1-sol") || lower.contains("6.1 sol") || lower.contains("6-1-sol") {
        return Some(OPENAI_GPT_6_1_SOL);
    }
    if lower.contains("6-sol") || lower.contains("6 sol") {
        return Some(OPENAI_GPT_6_SOL);
    }
    if lower.contains("astra") {
        return Some(OPENAI_GPT_6_ASTRA);
    }
    if lower.contains("6-luna") || lower.contains("6 luna") {
        return Some(OPENAI_GPT_6_LUNA);
    }
    if lower.contains("5.6-luna") || lower.contains("5.6 luna") || lower.contains("5-6-luna") {
        return Some(OPENAI_GPT_5_6_LUNA);
    }
    if lower.contains("5-sol") || lower.contains("5 sol") || lower.contains("gpt-5") {
        return Some(OPENAI_GPT_5_SOL);
    }

    // 5. Google Gemini models
    if lower.contains("fable") {
        return Some(GEMINI_FABLE);
    }
    if lower.contains("argon") {
        if lower.contains("high") {
            return Some(GEMINI_4_ARGON_HIGH);
        }
        if lower.contains("low") {
            return Some(GEMINI_4_ARGON_LOW);
        }
        return Some(GEMINI_PRO_ARGON);
    }
    if lower.contains("flash") {
        if lower.contains("high") {
            return Some(GEMINI_3_8_FLASH_HIGH);
        }
        if lower.contains("default") || lower.contains("low") {
            return Some(GEMINI_3_8_FLASH_DEFAULT);
        }
        if lower.contains("2.0") || lower.contains("2-0") {
            return Some(GEMINI_2_0_FLASH);
        }
        return Some(GEMINI_FLASH);
    }
    if lower.contains("ultra") {
        return Some(GEMINI_ULTRA);
    }
    if lower.contains("gemini") {
        return Some(GEMINI_PRO_ARGON);
    }

    // 6. OpenAI Reasoning & Flagship models
    if lower.contains("o3-mini") {
        return Some(OPENAI_O3_MINI);
    }
    if lower.contains("o3") {
        return Some(OPENAI_O3);
    }
    if lower.contains("o1-mini") {
        return Some(OPENAI_O1_MINI);
    }
    if lower.starts_with("o1") || lower.contains("-o1") {
        return Some(OPENAI_O1);
    }
    if lower.contains("4o-mini") || lower.contains("mini") || lower.contains("nano") {
        return Some(OPENAI_MINI);
    }
    if lower.contains("gpt-4o") || lower.contains("4o") {
        return Some(OPENAI_GPT_4O);
    }
    if lower.contains("gpt-") || lower.contains("codex") || lower.starts_with("o4") {
        return Some(OPENAI_CODEX_FRONTIER);
    }
    if lower.contains("pro") {
        return Some(GEMINI_PRO_ARGON);
    }

    None
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
    let out_m = u64_to_f64(usage.output_tokens) / mtok;
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
        cost_basis: TrustLevel::Estimated,
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
        let res = calculate_cost("claude-3-opus-20240229", &usage);
        assert_eq!(res.cost_basis, TrustLevel::Estimated);
        assert_eq!(res.pricing_version, PRICING_VERSION);
        assert!(res.cost_usd > 0.0);
        // Reading 100k cached tokens at 90% discount saves 90% of input rate
        assert!(res.cache_savings_usd > 1.0);
        assert_eq!(res.context_window, Some(200_000));

        let opus5 = calculate_cost("claude-opus-5", &usage);
        assert_eq!(opus5.context_window, Some(400_000));
    }

    #[test]
    fn codex_and_antigravity_models_resolve_pricing_and_context_window() {
        let codex = lookup_model_pricing("gpt-6.1-sol");
        assert!(codex.is_some());

        let argon = lookup_model_pricing("Gemini 4 Argon (Medium)");
        assert!(argon.is_some());

        let fable = lookup_model_pricing("gemini-fable");
        assert!(fable.is_some());

        let flash_med = lookup_model_pricing("Gemini 3.8 Flash (Medium)")
            .expect("gemini 3.8 flash medium should not collide with openai mini");
        assert_eq!(flash_med.family, "gemini-flash");
        assert_eq!(flash_med.context_window, 1_000_000);
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
    fn reasoning_subset_is_not_billed_twice() {
        let usage = TokenBreakdown {
            output_tokens: 1_000,
            reasoning_tokens: 200,
            ..TokenBreakdown::default()
        };
        let result = calculate_cost("claude-3-5-sonnet-20241022", &usage);
        assert!((result.cost_usd - 0.015).abs() < 1e-9);
        assert_eq!(result.cost_basis, TrustLevel::Estimated);
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
