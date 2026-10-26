//! Local Pro license checks, guardrails execution, and Resume Queue.

use fleethud_core::AgentId;
use fleethud_core::license::{FLEETHUD_LICENSE_PUBKEY_HEX, LicenseFile, Tier, verify_license};
use fleethud_core::rules::{RuleAction, evaluate_guardrails, is_resume_eligible};
use fleethud_platform::fleethud_home;
use fleethud_store::Database;
use serde::{Deserialize, Serialize};
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime};

/// Information about the current active license.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    /// Licensed tier (`free`, `pro`, `team`).
    pub tier: String,
    /// Whether a valid Pro or Team license is active.
    pub licensed: bool,
    /// License ID if signed license is present.
    pub license_id: Option<String>,
    /// Customer name or email if present in payload.
    pub customer_name: Option<String>,
    /// RFC3339 expiration timestamp if time-limited.
    pub expires_at: Option<String>,
}

/// Verifies the signed local license at a Pro feature entry point.
#[must_use]
#[inline(always)]
pub fn has_pro_license() -> bool {
    let Ok(bytes) = fs::read(fleethud_home().join("license.key")) else {
        return false;
    };
    let Ok(license) = serde_json::from_slice::<LicenseFile>(&bytes) else {
        return false;
    };
    matches!(
        verify_license(&license, FLEETHUD_LICENSE_PUBKEY_HEX),
        Ok(Tier::Pro | Tier::Team)
    )
}

/// Returns details on the local license status.
#[must_use]
pub fn get_license_status() -> LicenseStatus {
    let Ok(bytes) = fs::read(fleethud_home().join("license.key")) else {
        return LicenseStatus {
            tier: "free".to_owned(),
            licensed: false,
            license_id: None,
            customer_name: None,
            expires_at: None,
        };
    };

    let Ok(license) = serde_json::from_slice::<LicenseFile>(&bytes) else {
        return LicenseStatus {
            tier: "free".to_owned(),
            licensed: false,
            license_id: None,
            customer_name: None,
            expires_at: None,
        };
    };

    match verify_license(&license, FLEETHUD_LICENSE_PUBKEY_HEX) {
        Ok(tier) => {
            let (tier_str, is_lic) = match tier {
                Tier::Pro => ("pro", true),
                Tier::Team => ("team", true),
                Tier::Free => ("free", false),
            };
            LicenseStatus {
                tier: tier_str.to_owned(),
                licensed: is_lic,
                license_id: Some(format!(
                    "lic-{}",
                    &license.signature_hex[..12.min(license.signature_hex.len())]
                )),
                customer_name: Some(license.payload.licensee.clone()),
                expires_at: license.payload.expires_at.clone(),
            }
        }
        Err(_) => LicenseStatus {
            tier: "free".to_owned(),
            licensed: false,
            license_id: None,
            customer_name: None,
            expires_at: None,
        },
    }
}

/// Activates an Ed25519-signed license key.
pub fn activate_license(raw_input: &str) -> Result<LicenseStatus, String> {
    let trimmed = raw_input.trim();
    let license: LicenseFile =
        serde_json::from_str(trimmed).map_err(|e| format!("Invalid license JSON format: {e}"))?;

    let tier = verify_license(&license, FLEETHUD_LICENSE_PUBKEY_HEX)
        .map_err(|e| format!("License verification failed: {e}"))?;

    if tier == Tier::Free {
        return Err("License payload specifies Free tier; Pro or Team required.".to_owned());
    }

    let key_path = fleethud_home().join("license.key");
    fs::create_dir_all(fleethud_home())
        .map_err(|e| format!("Failed to create fleethud home: {e}"))?;
    fs::write(
        &key_path,
        serde_json::to_vec_pretty(&license).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Failed to save license key: {e}"))?;

    Ok(get_license_status())
}

/// Deletes the local license key, reverting to Free tier.
pub fn remove_license() -> Result<(), std::io::Error> {
    let key_path = fleethud_home().join("license.key");
    if key_path.exists() {
        fs::remove_file(key_path)?;
    }
    Ok(())
}

/// Evaluates persisted rules after ingestion and records alerts.
/// Claude hard stops are read by the fail-open PreToolUse hook.
pub fn run_guardrails(db: &Database) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let block_path = fleethud_home().join("blocked-sessions.json");
    if !has_pro_license() {
        let _ = fs::remove_file(block_path);
        return Ok(());
    }
    let rules = db.list_rules()?;
    let sessions = db.list_active_sessions(500)?;
    let mut blocked = Vec::new();

    for session in sessions {
        let (cost, turns) = db.session_guardrail_totals(&session.id)?;
        for rule in &rules {
            if let Some(action) =
                evaluate_guardrails(&session, cost, turns, std::slice::from_ref(rule))
            {
                let action_name = match action {
                    RuleAction::Warn => "warn",
                    RuleAction::HardStop => "hard_stop",
                };
                let is_new = db.insert_alert_once(&session.id, &rule.id, action_name)?;
                if is_new {
                    tracing::info!(
                        "Guardrail triggered for {}: rule {} ({})",
                        session.id,
                        rule.id,
                        action_name
                    );
                }
                if action == RuleAction::HardStop && session.agent == AgentId::ClaudeCode {
                    blocked.push(session.id.clone());
                }
            }
        }
    }

    // Persist blocked sessions list with fresh timestamp for fail-open safety
    fs::write(block_path, serde_json::to_vec(&blocked)?)?;
    Ok(())
}

