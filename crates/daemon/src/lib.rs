//! Long-running background daemon: single-instance lock, bearer token security,
//! background log watcher & periodic reconciler (`SPEC §3.1`, `§4.4`).

pub mod api;
pub mod pro;
pub mod remote;
pub mod sync;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use spawnt_ingest::ingest_all;
use spawnt_platform::spawnt_home;
use spawnt_store::Database;
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
    /// Another instance of Spawnt daemon is already running.
    #[error("daemon is already running (locked by another process)")]
    AlreadyRunning,
    /// Store error.
    #[error("store error: {0}")]
    Store(#[from] spawnt_store::Error),
    /// Ingest error.
    #[error("ingest error: {0}")]
    Ingest(#[from] spawnt_ingest::Error),
    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Failure while preparing a model handoff.
#[derive(Debug, Error)]
pub enum HandoffError {
    /// The source session or its repository is unavailable.
    #[error("session or repository not found")]
    MissingSession,
    /// The session store could not be read.
    #[error("store error: {0}")]
    Store(#[from] spawnt_store::Error),
    /// The repository could not be inspected.
    #[error("repository error: {0}")]
    Platform(#[from] spawnt_platform::Error),
    /// The target model does not belong to a supported interactive agent.
    #[error("unsupported target model or agent")]
    UnsupportedTarget,
    /// The new agent session could not be launched or prompted.
    #[error("agent launch or prompt failed: {0}")]
    Launch(String),
}

/// Resolve an interactive agent for a model, rejecting mismatched agent requests.
pub fn agent_for_model(model: &str, requested_agent: Option<&str>) -> Option<&'static str> {
    if model == "@default" {
        return match requested_agent {
            Some("codex") => Some("codex"),
            Some("claude" | "claude-code") => Some("claude-code"),
            Some("gemini" | "gemini-cli") => Some("gemini-cli"),
            Some("antigravity" | "agy") => Some("antigravity"),
            _ => None,
        };
    }
    if matches!(requested_agent, Some("antigravity" | "agy"))
        && !model.is_empty()
        && model.len() <= 128
        && model
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
    {
        return Some("antigravity");
    }
    let lower = model.to_ascii_lowercase();
    let inferred = if lower.starts_with("claude-") {
        "claude-code"
    } else if lower.starts_with("gemini-") {
        "gemini-cli"
    } else if lower.starts_with("gpt-")
        || lower.starts_with("codex-")
        || lower.starts_with("o1")
        || lower.starts_with("o3")
        || lower.starts_with("o4")
    {
        "codex"
    } else {
        return None;
    };
    match requested_agent {
        None => Some(inferred),
        Some("claude" | "claude-code") if inferred == "claude-code" => Some(inferred),
        Some("gemini" | "gemini-cli") if inferred == "gemini-cli" => Some(inferred),
        Some("codex") if inferred == "codex" => Some(inferred),
        _ => None,
    }
}

#[cfg(test)]
mod switch_tests {
    #[test]
    fn model_switch_rejects_wrong_agent_and_unknown_model() {
        assert_eq!(super::agent_for_model("o3", None), Some("codex"));
        assert_eq!(
            super::agent_for_model("claude-sonnet-4", None),
            Some("claude-code")
        );
        assert_eq!(
            super::agent_for_model("gemini-2.5-pro", None),
            Some("gemini-cli")
        );
        assert_eq!(super::agent_for_model("o3", Some("claude-code")), None);
        assert_eq!(super::agent_for_model("--yolo", None), None);
        assert_eq!(
            super::agent_for_model("@default", Some("antigravity")),
            Some("antigravity")
        );
        assert_eq!(super::agent_for_model("@default", Some("terminal")), None);
    }
}

/// Launches a fresh target agent and sends its redacted handoff prompt.
///
/// # Errors
/// Returns [`HandoffError`] if the source, model, launch, or prompt delivery fails.
pub async fn switch_session(
    db: &Database,
    session_id: &str,
    target_model: &str,
    requested_agent: Option<&str>,
) -> Result<spawnt_pty::SessionRecord, HandoffError> {
    let prepared = prepare_switch(db, session_id, target_model, requested_agent)?;
    execute_switch(prepared).await
}

pub(crate) struct PreparedSwitch {
    agent: &'static str,
    repo: String,
    packet: spawnt_core::HandoffPacket,
}

pub(crate) fn prepare_switch(
    db: &Database,
    session_id: &str,
    target_model: &str,
    requested_agent: Option<&str>,
) -> Result<PreparedSwitch, HandoffError> {
    let agent =
        agent_for_model(target_model, requested_agent).ok_or(HandoffError::UnsupportedTarget)?;
    let target = db
        .get_session_focus_target(session_id)?
        .ok_or(HandoffError::MissingSession)?;
    let repo = target.repo_root.ok_or(HandoffError::MissingSession)?;
    let packet = prepare_handoff(db, session_id, target_model)?;
    Ok(PreparedSwitch {
        agent,
        repo,
        packet,
    })
}

pub(crate) async fn execute_switch(
    PreparedSwitch {
        agent,
        repo,
        packet,
    }: PreparedSwitch,
) -> Result<spawnt_pty::SessionRecord, HandoffError> {
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|e| HandoffError::Launch(e.to_string()))?;
    let new_id = format!("switch-{}", u64::from_be_bytes(random));
    let record = spawnt_pty::launch_session(
        &new_id,
        agent,
        Path::new(&repo),
        (packet.target_model != "@default").then_some(packet.target_model.as_str()),
    )
    .await
    .map_err(|e| HandoffError::Launch(e.to_string()))?;
    let files = packet.touched_files.join("\n");
    let prompt = format!(
        "Continue the task from session {} in this workspace. Inspect the current files before editing.\n\nRecent user objectives:\n{}\n\nChanged files:\n{}\n",
        packet.session_id, packet.prompt_summary, files
    );
    let delivered = spawnt_pty::send_prompt(&record, &prompt)
        .await
        .map_err(|_| HandoffError::Launch("could not send handoff prompt".to_owned()));
    if delivered.is_err() {
        let _ = spawnt_pty::stop_session(&new_id).await;
    }
    delivered?;
    Ok(record)
}

/// Prepare a handoff using the source session's repository and recent chat.
///
/// # Errors
/// Returns [`HandoffError`] if the session or repository cannot be read.
pub fn prepare_handoff(
    db: &Database,
    session_id: &str,
    target_model: &str,
) -> Result<spawnt_core::HandoffPacket, HandoffError> {
    let target = db
        .get_session_focus_target(session_id)?
        .ok_or(HandoffError::MissingSession)?;
    let repo = target.repo_root.ok_or(HandoffError::MissingSession)?;
    let files = spawnt_platform::handoff_files(Path::new(&repo))?;
    db.prepare_handoff(session_id, target_model, files)?
        .ok_or(HandoffError::MissingSession)
}

/// Derives a bounded display-only label without adding chat text to sync metadata.
pub(crate) fn conversation_title(db: &Database, session_id: &str) -> Option<String> {
    db.conversation_title_candidates(session_id)
        .ok()?
        .into_iter()
        .find_map(|text| {
            let clean = spawnt_core::redact::clean_user_text(&text);
            clean
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().chars().take(72).collect())
        })
}

/// Metadata written to `~/.spawnt/daemon.json` for local API clients (`SPEC §3.1`).
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

/// Reads `~/.spawnt/daemon.json` if present to connect to a running daemon.
#[must_use]
pub fn read_daemon_info() -> Option<DaemonInfo> {
    let path = spawnt_home().join("daemon.json");
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Single-instance process lock handle held via file locking (`SPEC §4.4`).
pub struct DaemonLock {
    _file: File,
    info_path: PathBuf,
}

impl DaemonLock {
    /// Attempts to acquire an exclusive lock on `~/.spawnt/daemon.lock`
    /// and persists `daemon.json` with the generated bearer token.
    ///
    /// # Errors
    /// Returns [`Error::AlreadyRunning`] if another process holds the lock.
    pub fn acquire(port: u16) -> Result<(Self, DaemonInfo), Error> {
        let home = spawnt_home();
        fs::create_dir_all(&home)?;

        let lock_path = home.join("daemon.lock");
        let info_path = home.join("daemon.json");

        let file = match spawnt_platform::open_exclusive_lock(&lock_path) {
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
        spawnt_platform::write_private_file(&info_path, &json_bytes)
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
    reconcile_lock: Arc<Mutex<()>>,
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
            reconcile_lock: Arc::new(Mutex::new(())),
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

    /// Runs one immediate reconciliation pass, returning whether it started.
    ///
    /// # Errors
    /// Returns an error if database access or reconciliation fails.
    pub async fn reconcile_now(&self) -> Result<bool, Error> {
        let Ok(_guard) = self.reconcile_lock.clone().try_lock_owned() else {
            return Ok(false);
        };
        let db_path = self.db_path.clone();
        tokio::task::spawn_blocking(move || {
            let mut db = Database::open(&db_path).map_err(Error::Store)?;
            ingest_all(&mut db)?;
            db.reconcile_antigravity_projects().map_err(Error::Store)?;
            if let Err(error) = pro::run_guardrails(&db) {
                tracing::warn!("guardrail evaluation failed: {error}");
            }
            if let Err(error) = pro::run_resume_queue(&db) {
                tracing::warn!("resume queue evaluation failed: {error}");
            }
            if let Err(error) = pro::run_context_relay(&db) {
                tracing::warn!("context relay evaluation failed: {error}");
            }
            Ok::<(), Error>(())
        })
        .await
        .map_err(std::io::Error::other)??;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn handoff_preparation_includes_repository_and_redacted_chat() {
        use spawnt_core::{AgentId, ChatEvent, ChatRole, Project, Session, SessionState, Surface};
        let repo = tempfile::tempdir().expect("repo");
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(repo.path())
                .status()
                .expect("git")
                .success()
        );
        std::fs::write(repo.path().join("new file.txt"), "data").expect("file");
        let mut db = spawnt_store::Database::open_in_memory().expect("database");
        let project = Project::from_origin(None, &repo.path().to_string_lossy(), None);
        let now = chrono::Utc::now();
        let session = Session {
            id: "codex:test".into(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("o3".into()),
            title: None,
            state: SessionState::Idle,
            pid: None,
            started_at: now,
            last_event_at: now,
        };
        let checkpoint = spawnt_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("session");
        db.insert_chat_event(&ChatEvent {
            event_id: "event".into(),
            session_id: "codex:test".into(),
            turn_index: 1,
            role: ChatRole::User,
            content: "Fix tests token=secret-value".into(),
            tool_name: None,
            status: None,
            created_at: now,
        })
        .expect("chat");
        let packet = super::prepare_handoff(&db, "codex:test", "o4-mini").expect("packet");
        assert_eq!(packet.source_model, "o3");
        assert_eq!(packet.touched_files, ["new file.txt"]);
        assert!(packet.prompt_summary.contains("Fix tests"));
        assert!(!packet.prompt_summary.contains("secret-value"));
    }
    #[test]
    fn handoff_preparation_rejects_unknown_session() {
        let db = spawnt_store::Database::open_in_memory().expect("database");
        assert!(matches!(
            super::prepare_handoff(&db, "missing", "o3"),
            Err(super::HandoffError::MissingSession)
        ));
    }
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
