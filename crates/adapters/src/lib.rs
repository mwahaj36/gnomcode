//! Per-agent log parsers that turn raw bytes into normalized events. Pure and deterministic.
//!
//! See `docs/SPEC.md` §3.5 and §4.2 for the adapter architecture and invariants.

pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod gemini;

use fleethud_core::{ActivityEvent, AgentId, Project, QuotaSnapshot, Session, UsageRecord};
use serde::{Deserialize, Serialize};

/// How an agent writes its log files on disk (`SPEC §3.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    /// New lines are appended to the end of the file (`.jsonl`).
    AppendOnlyJsonl,
    /// The entire JSON file is rewritten on each turn (`.json`).
    RewrittenJson,
}

/// Context about the file being parsed, passed by `crates/ingest`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCtx {
    /// Logical path or filename of the log file (used to derive fallback IDs).
    pub source_path: String,
    /// Optional fallback model name (e.g. from `antigravity_state.pbtxt`).
    pub fallback_model: Option<String>,
    /// Optional fallback workspace path if the log file omits `cwd`.
    pub fallback_cwd: Option<String>,
}

/// Normalized batch of domain records produced by an adapter from a byte chunk.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ParsedBatch {
    /// Projects discovered or updated in this chunk.
    pub projects: Vec<Project>,
    /// Sessions discovered or updated in this chunk.
    pub sessions: Vec<Session>,
    /// Deduplicated per-turn token and cost records.
    pub usage_records: Vec<UsageRecord>,
    /// Redacted tool/file/command activity summaries.
    pub activity_events: Vec<ActivityEvent>,
    /// Per-model quota pool snapshots.
    pub quota_snapshots: Vec<QuotaSnapshot>,
    /// Count of malformed or unparseable lines/records skipped cleanly.
    pub parse_errors: u64,
}

/// Pure, deterministic log parser for a single AI coding agent (`SPEC §4.2`).
pub trait SourceAdapter {
    /// The agent identifier this adapter parses.
    fn agent_id(&self) -> AgentId;

    /// The file write pattern used by this agent.
    fn file_kind(&self) -> FileKind;

    /// Parses a raw byte chunk into normalized FleetHUD records.
    ///
    /// Must never panic or fail on malformed, truncated, or unknown-schema input:
    /// skip bad lines, increment `parse_errors`, and continue (`AGENTS.md` §5).
    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch;
}

