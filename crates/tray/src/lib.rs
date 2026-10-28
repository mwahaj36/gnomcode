//! Native desktop tray application controller and state (`SPEC §3.10`, `§4.5`).
//!
//! Provides tray menu actions, notification triggers, and window focus hooks
//! for the FleetHUD tray icon across Windows, macOS, and Linux.

pub mod focus;

use fleethud_core::SessionState;
use fleethud_store::Database;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Tray application error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Store query failure.
    #[error("store error: {0}")]
    Store(#[from] fleethud_store::Error),
}

/// Compact badge summary for the system tray icon (`SPEC §3.10`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayStatus {
    /// Number of live sessions currently executing a tool or generating tokens.
    pub active_count: usize,
    /// Number of sessions currently blocked awaiting user approval.
    pub awaiting_approval_count: usize,
    /// Formatted status tooltip string.
    pub tooltip: String,
}

/// Formats the tray tooltip and state badge.
#[must_use]
pub fn format_tray_tooltip(status: &TrayStatus) -> String {
    if status.awaiting_approval_count > 0 {
        format!(
            "spawnt · {} active EXACT · {} awaiting approval EXACT",
            status.active_count, status.awaiting_approval_count
        )
    } else if status.active_count > 0 {
        format!("spawnt · {} active sessions EXACT", status.active_count)
    } else {
        "spawnt · IDLE".to_owned()
    }
}

/// Computes the current [`TrayStatus`] from the active sessions in SQLite.
///
/// # Errors
/// Returns [`Error::Store`] if querying the database fails.
pub async fn compute_tray_status(db: &Arc<Mutex<Database>>) -> Result<TrayStatus, Error> {
    let guard = db.lock().await;
    let sessions = guard.list_active_sessions(50)?;

    let mut active_count = 0;
    let mut awaiting_approval_count = 0;

    for s in &sessions {
        match s.state {
            SessionState::Thinking | SessionState::Tool => active_count += 1,
            SessionState::AwaitingApproval => awaiting_approval_count += 1,
            SessionState::Idle | SessionState::Error | SessionState::Ended => {}
        }
    }

    let status = TrayStatus {
        active_count,
        awaiting_approval_count,
        tooltip: String::new(),
    };
    let tooltip = format_tray_tooltip(&status);

    Ok(TrayStatus {
        active_count,
        awaiting_approval_count,
        tooltip,
    })
}

#[cfg(test)]
mod tests {
    use super::{TrayStatus, format_tray_tooltip};

    #[test]
    fn format_tray_tooltip_uses_plain_branded_status_and_trust_labels() {
        let idle = TrayStatus {
            active_count: 0,
            awaiting_approval_count: 0,
            tooltip: String::new(),
        };
        assert_eq!(format_tray_tooltip(&idle), "spawnt · IDLE");

        let running = TrayStatus {
            active_count: 3,
            awaiting_approval_count: 0,
            tooltip: String::new(),
        };
        assert_eq!(
            format_tray_tooltip(&running),
            "spawnt · 3 active sessions EXACT"
        );

        let waiting = TrayStatus {
            active_count: 2,
            awaiting_approval_count: 1,
            tooltip: String::new(),
        };
        assert_eq!(
            format_tray_tooltip(&waiting),
            "spawnt · 2 active EXACT · 1 awaiting approval EXACT"
        );
    }
}