/// Evaluates armed Resume Queue entries and auto-resumes eligible sessions.
pub fn run_resume_queue(db: &Database) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !has_pro_license() {
        return Ok(());
    }

    let entries = db.list_resume_queue()?;
    if entries.is_empty() {
        return Ok(());
    }

    let snapshots = db.get_latest_quota_snapshots()?;
    let now = chrono::Utc::now();

    for entry in entries {
        let matching_quota = snapshots.iter().find(|q| match entry.agent {
            AgentId::ClaudeCode => q.provider.eq_ignore_ascii_case("anthropic"),
            AgentId::Codex => q.provider.eq_ignore_ascii_case("openai"),
            AgentId::Antigravity => q.provider.eq_ignore_ascii_case("google"),
            _ => false,
        });

        if is_resume_eligible(&entry, now, matching_quota) {
            let native_id = entry
                .session_id
                .split_once(':')
                .map_or(entry.session_id.as_str(), |(_, id)| id);

            tracing::info!(
                "Executing Resume Queue for session {} ({}) in {}",
                entry.session_id,
                entry.agent.as_str(),
                entry.cwd
            );

            // Execute resume command in background without escalating permissions
            let spawned = match entry.agent {
                AgentId::ClaudeCode => Command::new("claude")
                    .args(["--resume", native_id])
                    .current_dir(&entry.cwd)
                    .spawn(),
                AgentId::Codex => Command::new("codex")
                    .args(["resume", native_id])
                    .current_dir(&entry.cwd)
                    .spawn(),
                _ => continue,
            };

            match spawned {
                Ok(_) => {
                    let next_runs = entry.runs_completed + 1;
                    if next_runs >= entry.max_runs {
                        let _ = db.disarm_resume_queue(&entry.session_id);
                    } else {
                        let _ = db.update_resume_runs(&entry.session_id, next_runs);
                    }
                }
                Err(err) => {
                    tracing::warn!(
                        "Failed to spawn auto-resume for session {}: {err}",
                        entry.session_id
                    );
                }
            }
        }
    }

    Ok(())
}

/// Checks whether a session is currently blocked by a guardrail rule.
/// Used for 50ms loopback IPC query.
pub fn is_session_blocked(session_id: &str) -> bool {
    let block_path = fleethud_home().join("blocked-sessions.json");
    let Ok(meta) = fs::metadata(&block_path) else {
        return false;
    };

    // Stale check: if file is older than 30s, fail open
    if let Ok(mtime) = meta.modified() {
        if let Ok(elapsed) = SystemTime::now().duration_since(mtime) {
            if elapsed > Duration::from_secs(30) {
                return false;
            }
        }
    }

    let Ok(raw) = fs::read(block_path) else {
        return false;
    };
    let Ok(blocked) = serde_json::from_slice::<Vec<String>>(&raw) else {
        return false;
    };
    blocked.iter().any(|s| s == session_id)
}
