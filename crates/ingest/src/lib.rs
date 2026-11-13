//! File watching, reconciliation, incremental tailing, and checkpointing
//! of agent logs into the SQLite store (`SPEC §3.5`, `§4.4`).

mod antigravity;
pub mod watcher;

use chrono::Utc;
use gnomcode_adapters::{
    FileCtx, SourceAdapter,
    antigravity::{AntigravityAdapter, extract_conversation_id},
    claude::ClaudeAdapter,
    codex::{CodexAdapter, codex_session_id_from_path},
    gemini::GeminiAdapter,
};
use gnomcode_core::{AgentId, Project, Session, Surface};
use gnomcode_platform::{
    AgentRoot, detect_local_subscriptions, discover_agent_roots, read_antigravity_fallback_model,
    read_from_offset,
};
use gnomcode_store::{Database, SourceCheckpoint};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use thiserror::Error;

/// Ingestion engine error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Platform reading error.
    #[error("platform error: {0}")]
    Platform(#[from] gnomcode_platform::Error),
    /// Store operation error.
    #[error("store error: {0}")]
    Store(#[from] gnomcode_store::Error),
    /// Standard I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Discovered log file candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredFile {
    /// Owning agent.
    pub agent: AgentId,
    /// Absolute path to the log file.
    pub path: PathBuf,
}

/// Recursively scans agent log directories for parseable log files (`.jsonl`, `.json`).
#[must_use]
pub fn scan_log_files(roots: &[AgentRoot]) -> Vec<DiscoveredFile> {
    scan_log_files_with_extra(roots, true)
}

/// Scans agent log directories, optionally including extra adapters (`GeminiCli`) when enabled.
#[must_use]
pub fn scan_log_files_with_extra(
    roots: &[AgentRoot],
    include_extra_adapters: bool,
) -> Vec<DiscoveredFile> {
    let mut files = Vec::new();
    for root in roots {
        if !root.exists || (!include_extra_adapters && root.agent == AgentId::GeminiCli) {
            continue;
        }
        if root.path.is_file() {
            if is_agent_log_file(root.agent, &root.path) {
                files.push(DiscoveredFile {
                    agent: root.agent,
                    path: root.path.clone(),
                });
            }
        } else if root.agent == AgentId::Antigravity {
            // Brain folders hold thousands of artifacts; transcripts sit at a fixed depth.
            for conversation in fs::read_dir(&root.path).into_iter().flatten().flatten() {
                let path = conversation
                    .path()
                    .join(".system_generated")
                    .join("logs")
                    .join("transcript.jsonl");
                if path.is_file() {
                    files.push(DiscoveredFile {
                        agent: root.agent,
                        path,
                    });
                }
            }
        } else {
            collect_files_recursively(root.agent, &root.path, &mut files);
        }
    }
    files
}

fn collect_files_recursively(agent: AgentId, dir: &Path, out: &mut Vec<DiscoveredFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_recursively(agent, &path, out);
        } else if is_agent_log_file(agent, &path) {
            out.push(DiscoveredFile { agent, path });
        }
    }
}

fn is_agent_log_file(agent: AgentId, path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
        return false;
    };
    match agent {
        AgentId::ClaudeCode => ext.eq_ignore_ascii_case("jsonl"),
        AgentId::Codex => {
            ext.eq_ignore_ascii_case("jsonl")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n.starts_with("rollout-"))
                || is_archived_codex_rollout(path)
        }
        AgentId::Antigravity => {
            ext.eq_ignore_ascii_case("jsonl")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n == "transcript.jsonl")
        }
        AgentId::GeminiCli => ext.eq_ignore_ascii_case("json"),
    }
}

/// Newer Codex versions compress finished rollouts to `rollout-*.jsonl.zst`.
fn is_archived_codex_rollout(path: &Path) -> bool {
    path.file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl.zst"))
}

/// Upper bound for one decompressed archive; larger ones are skipped rather than exhausting memory.
const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

fn decompress_archive(bytes: &[u8]) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let decoder = ruzstd::decoding::StreamingDecoder::new(bytes).ok()?;
    let mut out = Vec::new();
    decoder
        .take(MAX_ARCHIVE_BYTES + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() as u64 <= MAX_ARCHIVE_BYTES).then_some(out)
}

