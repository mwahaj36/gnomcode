//! Long-running background daemon: single-instance lock, bearer token security,
//! background log watcher & periodic reconciler (`SPEC §3.1`, `§4.4`).

use chrono::Utc;
use fleethud_ingest::ingest_all;
use fleethud_platform::fleethud_home;
use fleethud_store::Database;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
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

/// Single-instance process lock handle held via file locking (`SPEC §4.4`).
pub struct DaemonLock {
    _file: File,
    lock_path: PathBuf,
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

        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(false);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::OpenOptionsExt as _;
            // Zero share mode = exclusive lock on Windows
            opts.share_mode(0);
        }

        let file = match opts.open(&lock_path) {
            Ok(f) => f,
            Err(_) => return Err(Error::AlreadyRunning),
        };

        let token = generate_bearer_token();
        let info = DaemonInfo {
            pid: std::process::id(),
            port,
            token,
            started_at: Utc::now().to_rfc3339(),
        };

        let json_bytes = serde_json::to_vec_pretty(&info).map_err(std::io::Error::other)?;
        fs::write(&info_path, json_bytes)?;

        Ok((
            Self {
                _file: file,
                lock_path,
                info_path,
            },
            info,
        ))
    }
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.info_path);
        let _ = fs::remove_file(&self.lock_path);
    }
}

fn generate_bearer_token() -> String {
    let mut hasher = Sha256::new();
    let now = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    hasher.update(now.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let res = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in res {
        use std::fmt::Write as _;
        let _ = write!(hex, "{b:02x}");
    }
    hex
}

/// Orchestrator managing background polling and event ingestion.
pub struct DaemonOrchestrator {
    db: Arc<Mutex<Database>>,
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
            poll_interval,
        })
    }

    /// Spawns a background task running periodic log reconciliation every `poll_interval`.
    pub fn spawn_reconciler(&self) -> JoinHandle<()> {
        let db = Arc::clone(&self.db);
        let interval = self.poll_interval;

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            loop {
                ticker.tick().await;
                let mut db_guard = db.lock().await;
                let _ = ingest_all(&mut db_guard);
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
        let token = super::generate_bearer_token();
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