#[cfg(test)]
mod tests {
    use super::{
        FileCtx, SourceAdapter, antigravity::AntigravityAdapter, claude::ClaudeAdapter,
        codex::CodexAdapter, gemini::GeminiAdapter,
    };
    use fleethud_core::{AgentId, Surface, TrustLevel};
    use std::fs;
    use std::path::PathBuf;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .to_path_buf()
    }

    #[test]
    fn golden_fixtures_are_valid_json_lines_and_scrubbed_of_pii() {
        let fixtures_dir = workspace_root().join("fixtures");
        let files = [
            fixtures_dir.join("claude-code/v2.1/session_sample.jsonl"),
            fixtures_dir.join("codex/v0.162/rollout_sample.jsonl"),
            fixtures_dir.join("antigravity/v2.0/transcript_sample.jsonl"),
        ];

        for file in &files {
            let content = fs::read_to_string(file).expect("fixture file exists");
            let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
            assert!(
                !lines.is_empty(),
                "fixture {} must not be empty",
                file.display()
            );

            for line in &lines {
                let parsed: serde_json::Value =
                    serde_json::from_str(line).expect("fixture line is valid JSON");
                assert!(parsed.is_object(), "fixture line must be a JSON object");
            }

            let lower = content.to_lowercase();
            for forbidden in ["wahaj", "c:\\users\\", "/users/wahaj"] {
                assert!(
                    !lower.contains(forbidden),
                    "fixture {} leaked forbidden substring '{}'",
                    file.display(),
                    forbidden
                );
            }
        }
    }

    #[test]
    fn claude_adapter_deduplicates_multi_block_assistant_turns() {
        let path = workspace_root().join("fixtures/claude-code/v2.1/session_sample.jsonl");
        let bytes = fs::read(&path).expect("read claude fixture");
        let mut adapter = ClaudeAdapter::new();
        let ctx = FileCtx {
            source_path: path.to_string_lossy().into_owned(),
            ..FileCtx::default()
        };

        let batch = adapter.parse(&ctx, &bytes);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.sessions.len(), 1);
        assert_eq!(batch.sessions[0].agent, AgentId::ClaudeCode);
        assert_eq!(batch.sessions[0].surface, Surface::Ide);
        // The fixture has 3 assistant lines with the same message.id + requestId ("id_0004:id_0005")
        assert_eq!(batch.usage_records.len(), 1);
        let u = &batch.usage_records[0];
        assert_eq!(u.dedup_key, "id_0004:id_0005");
        assert_eq!(u.model, "claude-opus-5");
        assert_eq!(u.input_tokens, 2);
        assert_eq!(u.cache_read_tokens, 27_349);
        assert_eq!(u.cache_write_1h_tokens, 8_505);
        assert_eq!(u.output_tokens, 282);
        assert_eq!(u.cost_basis, TrustLevel::Exact);
        assert!(!batch.activity_events.is_empty());
    }

    #[test]
    fn codex_adapter_parses_desktop_surface_deltas_and_quota_pools() {
        let path = workspace_root().join("fixtures/codex/v0.162/rollout_sample.jsonl");
        let bytes = fs::read(&path).expect("read codex fixture");
        let mut adapter = CodexAdapter::new();
        let ctx = FileCtx {
            source_path: path.to_string_lossy().into_owned(),
            ..FileCtx::default()
        };

        let batch = adapter.parse(&ctx, &bytes);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.sessions.len(), 1);
        assert_eq!(batch.sessions[0].agent, AgentId::Codex);
        assert_eq!(batch.sessions[0].surface, Surface::DesktopApp);
        assert_eq!(batch.projects.len(), 1);
        assert_eq!(
            batch.projects[0].canonical_key,
            "github.com/example/sample-project"
        );
        assert_eq!(batch.usage_records.len(), 3);
        // Check last_token_usage delta (not cumulative total_token_usage)
        assert_eq!(batch.usage_records[1].input_tokens, 33_662);
        assert_eq!(batch.usage_records[1].cache_read_tokens, 33_536);
        assert_eq!(batch.usage_records[1].context_window, Some(475_000));
        // Primary (5h) and secondary (7d) quota snapshots emitted for limit_id "codex"
        assert!(!batch.quota_snapshots.is_empty());
        assert!(batch.quota_snapshots.iter().any(|q| q.limit_id == "codex"));
    }

    #[test]
    fn antigravity_adapter_tracks_model_selection_transitions_and_steps() {
        let path = workspace_root().join("fixtures/antigravity/v2.0/transcript_sample.jsonl");
        let bytes = fs::read(&path).expect("read antigravity fixture");
        let mut adapter = AntigravityAdapter::new();
        let ctx = FileCtx {
            source_path: "/home/dev/.gemini/antigravity/brain/conv-999/.system_generated/logs/transcript.jsonl".to_owned(),
            fallback_cwd: Some("/workspace/sample-project".to_owned()),
            ..FileCtx::default()
        };

        let batch = adapter.parse(&ctx, &bytes);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.usage_records.len(), 1);
        assert_eq!(batch.usage_records[0].dedup_key, "conv-999:1");
        assert_eq!(batch.usage_records[0].model, "Claude Opus 5.5 (Medium)");
        // After step 94 USER_SETTINGS_CHANGE, session active model updated to Gemini 4 Argon (Medium)
        assert_eq!(
            batch.sessions[0].model.as_deref(),
            Some("Gemini 4 Argon (Medium)")
        );
    }

    #[test]
    fn gemini_adapter_parses_rewritten_session_json() {
        let path = workspace_root().join("fixtures/gemini-cli/v0.5/session_sample.json");
        let bytes = fs::read(&path).expect("read gemini fixture");
        let mut adapter = GeminiAdapter::new();
        let ctx = FileCtx {
            source_path: path.to_string_lossy().into_owned(),
            ..FileCtx::default()
        };

        let batch = adapter.parse(&ctx, &bytes);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.sessions.len(), 1);
        assert_eq!(batch.usage_records.len(), 1);
        assert_eq!(batch.usage_records[0].dedup_key, "gem-sess-0001:msg-0002");
        assert_eq!(batch.usage_records[0].model, "gemini-2.5-pro");
        assert_eq!(batch.usage_records[0].input_tokens, 12_400);
        assert_eq!(batch.usage_records[0].cache_read_tokens, 8_000);
    }

    #[test]
    fn adapters_tolerate_corrupted_lines_and_crlf_without_panicking() {
        let mut adapter = ClaudeAdapter::new();
        let ctx = FileCtx::default();
        let noisy = b"{not valid json\r\n{\"type\":\"unknown\"}\r\n{\"truncated\":123";
        let batch = adapter.parse(&ctx, noisy);
        assert_eq!(batch.parse_errors, 2);
        assert!(batch.usage_records.is_empty());
    }
}
