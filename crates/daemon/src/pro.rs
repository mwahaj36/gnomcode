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
    // 1. Check if authenticated cloud account session is paired
    if fleethud_home().join("session.json").exists() {
        return true;
    }

    // 2. Fall back to offline signed license key
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
    // 1. First check offline signed license key
    if let Ok(bytes) = fs::read(fleethud_home().join("license.key")) {
        if let Ok(license) = serde_json::from_slice::<LicenseFile>(&bytes) {
            if let Ok(tier) = verify_license(&license, FLEETHUD_LICENSE_PUBKEY_HEX) {
                let (tier_str, is_lic) = match tier {
                    Tier::Pro => ("pro", true),
                    Tier::Team => ("team", true),
                    Tier::Free => ("free", false),
                };
                return LicenseStatus {
                    tier: tier_str.to_owned(),
                    licensed: is_lic,
                    license_id: Some(format!(
                        "lic-{}",
                        &license.signature_hex[..12.min(license.signature_hex.len())]
                    )),
                    customer_name: Some(license.payload.licensee.clone()),
                    expires_at: license.payload.expires_at.clone(),
                };
            }
        }
    }

    // 2. Check if linked via cloud account pairing token
    if let Ok(raw) = fs::read_to_string(fleethud_home().join("session.json")) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
            let email = val
                .pointer("/user/email")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            let uid = val
                .pointer("/user/id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned());
            return LicenseStatus {
                tier: "pro".to_owned(),
                licensed: true,
                license_id: uid.map(|id| format!("cloud-{}", &id[..8.min(id.len())])),
                customer_name: email.or(Some("Cloud Account (Pro)".to_owned())),
                expires_at: None,
            };
        }
    }

    LicenseStatus {
        tier: "free".to_owned(),
        licensed: false,
        license_id: None,
        customer_name: None,
        expires_at: None,
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

/// Context Relay Engine (SPEC §3.14 / Pro Labs).
/// Scans active sessions for context window saturation >= threshold,
/// records pending relay events or auto-executes handoff based on policy.
pub fn run_context_relay(db: &Database) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !has_pro_license() {
        return Ok(());
    }

    let config = db.get_context_relay_config()?;
    if config.policy == "disabled" {
        return Ok(());
    }

    let (active_sessions, _) = db.list_sessions_paginated(None, Some("active"), 0, 50)?;
    let existing_relays = db.list_context_relays()?;

    for s in active_sessions {
        let Some(win) = s.context_window else {
            continue;
        };
        if win == 0 {
            continue;
        };
        let fill_pct = ((s.context_tokens as f64 / win as f64) * 100.0).round() as u32;

        if fill_pct >= config.threshold_pct {
            // Check if already recorded/pending for this session
            if existing_relays.iter().any(|r| {
                r.session_id == s.id
                    && (r.status == "pending" || r.status == "executed" || r.status == "dismissed")
            }) {
                continue;
            }

            // Determine target agent based on strategy
            let target_agent = if config.strategy == "failover" {
                if s.agent == "claude-code" {
                    "codex".to_owned()
                } else {
                    "claude-code".to_owned()
                }
            } else {
                s.agent.clone()
            };

            let relay_id = format!("relay-{}", s.id);
            let cwd = db
                .get_session_focus_target(&s.id)
                .ok()
                .flatten()
                .and_then(|t| t.repo_root)
                .filter(|p| !p.trim().is_empty())
                .or_else(|| db.find_antigravity_workspace(&s.id))
                .unwrap_or_else(|| {
                    std::env::current_dir()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| ".".to_owned())
                });

            let entry = fleethud_store::ContextRelayEntry {
                id: relay_id,
                session_id: s.id.clone(),
                source_agent: s.agent.clone(),
                target_agent: target_agent.clone(),
                project_name: s.project_name.clone(),
                cwd,
                fill_pct,
                status: if config.policy == "auto" {
                    "executed".to_owned()
                } else {
                    "pending".to_owned()
                },
                created_at: chrono::Utc::now().to_rfc3339(),
                executed_at: if config.policy == "auto" {
                    Some(chrono::Utc::now().to_rfc3339())
                } else {
                    None
                },
            };

            db.record_context_relay(&entry)?;

            // Dispatch desktop notification across platforms
            let title = format!("[RELAY] Context Reserve: {}", s.agent);
            let body = format!(
                "Session in {} reached {}% context. Relay checkpoint prepared for {}.",
                s.project_name, fill_pct, target_agent
            );
            let _ = fleethud_platform::send_desktop_notification(&title, &body);

            if config.policy == "auto" {
                let _ = execute_relay_handoff(&entry);
            }
        }
    }

    Ok(())
}

/// Executes a Context Relay handoff:
/// 1. Writes zero-knowledge .fleethud/HANDOFF.md checkpoint in project directory.
/// 2. Spawns fresh agent session in an interactive terminal window without permission escalation.
pub fn execute_relay_handoff(
    entry: &fleethud_store::ContextRelayEntry,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let repo_dir = std::path::Path::new(&entry.cwd);
    let fleethud_dir = repo_dir.join(".fleethud");
    let _ = std::fs::create_dir_all(&fleethud_dir);

    let handoff_path = fleethud_dir.join("HANDOFF.md");
    let content = format!(
        "# FleetHUD Context Relay Checkpoint\n\n\
        - **Source Agent:** {}\n\
        - **Target Agent:** {}\n\
        - **Project:** {}\n\
        - **Saturation:** {}% context consumed (Safety reserve active)\n\
        - **Timestamp:** {}\n\n\
        ## Continuity Instructions for Fresh Session\n\
        This workspace was handed off by FleetHUD Context Relay because the previous session reached the context saturation limit.\n\
        1. Inspect modified workspace files and recent git status (`git status`).\n\
        2. Verify existing test suites (`cargo test` / `npm test`).\n\
        3. Continue the task from this clean context window without repeating saturated tokens.\n",
        entry.source_agent,
        entry.target_agent,
        entry.project_name,
        entry.fill_pct,
        entry.created_at,
    );
    std::fs::write(&handoff_path, content)?;

    // Spawn the fresh target agent session in an interactive terminal or focus editor
    let _ = fleethud_platform::spawn_agent_in_terminal(&entry.target_agent, &entry.cwd);

    let title = format!("[RELAY] Handoff Complete: {}", entry.target_agent);
    let body = format!(
        "Checkpoint created in .fleethud/HANDOFF.md for {}. Clean session initialized.",
        entry.project_name
    );
    let _ = fleethud_platform::send_desktop_notification(&title, &body);

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
