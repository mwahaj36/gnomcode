//! Ed25519 offline license verification and tier capability enforcement (`SPEC §3.1`, `§6.8`).
//!
//! Licenses are offline cryptographic files (`~/.fleethud/license.key`).
//! The payload contains:
//! - `licensee`: Name or email of the purchaser.
//! - `tier`: `"pro"` or `"team"`.
//! - `issued_at`: ISO-8601 timestamp.
//! - `expires_at`: Optional expiration timestamp (`None` for lifetime).
//! - `signature`: Base64-encoded Ed25519 signature over canonical payload bytes.

use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Hardcoded public key for verifying FleetHUD Pro and Team license files.
/// In release binaries, the key is compiled directly into the binary.
pub const FLEETHUD_LICENSE_PUBKEY_HEX: &str =
    "c8a5542a1226a7988ee0fcfe7dfa2c91838883ff0c96c567e997a66b9d628dd5";

/// Licensing subsystem error type.
#[derive(Debug, Error)]
pub enum LicenseError {
    /// License file cannot be parsed or decoded.
    #[error("malformed license file: {0}")]
    Malformed(String),
    /// Cryptographic signature check failed.
    #[error("invalid license signature")]
    InvalidSignature,
    /// License has expired.
    #[error("license has expired")]
    Expired,
}

/// Product feature tier unlocked by a license.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Free tier (CLI, daemon, web dashboard, tray, local state).
    Free,
    /// Pro tier ($19 one-time: guardrails, Resume Queue, custom alert rules, export).
    Pro,
    /// Teams tier ($12/seat/mo: Supabase shared rollups, org budgets, Slack alerts).
    Team,
}

/// Unsigned license body fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicensePayload {
    /// Purchaser name or email.
    pub licensee: String,
    /// Licensed tier.
    pub tier: Tier,
    /// ISO-8601 issuance timestamp.
    pub issued_at: String,
    /// Optional expiration timestamp (`None` for perpetual).
    pub expires_at: Option<String>,
}

/// Signed license file structure persisted in `~/.fleethud/license.key`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseFile {
    /// License claims payload.
    pub payload: LicensePayload,
    /// Hex-encoded Ed25519 signature over serialized payload bytes.
    pub signature_hex: String,
}

/// Verifies a [`LicenseFile`] against an Ed25519 public key.
///
/// # Errors
/// Returns [`LicenseError`] if the signature does not match or public key is malformed.
pub fn verify_license(license: &LicenseFile, pubkey_hex: &str) -> Result<Tier, LicenseError> {
    let pubkey_bytes = hex_decode(pubkey_hex)
        .map_err(|e| LicenseError::Malformed(format!("invalid pubkey: {e}")))?;

    if pubkey_bytes.len() != 32 {
        return Err(LicenseError::Malformed(
            "public key must be 32 bytes".to_owned(),
        ));
    }

    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&pubkey_bytes);

    let verifying_key =
        VerifyingKey::from_bytes(&key_arr).map_err(|e| LicenseError::Malformed(e.to_string()))?;

    let canonical_bytes =
        serde_json::to_vec(&license.payload).map_err(|e| LicenseError::Malformed(e.to_string()))?;

    let sig_bytes = hex_decode(&license.signature_hex)
        .map_err(|e| LicenseError::Malformed(format!("invalid signature hex: {e}")))?;

    if sig_bytes.len() != 64 {
        return Err(LicenseError::InvalidSignature);
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    verifying_key
        .verify(&canonical_bytes, &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;

    if let Some(expires_at) = &license.payload.expires_at {
        let expiration = chrono::DateTime::parse_from_rfc3339(expires_at)
            .map_err(|e| LicenseError::Malformed(e.to_string()))?;
        if expiration <= chrono::Utc::now() {
            return Err(LicenseError::Expired);
        }
    }

    Ok(license.payload.tier)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("odd hex string length".to_owned());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LicenseError, LicenseFile, LicensePayload, Tier, verify_license};
    use ed25519_dalek::{Signer as _, SigningKey};

    #[test]
    fn valid_license_verifies_cleanly() {
        // Generate a test keypair
        let seed = [0x42u8; 32];
        let signing_key = SigningKey::from_bytes(&seed);
        let verifying_key = signing_key.verifying_key();
        let pubkey_hex: String = verifying_key
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let payload = LicensePayload {
            licensee: "developer@example.com".to_owned(),
            tier: Tier::Pro,
            issued_at: "2026-10-26T00:00:00Z".to_owned(),
            expires_at: None,
        };

        let canonical_bytes = serde_json::to_vec(&payload).expect("serialize payload");
        let sig = signing_key.sign(&canonical_bytes);
        let sig_hex: String = sig.to_bytes().iter().map(|b| format!("{b:02x}")).collect();

        let license = LicenseFile {
            payload,
            signature_hex: sig_hex,
        };

        let verified_tier = verify_license(&license, &pubkey_hex).expect("verify");
        assert_eq!(verified_tier, Tier::Pro);

        // Tampered payload must fail verification
        let mut tampered = license.clone();
        tampered.payload.licensee = "hacker@example.com".to_owned();
        assert!(verify_license(&tampered, &pubkey_hex).is_err());

        let mut expired = license;
        expired.payload.expires_at = Some("2020-01-01T00:00:00Z".to_owned());
        let expired_bytes =
            serde_json::to_vec(&expired.payload).expect("serialize expired payload");
        expired.signature_hex = signing_key
            .sign(&expired_bytes)
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(matches!(
            verify_license(&expired, &pubkey_hex),
            Err(LicenseError::Expired)
        ));
    }
}