fn complete_jsonl_prefix_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |i| i + 1)
}

/// Resolves a session folder to its repository: the Git remote when there is one, otherwise the
/// repository root (worktrees and subfolders share their repository). Scratch folders, unassigned
/// folders and folders outside any usable repository keep their path identity.
#[must_use]
pub fn resolve_project(project: &Project) -> Project {
    if project.is_unassigned() || project.is_scratch() {
        return project.clone();
    }
    let (root, remote) =
        gnomcode_platform::resolve_git_repo_and_remote(Path::new(&project.repo_root));
    let Some(root) = root else {
        return project.clone();
    };
    // A project already keyed by its remote keeps that key even when the folder has none.
    let remote = if project.has_remote_key() {
        Some(project.canonical_key.clone())
    } else {
        remote
    };
    let mut resolved = Project::from_origin(
        remote.as_deref(),
        &root.to_string_lossy(),
        project.git_branch.as_deref(),
    );
    if resolved.is_unassigned() || resolved.is_scratch() {
        return project.clone();
    }
    // People know a project by its folder; a renamed remote would otherwise show an old name.
    if let Some(folder) = root.file_name().and_then(|name| name.to_str()) {
        folder.clone_into(&mut resolved.name);
    }
    resolved
}

/// Resolves the projects of a freshly parsed batch and points its sessions at the resolved ids.
///
/// Returns each session's own folder (before resolution; none for Unassigned), so resume can start
/// the agent where the session ran even when its project merges several folders.
fn resolve_batch_projects(batch: &mut gnomcode_adapters::ParsedBatch) -> Vec<(String, String)> {
    let cwds = batch
        .sessions
        .iter()
        .filter_map(|session| {
            let project = batch.projects.iter().find(|p| p.id == session.project_id)?;
            (!project.is_unassigned()).then(|| (session.id.clone(), project.repo_root.clone()))
        })
        .collect();
    for project in &mut batch.projects {
        let resolved = resolve_project(project);
        if resolved.id == project.id {
            *project = resolved;
            continue;
        }
        for session in batch
            .sessions
            .iter_mut()
            .filter(|session| session.project_id == project.id)
        {
            session.project_id.clone_from(&resolved.id);
        }
        *project = resolved;
    }
    cwds
}

fn record_cwds(db: &Database, cwds: &[(String, String)]) {
    for (session, cwd) in cwds {
        if let Err(error) = db.record_session_cwd(session, cwd) {
            tracing::warn!("session folder could not be recorded: {error}");
        }
    }
}

/// Recomputes the derived identity of every stored project (names, merged spellings, repository
/// roots). Sessions, usage and chat are untouched; only project rows are merged or renamed.
///
/// # Errors
/// Returns [`Error`] if the store fails.
pub fn reproject_all(db: &mut Database) -> Result<usize, Error> {
    let mut changed = 0;
    for project in db.list_projects()? {
        // A remote-keyed project keeps its key (the stored key is already normalized).
        let remote = project
            .has_remote_key()
            .then_some(project.canonical_key.as_str());
        let target = resolve_project(&Project::from_origin(
            remote,
            &project.repo_root,
            project.git_branch.as_deref(),
        ));
        if target.id != project.id
            || target.name != project.name
            || target.repo_root != project.repo_root
        {
            if target.id != project.id && !target.is_unassigned() {
                // Before the merge a stored project is one folder: each session's own.
                db.backfill_session_cwd(&project.id, &project.repo_root)?;
            }
            db.reassign_project(&project.id, &target)?;
            changed += 1;
        }
    }
    Ok(changed)
}

/// Ingests all available agent logs across discovered roots in one synchronous pass.
///
/// # Errors
/// Returns [`Error`] if database commit or reading fails.
pub fn ingest_all(db: &mut Database) -> Result<u64, Error> {
    reproject_all(db)?;
    let include_extra = db.get_feature_toggles()?.extra_adapters_enabled;
    let roots = discover_agent_roots();
    let discovered = scan_log_files_with_extra(&roots, include_extra);
    let repair_chat_format = db.chat_format_needs_repair()?;
    let total = ingest_files(db, &roots, &discovered, repair_chat_format)?;
    ingest_claude_status_limits(
        db,
        &gnomcode_platform::gnomcode_home().join(CLAUDE_LIMITS_FILE),
    )?;
    Ok(total)
}

