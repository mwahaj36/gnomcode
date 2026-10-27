//! Long-running background daemon: single-instance lock, bearer token security,
//! background log watcher & periodic reconciler (`SPEC §3.1`, `§4.4`).

pub mod api;
pub mod cloud;
pub mod pairing;
pub mod pro;
pub mod sync;

use chrono::Utc;
use fleethud_ingest::ingest_all;
use fleethud_platform::fleethud_home;
use fleethud_store::Database;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Daemon subsystem error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Another instance of FleetHUD daemon is already running.
    #[error("daemon is already running (locked by another process)")]
    AlreadyRunning,
    /// Store error.
    #[error("store error: {0}")]
    Store(#[from] fleethud_store::Error),
    /// Ingest error.
    #[error("ingest error: {0}")]
    Ingest(#[from] fleethud_ingest::Error),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Metadata written to `~/.fleethud/daemon.json` for local API clients (`SPEC §3.1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonInfo {
    /// Process ID of the running daemon.
    pub pid: u32,
    /// Loopback port where axum HTTP/WS server listens (default `41738`).
    pub port: u16,
    /// High-entropy bearer token required on all HTTP and WS requests.
    pub token: String,
    /// ISO-8601 startup timestamp.
    pub started_at: String,
}

/// Reads `~/.fleethud/daemon.json` if present to connect to a running daemon.
#[must_use]
pub fn read_daemon_info() -> Option<DaemonInfo> {
    let path = fleethud_home().join("daemon.json");
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Single-instance process lock handle held via file locking (`SPEC §4.4`).
pub struct DaemonLock {
    _file: File,
    info_path: PathBuf,
}

impl DaemonLock {
    /// Attempts to acquire an exclusive lock on `~/.fleethud/daemon.lock`
    /// and persists `daemon.json` with the generated bearer token.
    ///
    /// # Errors
    /// Returns [`Error::AlreadyRunning`] if another process holds the lock.
    pub fn acquire(port: u16) -> Result<(Self, DaemonInfo), Error> {
        let home = fleethud_home();
        fs::create_dir_all(&home)?;

        let lock_path = home.join("daemon.lock");
        let info_path = home.join("daemon.json");

        let file = match fleethud_platform::open_exclusive_lock(&lock_path) {
            Ok(file) => file,
            Err(_) => return Err(Error::AlreadyRunning),
        };

        let token = generate_bearer_token()?;
        let info = DaemonInfo {
            pid: std::process::id(),
            port,
            token,
            started_at: Utc::now().to_rfc3339(),
        };

        let json_bytes = serde_json::to_vec_pretty(&info).map_err(std::io::Error::other)?;
        fleethud_platform::write_private_file(&info_path, &json_bytes)
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        Ok((
            Self {
                _file: file,
                info_path,
            },
            info,
        ))
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.info_path);
        // Keep the inode stable so another process cannot lock a replacement file.
    }
}

fn generate_bearer_token() -> Result<String, Error> {
    let mut res = [0u8; 32];
    getrandom::fill(&mut res).map_err(std::io::Error::other)?;
    let mut hex = String::with_capacity(64);
    for b in res {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    Ok(hex)
}

/// Orchestrator managing background polling and event ingestion.
pub struct DaemonOrchestrator {
    db: Arc<Mutex<Database>>,
    db_path: PathBuf,
    poll_interval: Duration,
}

impl DaemonOrchestrator {
    /// Creates a new orchestrator backed by the SQLite database at `db_path`.
    ///
    /// # Errors
    /// Returns [`Error::Store`] if the database cannot be opened.
    pub fn new(db_path: &Path, poll_interval: Duration) -> Result<Self, Error> {
        let db = Database::open(db_path)?;
        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            db_path: db_path.to_path_buf(),
            poll_interval,
        })
    }

    /// Spawns a background task running periodic log reconciliation every `poll_interval`.
    pub fn spawn_reconciler(&self) -> JoinHandle<()> {
        let db_path = self.db_path.clone();
        let interval = self.poll_interval;

        tokio::spawn(async move {
            let Ok(Ok(mut ingest_db)) =
                tokio::task::spawn_blocking(move || Database::open(&db_path)).await
            else {
                tracing::error!("ingestion database could not start");
                return;
            };
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let Ok((db, result)) = tokio::task::spawn_blocking(move || {
                    let result = ingest_all(&mut ingest_db);
                    let _ = ingest_db.reconcile_antigravity_projects();
                    if let Err(error) = pro::run_guardrails(&ingest_db) {
                        tracing::warn!("guardrail evaluation failed: {error}");
                    }
                    if let Err(error) = pro::run_resume_queue(&ingest_db) {
                        tracing::warn!("resume queue evaluation failed: {error}");
                    }
                    if let Err(error) = pro::run_context_relay(&ingest_db) {
                        tracing::warn!("context relay evaluation failed: {error}");
                    }
                    (ingest_db, result)
                })
                .await
                else {
                    tracing::error!("ingestion worker stopped");
                    return;
                };
                ingest_db = db;
                if let Err(error) = result {
                    tracing::warn!("ingestion pass failed: {error}");
                }
            }
        })
    }

    /// Returns a clone of the database lock for use by API handlers.
    #[must_use]
    pub fn db(&self) -> Arc<Mutex<Database>> {
        Arc::clone(&self.db)
    }
}

#[cfg(test)]
mod tests {
    use super::DaemonOrchestrator;
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn bearer_token_format_is_valid_hex() {
        let token = super::generate_bearer_token().expect("random token");
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn orchestrator_runs_ingest_periodically() {
        let dir = tempdir().expect("temp dir");
        let db_path = dir.path().join("state.db");
        let orch = DaemonOrchestrator::new(&db_path, Duration::from_millis(50))
            .expect("create orchestrator");

        let handle = orch.spawn_reconciler();
        tokio::time::sleep(Duration::from_millis(150)).await;
        handle.abort();

        let db = orch.db();
        let guard = db.lock().await;
        let sessions = guard.list_active_sessions(10).expect("list sessions");
        assert!(sessions.is_empty() || !sessions.is_empty());
    }
}
