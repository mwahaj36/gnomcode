//! File watching, reconciliation, incremental tailing, and checkpointing
//! of agent logs into the SQLite store (`SPEC §3.5`, `§4.4`).

use chrono::Utc;
use fleethud_adapters::{
    FileCtx, SourceAdapter,
    antigravity::{AntigravityAdapter, extract_conversation_id},
    claude::ClaudeAdapter,
    codex::{CodexAdapter, codex_session_id_from_path},
    gemini::GeminiAdapter,
};
use fleethud_core::{AgentId, Session};
use fleethud_platform::{
    AgentRoot, detect_local_subscriptions, discover_agent_roots, read_antigravity_fallback_model,
    read_from_offset,
};
use fleethud_store::{Database, SourceCheckpoint};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Ingestion engine error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Platform reading error.
    #[error("platform error: {0}")]
    Platform(#[from] fleethud_platform::Error),
    /// Store operation error.
    #[error("store error: {0}")]
    Store(#[from] fleethud_store::Error),
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
        collect_files_recursively(root.agent, &root.path, &mut files);
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
        AgentId::Cline | AgentId::Cursor | AgentId::Aider => false,
    }
}

/// Ingests all available agent logs across discovered roots in one synchronous pass.
///
/// # Errors
/// Returns [`Error`] if database commit or reading fails.
pub fn ingest_all(db: &mut Database) -> Result<u64, Error> {
    let roots = discover_agent_roots();
    let discovered = scan_log_files(&roots);
    let mut total_records_ingested = 0;

    let agy_fallback_model = read_antigravity_fallback_model();

    for file in discovered {
        let path_str = file.path.to_string_lossy().into_owned();
        let checkpoint = match db.get_checkpoint(&path_str) {
            Ok(cp) => cp,
            Err(e) => {
                tracing::warn!("skipping checkpoint lookup for {path_str}: {e}");
                continue;
            }
        };
        let start_offset = checkpoint.as_ref().map_or(0, |c| c.last_read_offset);

        let Ok(read_result) = read_from_offset(&file.path, start_offset) else {
            continue;
        };
        if read_result.bytes.is_empty() && start_offset == read_result.file_size {
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

        let ctx = FileCtx {
            source_path: path_str.clone(),
            fallback_model,
            fallback_cwd: None,
        };

        let mut adapter: Box<dyn SourceAdapter> = match file.agent {
            AgentId::ClaudeCode => Box::new(ClaudeAdapter::new()),
            AgentId::Codex => Box::new(CodexAdapter::new()),
            AgentId::Antigravity => Box::new(AntigravityAdapter::new()),
            AgentId::GeminiCli => Box::new(GeminiAdapter::new()),
            AgentId::Cline | AgentId::Cursor | AgentId::Aider => continue,
        };

        let batch = adapter.parse(&ctx, &read_result.bytes);
        total_records_ingested += batch.usage_records.len() as u64;

        let new_offset = read_result.effective_start_offset + read_result.bytes.len() as u64;
        let new_checkpoint = SourceCheckpoint {
            file_path: path_str.clone(),
            last_read_offset: new_offset,
            file_size: read_result.file_size,
            fingerprint: format!("{}:{}", read_result.file_size, new_offset),
            last_ingested_at: Utc::now(),
        };

        if let Err(e) = db.commit_batch(
            &batch.projects,
            &batch.sessions,
            &batch.usage_records,
            &batch.activity_events,
            &batch.quota_snapshots,
            &new_checkpoint,
        ) {
            tracing::warn!("failed to commit batch for {path_str}: {e}");
        }
    }

    let detected_subs = detect_local_subscriptions();
    let _ = db.auto_populate_subscriptions(
        detected_subs.anthropic.map(|(p, _)| p),
        detected_subs.google.map(|(p, _)| p),
    );

    Ok(total_records_ingested)
}

#[cfg(test)]
mod tests {
    use super::scan_log_files;
    use fleethud_core::AgentId;
    use fleethud_platform::AgentRoot;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_log_files_filters_proper_extensions_and_names() {
        let dir = tempdir().expect("temp dir");
        let claude_dir = dir.path().join("claude");
        fs::create_dir_all(&claude_dir).expect("create claude dir");
        fs::write(claude_dir.join("session-1.jsonl"), b"{}").expect("write claude");
        fs::write(claude_dir.join("ignore.txt"), b"").expect("write ignore");

        let roots = [AgentRoot {
            agent: AgentId::ClaudeCode,
            path: claude_dir,
            exists: true,
        }];

        let files = scan_log_files(&roots);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].agent, AgentId::ClaudeCode);
        assert!(files[0].path.ends_with("session-1.jsonl"));
    }
}
