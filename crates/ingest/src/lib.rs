//! File watching, reconciliation, incremental tailing, and checkpointing
//! of agent logs into the SQLite store (`SPEC §3.5`, `§4.4`).

use chrono::Utc;
use spawnt_adapters::{
    FileCtx, SourceAdapter,
    aider::AiderAdapter,
    antigravity::{AntigravityAdapter, extract_conversation_id},
    claude::ClaudeAdapter,
    cline::ClineAdapter,
    codex::{CodexAdapter, codex_session_id_from_path},
    cursor::CursorAdapter,
    gemini::GeminiAdapter,
};
use spawnt_core::{AgentId, Session};
use spawnt_platform::{
    AgentRoot, detect_local_subscriptions, discover_agent_roots, read_antigravity_fallback_model,
    read_from_offset,
};
use spawnt_store::{Database, SourceCheckpoint};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use thiserror::Error;

/// Ingestion engine error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Platform reading error.
    #[error("platform error: {0}")]
    Platform(#[from] spawnt_platform::Error),
    /// Store operation error.
    #[error("store error: {0}")]
    Store(#[from] spawnt_store::Error),
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
    let mut files = Vec::new();
    for root in roots {
        if !root.exists {
            continue;
        }
        if root.path.is_file() {
            if is_agent_log_file(root.agent, &root.path) {
                files.push(DiscoveredFile {
                    agent: root.agent,
                    path: root.path.clone(),
                });
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
        }
        AgentId::Antigravity => {
            ext.eq_ignore_ascii_case("jsonl")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n == "transcript.jsonl")
        }
        AgentId::GeminiCli => ext.eq_ignore_ascii_case("json"),
        AgentId::Cline => {
            ext.eq_ignore_ascii_case("json")
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|n| n == "ui_messages.json")
        }
        AgentId::Cursor => ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("jsonl"),
        AgentId::Aider => ext.eq_ignore_ascii_case("jsonl") || ext.eq_ignore_ascii_case("json"),
    }
}

fn complete_jsonl_prefix_len(bytes: &[u8]) -> usize {
    bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(0, |i| i + 1)
}

/// Ingests all available agent logs across discovered roots in one synchronous pass.
///
/// # Errors
/// Returns [`Error`] if database commit or reading fails.
pub fn ingest_all(db: &mut Database) -> Result<u64, Error> {
    let roots = discover_agent_roots();
    let discovered = scan_log_files(&roots);
    let repair_chat_format = db.chat_format_needs_repair()?;
    ingest_files(db, &roots, &discovered, repair_chat_format)
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
        let is_rewritten = matches!(
            file.agent,
            AgentId::GeminiCli | AgentId::Cline | AgentId::Cursor
        );

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
        let Ok(read_result) = read_from_offset(&file.path, read_offset) else {
            repair_complete = false;
            continue;
        };
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
                db.get_session_model(&sid)
                    .or_else(|| agy_fallback_model.clone())
            }
            _ => None,
        };

        let fallback_cwd = if file.agent == AgentId::Antigravity {
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
        };

        let mut adapter: Box<dyn SourceAdapter> = match file.agent {
            AgentId::ClaudeCode => Box::new(ClaudeAdapter::new()),
            AgentId::Codex => Box::new(CodexAdapter::new()),
            AgentId::Antigravity => Box::new(AntigravityAdapter::new()),
            AgentId::GeminiCli => Box::new(GeminiAdapter::new()),
            AgentId::Cline => Box::new(ClineAdapter::new()),
            AgentId::Cursor => Box::new(CursorAdapter::new()),
            AgentId::Aider => Box::new(AiderAdapter::new()),
        };

        let parsed_len = if is_rewritten {
            read_result.bytes.len()
        } else {
            complete_jsonl_prefix_len(&read_result.bytes)
        };
        if parsed_len == 0 {
            continue;
        }
        let batch = adapter.parse(&ctx, &read_result.bytes[..parsed_len]);
        if repair_chat_format {
            db.repair_chat_contents(&batch.chat_events)?;
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

    let detected_subs = detect_local_subscriptions();
    let _ = db.auto_populate_subscriptions(
        detected_subs.anthropic.map(|(p, _)| p),
        detected_subs.google.map(|(p, _)| p),
    );

    Ok(total_records_ingested)
}

fn import_native_titles(db: &Database, agent: AgentId, bytes: &[u8]) -> Result<(), Error> {
    for (id, title) in spawnt_adapters::titles::native_session_titles(agent, bytes) {
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
mod tests {
    #[test]
    fn legacy_markdown_repairs_from_native_source_without_rewinding_checkpoint() {
        const CHILD: &str = "SPAWNT_TEST_CHAT_REPAIR";
        if std::env::var_os(CHILD).is_none() {
            let root = tempfile::tempdir().expect("root");
            let status=std::process::Command::new(std::env::current_exe().expect("test binary"))
                .args(["--exact","tests::legacy_markdown_repairs_from_native_source_without_rewinding_checkpoint"])
                .env(CHILD,root.path()).env("HOME",root.path()).env("USERPROFILE",root.path())
                .env("APPDATA",root.path()).env("LOCALAPPDATA",root.path()).env("XDG_CONFIG_HOME",root.path()).env("XDG_DATA_HOME",root.path())
                .env("CODEX_HOME",root.path().join("codex")).env("CLAUDE_CONFIG_DIR",root.path().join("claude")).env("GEMINI_HOME",root.path().join("gemini"))
                .env_remove("AIDER_ANALYTICS_LOG").status().expect("isolated test");
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
        use spawnt_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), bytes);
        let mut db = spawnt_store::Database::open_in_memory().expect("database");
        let checkpoint = spawnt_store::SourceCheckpoint {
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
        assert_eq!(chat[0].role, spawnt_core::ChatRole::User);
        assert_eq!(chat[1].role, spawnt_core::ChatRole::Assistant);
    }
    #[test]
    fn deleted_chat_stays_deleted_after_source_replay() {
        use spawnt_adapters::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), bytes);
        let mut db = spawnt_store::Database::open_in_memory().expect("database");
        let checkpoint = spawnt_store::SourceCheckpoint {
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
    use spawnt_core::AgentId;
    use spawnt_platform::AgentRoot;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_includes_an_explicit_aider_log_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("aider.jsonl");
        fs::write(&path, b"{}\n").expect("fixture");
        let roots = [AgentRoot {
            agent: AgentId::Aider,
            path: path.clone(),
            exists: true,
        }];
        let files = scan_log_files(&roots);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, path);
    }

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
                agent: AgentId::Cline,
                path: dir.path().join("cline"),
                exists: true,
            },
        ];
        let cline_dir = dir.path().join("cline");
        fs::create_dir_all(&cline_dir).expect("create cline dir");
        fs::write(cline_dir.join("ui_messages.json"), b"[]").expect("write cline");

        let files = scan_log_files(&roots);
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|f| f.agent == AgentId::ClaudeCode));
        assert!(files.iter().any(|f| f.agent == AgentId::Cline));
    }

    #[test]
    fn checkpoint_waits_for_complete_jsonl_line() {
        assert_eq!(complete_jsonl_prefix_len(b"{\"a\":1}\n{\"b\":"), 8);
        assert_eq!(complete_jsonl_prefix_len(b"{\"b\":2}\r\n"), 9);
        assert_eq!(complete_jsonl_prefix_len(b"{\"a\":"), 0);
    }
}