/// Written by `gnomcode statusline` from Claude Code's status line input.
pub const CLAUDE_LIMITS_FILE: &str = "claude-limits.json";

fn ingest_claude_status_limits(db: &mut Database, path: &Path) -> Result<(), Error> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(());
    };
    let path_str = path.to_string_lossy().into_owned();
    // The file is a few hundred bytes of limits only, so its content is the fingerprint.
    let fingerprint = String::from_utf8_lossy(&bytes).into_owned();
    if db
        .get_checkpoint(&path_str)?
        .is_some_and(|checkpoint| checkpoint.fingerprint == fingerprint)
    {
        return Ok(());
    }
    let quota = gnomcode_adapters::claude::parse_status_limits(&bytes);
    let size = bytes.len() as u64;
    db.commit_batch(
        &[],
        &[],
        &[],
        &[],
        &quota,
        &SourceCheckpoint {
            file_path: path_str,
            last_read_offset: size,
            file_size: size,
            fingerprint,
            last_ingested_at: Utc::now(),
        },
    )?;
    Ok(())
}

fn ingest_files(
    db: &mut Database,
    roots: &[AgentRoot],
    discovered: &[DiscoveredFile],
    repair_chat_format: bool,
) -> Result<u64, Error> {
    let mut total_records_ingested = 0;
    let mut repair_complete = true;

    let agy_fallback_model = read_antigravity_fallback_model();
    let agy = antigravity::Catalog::scan(roots);

    for file in discovered {
        let path_str = file.path.to_string_lossy().into_owned();
        let checkpoint = match db.get_checkpoint(&path_str) {
            Ok(cp) => cp,
            Err(e) => {
                tracing::warn!("skipping checkpoint lookup for {path_str}: {e}");
                repair_complete = false;
                continue;
            }
        };
        let start_offset = checkpoint.as_ref().map_or(0, |c| c.last_read_offset);
        let archived = is_archived_codex_rollout(&file.path);
        let is_rewritten = file.agent == AgentId::GeminiCli || archived;

        let file_fingerprint = if is_rewritten {
            fs::metadata(&file.path).ok().and_then(|meta| {
                let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
                Some(format!("{}:{}", meta.len(), modified.as_nanos()))
            })
        } else {
            None
        };
        if !repair_chat_format
            && file_fingerprint.as_ref().is_some_and(|fingerprint| {
                checkpoint
                    .as_ref()
                    .is_some_and(|cp| &cp.fingerprint == fingerprint)
            })
        {
            continue;
        }

        let read_offset = if is_rewritten || repair_chat_format {
            0
        } else {
            start_offset
        };
        let Ok(mut read_result) = read_from_offset(&file.path, read_offset) else {
            repair_complete = false;
            continue;
        };
        if archived {
            let Some(bytes) = decompress_archive(&read_result.bytes) else {
                tracing::warn!("skipping unreadable Codex archive");
                continue;
            };
            read_result.bytes = bytes;
        }
        if read_result.bytes.is_empty() && read_offset == read_result.file_size {
            continue;
        }

        let fallback_model = match file.agent {
            AgentId::Codex => {
                let sid =
                    Session::compose_id(AgentId::Codex, codex_session_id_from_path(&path_str));
                db.get_session_model(&sid)
            }
            AgentId::Antigravity => {
                let sid =
                    Session::compose_id(AgentId::Antigravity, extract_conversation_id(&path_str));
                // The last model chosen in the 2.0 app says nothing about CLI or IDE sessions.
                db.get_session_model(&sid).or_else(|| {
                    (agy.owner(extract_conversation_id(&path_str)) == Some(Surface::DesktopApp))
                        .then(|| agy_fallback_model.clone())
                        .flatten()
                })
            }
            _ => None,
        };

        let codex_session = (file.agent == AgentId::Codex)
            .then(|| Session::compose_id(AgentId::Codex, codex_session_id_from_path(&path_str)));
        let fallback_cwd = if let Some(id) = codex_session.as_deref() {
            db.get_session_focus_target(id)?
                .and_then(|target| target.repo_root)
        } else if file.agent == AgentId::Antigravity {
            let conv_id = extract_conversation_id(&path_str);
            db.find_antigravity_workspace(conv_id)
        } else {
            None
        };

        let ctx = FileCtx {
            source_path: path_str.clone(),
            source_offset: read_result.effective_start_offset,
            fallback_model,
            fallback_cwd,
            fallback_state: codex_session
                .as_deref()
                .map(|id| db.session_state(id))
                .transpose()?
                .flatten(),
        };

        let mut adapter: Box<dyn SourceAdapter> = match file.agent {
            AgentId::ClaudeCode => Box::new(ClaudeAdapter::new()),
            AgentId::Codex => Box::new(CodexAdapter::new()),
            AgentId::Antigravity => Box::new(AntigravityAdapter::new().with_usage_from_trajectory(
                agy.usage_from_trajectory(db, extract_conversation_id(&path_str)),
            )),
            AgentId::GeminiCli => Box::new(GeminiAdapter::new()),
        };

        let parsed_len = if is_rewritten {
            read_result.bytes.len()
        } else {
            complete_jsonl_prefix_len(&read_result.bytes)
        };
        if parsed_len == 0 {
            continue;
        }
        let mut batch = adapter.parse(&ctx, &read_result.bytes[..parsed_len]);
        if file.agent == AgentId::Antigravity
            && let Some(owner) = agy.owner(extract_conversation_id(&path_str))
        {
            for session in &mut batch.sessions {
                session.surface = owner;
            }
        }
        let cwds = resolve_batch_projects(&mut batch);
        if repair_chat_format {
            db.repair_chat_contents(&batch.chat_events)?;
            db.repair_session_metadata(&batch.projects, &batch.sessions)?;
            record_cwds(db, &cwds);
            continue;
        }
        if is_rewritten
            && batch.parse_errors > 0
            && batch.usage_records.is_empty()
            && batch.chat_events.is_empty()
        {
            continue;
        }
        total_records_ingested += batch.usage_records.len() as u64;

        let new_offset = read_result.effective_start_offset + parsed_len as u64;
        let new_checkpoint = SourceCheckpoint {
            file_path: path_str.clone(),
            last_read_offset: new_offset,
            file_size: read_result.file_size,
            fingerprint: file_fingerprint
                .unwrap_or_else(|| format!("{}:{}", read_result.file_size, new_offset)),
            last_ingested_at: Utc::now(),
        };

        if let Err(e) = db.commit_batch_with_chat(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &new_checkpoint,
            &batch.chat_events,
        ) {
            tracing::warn!("failed to commit batch for {path_str}: {e}");
        } else {
            record_cwds(db, &cwds);
        }
        import_native_titles(db, file.agent, &read_result.bytes[..parsed_len])?;
    }

    if repair_chat_format {
        if repair_complete {
            db.finish_chat_format_repair()?;
        }
        return ingest_files(db, roots, discovered, false);
    }
    refresh_native_titles(db, roots)?;
    agy.repair_surfaces(db)?;
    total_records_ingested += antigravity::ingest_trajectories(db, &agy)?;

    let detected_subs = detect_local_subscriptions();
    let _ = db.auto_populate_subscriptions(
        detected_subs.anthropic.map(|(p, _)| p),
        detected_subs.google.map(|(p, _)| p),
    );

    Ok(total_records_ingested)
}

