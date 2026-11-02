//! Native desktop tray application controller and state (`SPEC §3.10`, `§4.5`).
//!
//! Provides tray menu actions, notification triggers, and window focus hooks
//! for the Gnomcode tray icon across Windows, macOS, and Linux.

pub mod focus;

use gnomcode_store::Database;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Tray application error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Store query failure.
    #[error("store error: {0}")]
    Store(#[from] gnomcode_store::Error),
}

/// Compact badge summary for the system tray icon (`SPEC §3.10`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayStatus {
    /// Ongoing count, including the two-minute idle grace period.
    pub active_count: usize,
    /// Number of sessions currently blocked awaiting user approval.
    pub awaiting_approval_count: usize,
    /// Formatted status tooltip string.
    pub tooltip: String,
}

/// Formats the tray tooltip and state badge.
#[must_use]
pub fn format_tray_tooltip(status: &TrayStatus) -> String {
    let suffix = if status.active_count == 1 { "" } else { "s" };
    if status.awaiting_approval_count > 0 {
        format!(
            "{} active session{suffix} · {} awaiting approval",
            status.active_count, status.awaiting_approval_count
        )
    } else if status.active_count > 0 {
        format!("{} active session{suffix}", status.active_count)
    } else {
        "No active sessions".to_owned()
    }
}

/// Computes the current [`TrayStatus`] from the active sessions in SQLite.
///
/// # Errors
/// Returns [`Error::Store`] if querying the database fails.
pub async fn compute_tray_status(db: &Arc<Mutex<Database>>) -> Result<TrayStatus, Error> {
    let guard = db.lock().await;
    let (_, active_count) = guard.list_sessions_paginated(None, Some("ongoing"), 0, 1)?;
    let (_, awaiting_approval_count) =
        guard.list_sessions_paginated(None, Some("awaiting_approval"), 0, 1)?;

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
        assert_eq!(format_tray_tooltip(&idle), "No active sessions");

        let running = TrayStatus {
            active_count: 3,
            awaiting_approval_count: 0,
            tooltip: String::new(),
        };
        assert_eq!(format_tray_tooltip(&running), "3 active sessions");

        let waiting = TrayStatus {
            active_count: 2,
            awaiting_approval_count: 1,
            tooltip: String::new(),
        };
        assert_eq!(
            format_tray_tooltip(&waiting),
            "2 active sessions · 1 awaiting approval"
        );
    }
}
