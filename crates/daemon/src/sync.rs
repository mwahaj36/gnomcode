//! Supabase cloud synchronization client structures and rollup hashing (`SPEC §3.15`).
//!
//! Provides:
//! - Device registration payload and token hash verification.
//! - Hourly usage rollup batching (`usage_hourly_sync`).
//! - Cross-device quota pool sync (`quota_pool_sync`).
//! - HMAC-SHA256 privacy-preserving project key generation (`SPEC §3.15.D`).

use chrono::{DateTime, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Synchronized hourly usage record for multi-device mirroring and team rollups.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HourlyUsageSync {
    /// Associated organization ID.
    pub org_id: String,
    /// Associated user ID.
    pub user_id: String,
    /// Originating device ID.
    pub device_id: String,
    /// UTC hourly bucket timestamp (e.g. `2026-10-26T15:00:00Z`).
    pub hour_bucket: DateTime<Utc>,
    /// Privacy-preserving HMAC-SHA256 hash or plaintext project key.
    pub project_key: String,
    /// Optional plaintext project name (omitted unless user opted in).
    pub project_label: Option<String>,
    /// Agent identifier.
    pub agent: String,
    /// Model identifier.
    pub model: String,
    /// Total input tokens during this hour bucket.
    pub input_tokens: u64,
    /// Total output tokens during this hour bucket.
    pub output_tokens: u64,
    /// Total cached input tokens read.
    pub cache_read_tokens: u64,
    /// Total cache write tokens.
    pub cache_write_tokens: u64,
    /// Total API-equivalent cost in USD.
    pub cost_usd: f64,
    /// Cost basis (`exact` or `estimated`).
    pub cost_basis: String,
    /// Number of turns executed.
    pub turn_count: u32,
}

/// Computes the privacy-preserving HMAC-SHA256 project key (`SPEC §3.15.D`).
///
/// Ensures repository names and folder structures never leak to remote servers
/// unless the user or team explicitly enables `share_project_names`.
#[must_use]
pub fn hash_project_key(org_salt: &[u8], canonical_project_name: &str) -> String {
    let Ok(mut mac) = HmacSha256::new_from_slice(org_salt) else {
        return "proj_unhashed".to_owned();
    };
    mac.update(canonical_project_name.as_bytes());
    let res = mac.finalize().into_bytes();
    let mut hex = String::with_capacity(32);
    for b in &res[..16] {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    format!("proj_{hex}")
}

#[cfg(test)]
mod tests {
    use super::hash_project_key;

    #[test]
    fn project_key_hashing_is_deterministic_and_prefixes_proj() {
        let salt = b"fleethud-test-org-salt-123456";
        let h1 = hash_project_key(salt, "github.com/mwahaj36/fleethud");
        let h2 = hash_project_key(salt, "github.com/mwahaj36/fleethud");
        let h3 = hash_project_key(salt, "github.com/other/project");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
        assert!(h1.starts_with("proj_"));
        assert_eq!(h1.len(), 5 + 32);
    }
}