fn import_native_titles(db: &Database, agent: AgentId, bytes: &[u8]) -> Result<(), Error> {
    for (id, title) in gnomcode_adapters::titles::native_session_titles(agent, bytes) {
        db.update_native_session_title(&id, &title)?;
    }
    Ok(())
}

fn refresh_native_titles(db: &Database, roots: &[AgentRoot]) -> Result<(), Error> {
    for root in roots.iter().filter(|root| root.exists) {
        let paths = match root.agent {
            AgentId::Codex => root
                .path
                .parent()
                .map(|parent| vec![parent.join(obfstr::obfstr!("session_index.jsonl"))])
                .unwrap_or_default(),
            AgentId::ClaudeCode => fs::read_dir(&root.path)
                .into_iter()
                .flatten()
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.path().join(obfstr::obfstr!("sessions-index.json")))
                .collect(),
            _ => Vec::new(),
        };
        for path in paths {
            if !fs::metadata(&path).is_ok_and(|metadata| metadata.len() <= 8 * 1024 * 1024) {
                continue;
            }
            if let Ok(read) = read_from_offset(&path, 0) {
                import_native_titles(db, root.agent, &read.bytes)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod antigravity_tests;
#[cfg(test)]
mod project_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn legacy_markdown_repairs_from_native_source_without_rewinding_checkpoint() {
        const CHILD: &str = "GNOMCODE_TEST_CHAT_REPAIR";
        if std::env::var_os(CHILD).is_none() {
            let root = tempfile::tempdir().expect("root");
            let status=std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args(["--exact","tests::legacy_markdown_repairs_from_native_source_without_rewinding_checkpoint"])
                .env(CHILD,root.path()).env("HOME",root.path()).env("USERPROFILE",root.path())
                .env("APPDATA",root.path()).env("LOCALAPPDATA",root.path()).env("XDG_CONFIG_HOME",root.path()).env("XDG_DATA_HOME",root.path())
                .env("CODEX_HOME",root.path().join("codex")).env("CLAUDE_CONFIG_DIR",root.path().join("claude")).env("GEMINI_HOME",root.path().join("gemini"))
                .status().expect("isolated test");
            assert!(status.success());
            return;
        }
        use super::*;
        let root = PathBuf::from(std::env::var_os(CHILD).expect("root"));
        let directory = root.join("codex").join("sessions");
        fs::create_dir_all(&directory).expect("sessions");
        let path = directory.join("rollout-format.jsonl");
        let markdown = "# Result\n\n- one\n- two\n\n```rust\nlet x = 1;\n```";
        let bytes = format!(
            "{}\n{}\n",
            serde_json::json!({"type":"session_meta","timestamp":"2026-11-02T00:00:00Z","payload":{"id":"format","cwd":"/workspace"}}),
            serde_json::json!({"type":"response_item","timestamp":"2026-11-02T00:00:01Z","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":markdown}]}})
        );
        fs::write(&path, &bytes).expect("native log");
        let context = FileCtx {
            source_path: path.to_string_lossy().into_owned(),
            ..FileCtx::default()
        };
        let mut batch = CodexAdapter::new().parse(&context, bytes.as_bytes());
        let expected_project = batch.sessions[0].project_id.clone();
        let original_project = batch.projects[0].clone();
        batch.sessions[0].project_id =
            gnomcode_core::Project::from_origin(None, "/unknown", None).id;
        batch
            .projects
            .push(gnomcode_core::Project::from_origin(None, "/unknown", None));
        batch.chat_events[0].content = markdown.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut db = Database::open_in_memory().expect("db");
        let checkpoint = SourceCheckpoint {
            file_path: context.source_path.clone(),
            last_read_offset: bytes.len() as u64,
            file_size: bytes.len() as u64,
            fingerprint: "original".into(),
            last_ingested_at: Utc::now(),
        };
        db.commit_batch_with_chat(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &checkpoint,
            &batch.chat_events,
        )
        .expect("legacy chat");
        let event_id = batch.chat_events[0].event_id.clone();
        let fresh = directory.join("rollout-fresh.jsonl");
        fs::write(
            &fresh,
            bytes.replace("\"id\":\"format\"", "\"id\":\"fresh\""),
        )
        .expect("new log");
        let roots = discover_agent_roots();
        let discovered = scan_log_files(&roots);
        let unavailable = root.join("temporarily-unavailable.jsonl");
        fs::rename(&path, &unavailable).expect("source disappears after discovery");
        ingest_files(&mut db, &roots, &discovered, true)
            .expect("continue despite unreadable source");
        assert!(db.chat_format_needs_repair().expect("repair pending"));
        assert_eq!(
            db.get_session_chat("codex:fresh", None)
                .expect("normal ingestion")
                .len(),
            1
        );
        assert_eq!(
            db.get_session_chat("codex:format", None)
                .expect("legacy chat")[0]
                .content,
            batch.chat_events[0].content
        );
        fs::rename(unavailable, &path).expect("source becomes readable");
        ingest_all(&mut db).expect("repair");
        let chat = db.get_session_chat("codex:format", None).expect("chat");
        assert_eq!(chat[0].content, markdown);
        assert_eq!(
            db.get_session_focus_target("codex:format")
                .expect("focus")
                .expect("session")
                .repo_root
                .as_deref(),
            Some(original_project.repo_root.as_str())
        );
        assert!(!expected_project.is_empty());
        assert_eq!(chat[0].event_id, event_id);
        assert_eq!(
            db.get_checkpoint(&context.source_path)
                .expect("checkpoint")
                .expect("source")
                .fingerprint,
            "original"
        );
        assert!(!db.chat_format_needs_repair().expect("finished"));
        assert_eq!(fs::read_to_string(path).expect("untouched source"), bytes);
    }
    #[test]
    fn native_index_renames_refresh_without_reingesting_or_touching_activity() {
        use super::*;
        let batch = CodexAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl"),
        );
        let mut db = Database::open_in_memory().expect("database");
        db.commit_batch_with_chat(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &SourceCheckpoint {
                file_path: "fixture".into(),
                last_read_offset: 1,
                file_size: 1,
                fingerprint: "unchanged".into(),
                last_ingested_at: Utc::now(),
            },
            &batch.chat_events,
        )
        .expect("fixture");
        let source = &batch.sessions[0];
        let dir = tempfile::tempdir().expect("temp");
        let sessions = dir.path().join("sessions");
        fs::create_dir(&sessions).expect("sessions directory");
        let roots = [AgentRoot {
            agent: AgentId::Codex,
            path: sessions,
            exists: true,
        }];
        let raw_id = source.id.strip_prefix("codex:").expect("native id");
        for name in ["Original native title", "Renamed in the app"] {
            fs::write(
                dir.path().join("session_index.jsonl"),
                format!(r#"{{"id":"{raw_id}","thread_name":"{name}"}}"#),
            )
            .expect("index");
            refresh_native_titles(&db, &roots).expect("refresh names");
            let stored = db.list_active_sessions(10).expect("sessions");
            assert_eq!(stored[0].title.as_deref(), Some(name));
            assert_eq!(stored[0].last_event_at, source.last_event_at);
            assert_eq!(
                db.get_checkpoint("fixture")
                    .expect("checkpoint")
                    .expect("present")
                    .fingerprint,
                "unchanged"
            );
        }
    }
    #[test]
    fn codex_chat_batch_persists_and_replays_without_duplicates() {
        use gnomcode_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), bytes);
        let mut db = gnomcode_store::Database::open_in_memory().expect("database");
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "synthetic".into(),
            last_read_offset: bytes.len() as u64,
            file_size: bytes.len() as u64,
            fingerprint: "synthetic".into(),
            last_ingested_at: chrono::Utc::now(),
        };
        for _ in 0..2 {
            db.commit_batch_with_chat(
                &batch.projects,
                &batch.sessions,
                &batch.usage_records,
                &batch.activity_events,
                &batch.quota_snapshots,
                &checkpoint,
                &batch.chat_events,
            )
            .expect("commit");
        }
        let chat = db.get_session_chat("codex:id_0001", None).expect("chat");
        assert_eq!(chat.len(), 2);
        assert_eq!(chat[0].role, gnomcode_core::ChatRole::User);
        assert_eq!(chat[1].role, gnomcode_core::ChatRole::Assistant);
    }
    #[test]
    fn deleted_chat_stays_deleted_after_source_replay() {
        use gnomcode_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), bytes);
        let mut db = gnomcode_store::Database::open_in_memory().expect("database");
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "synthetic".into(),
            last_read_offset: bytes.len() as u64,
            file_size: bytes.len() as u64,
            fingerprint: "synthetic".into(),
            last_ingested_at: chrono::Utc::now(),
        };
        db.commit_batch_with_chat(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &checkpoint,
            &batch.chat_events,
        )
        .expect("first import");
        db.delete_session_chat("codex:id_0001").expect("delete");
        db.commit_batch_with_chat(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &checkpoint,
            &batch.chat_events,
        )
        .expect("source replay");
        assert!(
            db.get_session_chat("codex:id_0001", None)
                .expect("chat")
                .is_empty()
        );
    }

    use super::{complete_jsonl_prefix_len, scan_log_files};
    use gnomcode_core::AgentId;
    use gnomcode_platform::AgentRoot;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_log_files_filters_proper_extensions_and_names() {
        let dir = tempdir().expect("temp dir");
        let claude_dir = dir.path().join("claude");
        fs::create_dir_all(&claude_dir).expect("create claude dir");
        fs::write(claude_dir.join("session-1.jsonl"), b"{}").expect("write claude");
        fs::write(claude_dir.join("ignore.txt"), b"").expect("write ignore");

        let roots = [
            AgentRoot {
                agent: AgentId::ClaudeCode,
                path: claude_dir,
                exists: true,
            },
            AgentRoot {
                agent: AgentId::GeminiCli,
                path: dir.path().join("gemini"),
                exists: true,
            },
        ];
        let gemini_dir = dir.path().join("gemini");
        fs::create_dir_all(&gemini_dir).expect("create gemini dir");
        fs::write(gemini_dir.join("session.json"), b"{}").expect("write gemini");

        let files = scan_log_files(&roots);
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.agent == AgentId::ClaudeCode));
        assert!(files.iter().any(|f| f.agent == AgentId::GeminiCli));

        let core_only = super::scan_log_files_with_extra(&roots, false);
        assert_eq!(core_only.len(), 1);
        assert!(core_only.iter().any(|f| f.agent == AgentId::ClaudeCode));
        assert!(!core_only.iter().any(|f| f.agent == AgentId::GeminiCli));
    }

    #[test]
    fn antigravity_scan_reads_only_fixed_transcript_paths() {
        let dir = tempdir().expect("temp dir");
        let logs = dir
            .path()
            .join("conv")
            .join(".system_generated")
            .join("logs");
        fs::create_dir_all(&logs).expect("logs");
        fs::write(logs.join("transcript.jsonl"), b"{}").expect("transcript");
        fs::write(dir.path().join("conv").join("artifact.jsonl"), b"{}").expect("artifact");
        let files = scan_log_files(&[AgentRoot {
            agent: AgentId::Antigravity,
            path: dir.path().to_path_buf(),
            exists: true,
        }]);
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("transcript.jsonl"));
    }

    #[test]
    fn archived_codex_rollouts_are_discovered_and_decompressed() {
        let name = "rollout-2026-10-06T18-57-03-01a11180-e7a8-7d12-9e69-9f6fadc2399b.jsonl.zst";
        assert!(super::is_agent_log_file(
            AgentId::Codex,
            std::path::Path::new(name)
        ));
        assert_eq!(
            gnomcode_adapters::codex::codex_session_id_from_path(name),
            "01a11180-e7a8-7d12-9e69-9f6fadc2399b"
        );
        let plain = b"{\"type\":\"session_meta\"}
"
        .repeat(100);
        let mut packed = Vec::new();
        ruzstd::encoding::compress(
            &plain[..],
            &mut packed,
            ruzstd::encoding::CompressionLevel::Fastest,
        );
        assert_eq!(
            super::decompress_archive(&packed).as_deref(),
            Some(&plain[..])
        );
        assert!(super::decompress_archive(b"not zstd").is_none());
    }

    #[test]
    fn checkpoint_waits_for_complete_jsonl_line() {
        assert_eq!(complete_jsonl_prefix_len(b"{\"a\":1}\n{\"b\":"), 8);
        assert_eq!(complete_jsonl_prefix_len(b"{\"b\":2}\r\n"), 9);
        assert_eq!(complete_jsonl_prefix_len(b"{\"a\":"), 0);
    }
}
