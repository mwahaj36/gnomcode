//! File watching, reconciliation, incremental tailing, and checkpointing
//! of agent logs into the SQLite store (`SPEC §3.5`, `§4.4`).

use chrono::Utc;
use fleethud_adapters::{
    FileCtx, SourceAdapter,
    aider::AiderAdapter,
    antigravity::{AntigravityAdapter, extract_conversation_id},
    claude::ClaudeAdapter,
    cline::ClineAdapter,
    codex::{CodexAdapter, codex_session_id_from_path},
    cursor::CursorAdapter,
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
use std::time::UNIX_EPOCH;
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
        if file_fingerprint.as_ref().is_some_and(|fingerprint| {
            checkpoint
                .as_ref()
                .is_some_and(|cp| &cp.fingerprint == fingerprint)
        }) {
            continue;
        }

        let read_offset = if is_rewritten { 0 } else { start_offset };
        let Ok(read_result) = read_from_offset(&file.path, read_offset) else {
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
        if is_rewritten && batch.parse_errors > 0 && batch.usage_records.is_empty() {
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
    use super::{complete_jsonl_prefix_len, scan_log_files};
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
