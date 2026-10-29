//! Per-agent log parsers that turn raw bytes into normalized events. Pure and deterministic.
//!
//! See `docs/SPEC.md` §3.5 and §4.2 for the adapter architecture and invariants.

pub mod aider;
pub mod antigravity;
pub mod claude;
pub mod cline;
pub mod codex;
pub mod cursor;
pub mod gemini;

use serde::{Deserialize, Serialize};
use spawnt_core::{ActivityEvent, AgentId, Project, QuotaSnapshot, Session, UsageRecord};

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
    /// Byte position of the first byte in this chunk within the source file.
    pub source_offset: u64,
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
    /// Redacted user and assistant conversation events for local continuity.
    pub chat_events: Vec<spawnt_core::ChatEvent>,
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

    /// Parses a raw byte chunk into normalized Spawnt records.
    ///
    /// Must never panic or fail on malformed, truncated, or unknown-schema input:
    /// skip bad lines, increment `parse_errors`, and continue (`AGENTS.md` §5).
    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch;
}

#[cfg(test)]
mod tests {
    #[test]
    fn codex_injected_context_is_not_a_user_turn() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let batch = CodexAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/codex/v0.165/injected_context.jsonl"),
        );
        assert!(
            batch
                .chat_events
                .iter()
                .all(|event| !event.content.contains("environment_context")
                    && !event.content.contains("AGENTS.md"))
        );
        assert!(batch.chat_events.iter().any(
            |event| event.role == spawnt_core::ChatRole::User && event.content == "[REDACTED]"
        ));
        assert_eq!(
            batch
                .chat_events
                .iter()
                .filter(|event| event.role == spawnt_core::ChatRole::User)
                .count(),
            1
        );
    }

    #[test]
    fn codex_legacy_context_cleanup_preserves_prompts_and_cursor_ids() {
        use spawnt_core::{ChatEvent, ChatRole};
        let event = |id: &str, text: &str| ChatEvent {
            event_id: id.into(),
            session_id: "codex:legacy".into(),
            turn_index: 1,
            role: ChatRole::User,
            content: text.into(),
            tool_name: None,
            status: None,
            created_at: chrono::Utc::now(),
        };
        let cleaned = super::codex::clean_stored_conversation(vec![
            event(
                "context",
                "# AGENTS.md instructions for sample\n<INSTRUCTIONS>internal</INSTRUCTIONS>\n<environment_context>internal</environment_context>",
            ),
            event(
                "mixed",
                "<environment_context>internal</environment_context>\nHello",
            ),
            event("literal", "Explain the <environment_context> tag"),
        ]);
        assert_eq!(cleaned.len(), 3);
        assert_eq!(cleaned[0].event_id, "context");
        assert!(cleaned[0].content.is_empty());
        assert_eq!(cleaned[1].content, "Hello");
        assert_eq!(cleaned[2].content, "Explain the <environment_context> tag");
    }

    #[test]
    fn codex_turn_model_changes_emit_a_durable_notice() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = concat!(
            "{\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6.1-sol\"}}\n",
            "{\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6.1-sol\"}}\n",
            "{\"type\":\"turn_context\",\"payload\":{\"model\":\"o3\"}}\n",
        );
        let events = CodexAdapter::new()
            .parse(&FileCtx::default(), bytes.as_bytes())
            .chat_events;
        let changes: Vec<_> = events
            .iter()
            .filter(|event| event.status.as_deref() == Some("model_changed"))
            .collect();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].content, "Model changed from gpt-6.1-sol to o3");
        let resumed = CodexAdapter::new().parse(
            &FileCtx {
                source_offset: 100,
                fallback_model: Some("gpt-6.1-sol".into()),
                ..FileCtx::default()
            },
            b"{\"type\":\"turn_context\",\"payload\":{\"model\":\"o3\"}}\n",
        );
        assert_eq!(
            resumed.chat_events[0].content,
            "Model changed from gpt-6.1-sol to o3"
        );
    }
    #[test]
    fn codex_conversation_fixture_emits_only_user_and_assistant_messages() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let bytes = include_bytes!("../../../fixtures/codex/v0.162/conversation_sample.jsonl");
        let context = FileCtx::default();
        let first = CodexAdapter::new().parse(&context, bytes);
        let second = CodexAdapter::new().parse(&context, bytes);
        assert_eq!(first.chat_events.len(), 2);
        assert_eq!(first.chat_events, second.chat_events);
        assert_eq!(first.chat_events[0].role, spawnt_core::ChatRole::User);
        assert_eq!(first.chat_events[1].role, spawnt_core::ChatRole::Assistant);
        let split = bytes.iter().position(|b| *b == b'\n').expect("newline") + 1;
        let mut adapter = CodexAdapter::new();
        let _ = adapter.parse(&context, &bytes[..split]);
        let resumed = adapter.parse(
            &FileCtx {
                source_offset: split as u64,
                ..context
            },
            &bytes[split..],
        );
        assert_eq!(first.chat_events, resumed.chat_events);
    }

    #[test]
    fn codex_chat_ids_are_unique_without_ordinals() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let lines = concat!(
            "{\"timestamp\":\"2026-01-01T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"same-second\"}}\n",
            "{\"timestamp\":\"2026-01-01T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"first\"}]}}\n",
            "{\"timestamp\":\"2026-01-01T00:00:01Z\",\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"second\"}]}}\n",
        );
        let context = FileCtx::default();
        let events = CodexAdapter::new()
            .parse(&context, lines.as_bytes())
            .chat_events;
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].event_id, events[1].event_id);
        assert!(events[0].turn_index < events[1].turn_index);
        let mut prefixed = b"\xff\n".to_vec();
        prefixed.extend_from_slice(lines.as_bytes());
        let shifted = CodexAdapter::new().parse(&context, &prefixed).chat_events;
        assert_eq!(shifted[0].turn_index, events[0].turn_index + 2);
    }

    #[test]
    fn claude_conversation_fixture_excludes_tool_results() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let bytes = include_bytes!("../../../fixtures/claude-code/v2.1/session_sample.jsonl");
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.chat_events.len(), 2);
        assert_eq!(batch.chat_events[0].role, spawnt_core::ChatRole::User);
        assert_eq!(batch.chat_events[1].role, spawnt_core::ChatRole::Assistant);
        assert!(
            batch
                .chat_events
                .iter()
                .all(|event| event.content == "[REDACTED]")
        );
    }

    #[test]
    fn gemini_rewritten_chat_keeps_only_user_and_model_text() {
        use super::{FileCtx, SourceAdapter, gemini::GeminiAdapter};
        let bytes = br#"{"sessionId":"gem-chat","messages":[{"id":"u1","type":"user","content":"Question"},{"id":"m1","type":"gemini","content":"Answer","toolCalls":[{"name":"run_shell_command","args":{"command":"secret"}}]}]}"#;
        let batch = GeminiAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.chat_events.len(), 2);
        assert_eq!(batch.chat_events[0].content, "Question");
        assert_eq!(batch.chat_events[1].content, "Answer");
        assert!(
            batch
                .chat_events
                .iter()
                .all(|event| !event.content.contains("secret"))
        );
    }

    #[test]
    fn antigravity_chat_skips_settings_and_tool_outputs() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let bytes = concat!(
            "{\"step_index\":0,\"type\":\"USER_INPUT\",\"content\":\"<USER_SETTINGS_CHANGE>model</USER_SETTINGS_CHANGE>\"}\n",
            "{\"step_index\":1,\"type\":\"USER_INPUT\",\"content\":\"<USER_REQUEST>\\nQuestion\\n</USER_REQUEST>\\n<ADDITIONAL_METADATA>meta</ADDITIONAL_METADATA>\"}\n",
            "{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"GENERIC\",\"content\":\"Answer\"}\n",
            "{\"step_index\":3,\"source\":\"TOOL\",\"type\":\"GENERIC\",\"content\":\"secret output\"}\n",
            "{\"step_index\":4,\"source\":\"MODEL\",\"type\":\"GENERIC\",\"content\":\"Created At: 2026-10-29\\nCommand output:\\ntool output\"}\n",
        );
        let batch = AntigravityAdapter::new().parse(&FileCtx::default(), bytes.as_bytes());
        assert_eq!(batch.chat_events.len(), 2);
        assert_eq!(batch.chat_events[0].content, "Question");
        assert_eq!(batch.chat_events[1].content, "Answer");
        let mut legacy = batch.chat_events.clone();
        legacy[0].content =
            "<USER_REQUEST>Question</USER_REQUEST><ADDITIONAL_METADATA>noise</ADDITIONAL_METADATA>"
                .into();
        legacy[1].content = "Created At: yesterday\nCommand output: private output".into();
        let cleaned = super::antigravity::clean_stored_conversation(legacy);
        assert_eq!(
            cleaned.len(),
            2,
            "cursor pages must retain their original size"
        );
        assert_eq!(cleaned[0].content, "Question");
        assert_eq!(cleaned[1].role, spawnt_core::ChatRole::Tool);
        assert!(cleaned[1].content.is_empty());
    }

    #[test]
    fn cursor_chat_keeps_user_and_assistant_text_only() {
        use super::{FileCtx, SourceAdapter, cursor::CursorAdapter};
        let bytes = br#"{"sessionId":"cursor-chat","messages":[{"role":"user","content":"Question"},{"role":"assistant","content":"Answer","tool_calls":[{"name":"Shell","output":"secret"}]},{"role":"tool","content":"secret output"}]}"#;
        let batch = CursorAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.chat_events.len(), 2);
        assert_eq!(batch.chat_events[0].content, "Question");
        assert_eq!(batch.chat_events[1].content, "Answer");
    }

    #[test]
    fn cline_chat_keeps_task_feedback_and_answer_without_tools() {
        use super::{FileCtx, SourceAdapter, cline::ClineAdapter};
        let bytes = br#"[{"ts":1,"type":"say","say":"task","text":"Question"},{"ts":2,"type":"say","say":"text","text":"Answer"},{"ts":3,"type":"say","say":"tool","text":"secret tool output"},{"ts":4,"type":"say","say":"user_feedback","text":"Follow up"}]"#;
        let batch = ClineAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.chat_events.len(), 3);
        assert_eq!(batch.chat_events[0].content, "Question");
        assert_eq!(batch.chat_events[1].content, "Answer");
        assert_eq!(batch.chat_events[2].content, "Follow up");
    }
    use super::{
        FileCtx, SourceAdapter, antigravity::AntigravityAdapter, claude::ClaudeAdapter,
        codex::CodexAdapter, gemini::GeminiAdapter,
    };
    use spawnt_core::{AgentId, Surface, TrustLevel};
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
        assert_eq!(u.cost_basis, TrustLevel::Estimated);
        assert!(!batch.activity_events.is_empty());
    }

    #[test]
    fn claude_end_turn_is_idle() {
        let lines = concat!(
            "{\"timestamp\":\"2026-10-26T10:00:00Z\",\"type\":\"assistant\",\"sessionId\":\"s1\",\"cwd\":\"/workspace/test\",\"message\":{\"model\":\"claude-sonnet-5\",\"stop_reason\":\"tool_use\",\"content\":[{\"type\":\"tool_use\",\"name\":\"Bash\",\"input\":{}}]}}\n",
            "{\"timestamp\":\"2026-10-26T10:00:01Z\",\"type\":\"assistant\",\"sessionId\":\"s1\",\"cwd\":\"/workspace/test\",\"message\":{\"model\":\"claude-sonnet-5\",\"stop_reason\":\"end_turn\",\"content\":[]}}\n",
        );
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), lines.as_bytes());
        assert_eq!(batch.sessions[0].state, spawnt_core::SessionState::Idle);
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
    fn codex_context_window_uses_current_usage_after_compaction() {
        let path = workspace_root().join("fixtures/codex/v0.162/compaction_context.jsonl");
        let bytes = fs::read(path).expect("read Codex compaction fixture");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), &bytes);

        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.usage_records.len(), 2);
        assert_eq!(batch.usage_records[0].context_tokens, 190_000);
        assert_eq!(batch.usage_records[1].context_tokens, 20_000);
        assert_eq!(batch.usage_records[1].context_window, Some(258_000));
    }

    #[test]
    fn codex_task_complete_ends_thinking_state() {
        let lines = concat!(
            "{\"timestamp\":\"2026-10-26T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"s1\",\"cwd\":\"/workspace/test\"}}\n",
            "{\"timestamp\":\"2026-10-26T10:00:01Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n",
        );
        let mut adapter = CodexAdapter::new();
        let batch = adapter.parse(&FileCtx::default(), lines.as_bytes());
        assert_eq!(batch.sessions[0].state, spawnt_core::SessionState::Idle);
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
        assert_eq!(batch.sessions[0].state, spawnt_core::SessionState::Idle);
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

    #[test]
    fn cline_adapter_parses_ui_messages_and_approval() {
        use super::cline::ClineAdapter;
        let mut adapter = ClineAdapter::new();
        let ctx = FileCtx {
            source_path: "/home/dev/.config/Code/User/globalStorage/saoudrizwan.claude-dev/tasks/task-123/ui_messages.json".to_owned(),
            ..FileCtx::default()
        };
        let sample = br#"[
            {"ts": 1729900000000, "type": "say", "say": "task", "text": "start task"},
            {"ts": 1729900010000, "type": "say", "say": "api_req_started", "text": "{\"tokensIn\":1000,\"tokensOut\":200,\"cost\":0.01,\"modelId\":\"claude-3-5-sonnet\"}"},
            {"ts": 1729900020000, "type": "ask", "ask": "tool", "text": "execute command"}
        ]"#;
        let batch = adapter.parse(&ctx, sample);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.sessions.len(), 1);
        assert_eq!(
            batch.sessions[0].state,
            spawnt_core::SessionState::AwaitingApproval
        );
        assert_eq!(batch.usage_records.len(), 1);
        assert_eq!(batch.usage_records[0].input_tokens, 1000);
        assert_eq!(batch.usage_records[0].cost_usd, 0.01);
    }

    #[test]
    fn cursor_and_aider_adapters_parse_cleanly() {
        use super::aider::AiderAdapter;
        use super::cursor::CursorAdapter;

        let mut cursor = CursorAdapter::new();
        let c_ctx = FileCtx {
            source_path: "/workspace/.cursor/transcripts/sess-abc.json".to_owned(),
            ..FileCtx::default()
        };
        let c_sample = br#"{
            "sessionId": "sess-abc",
            "messages": [
                {"role": "user", "timestamp": "2026-10-26T12:00:00Z"},
                {"role": "assistant", "timestamp": "2026-10-26T12:00:05Z", "model": "gpt-4o", "usage": {"prompt_tokens": 500, "completion_tokens": 150}}
            ]
        }"#;
        let c_batch = cursor.parse(&c_ctx, c_sample);
        assert_eq!(c_batch.parse_errors, 0);
        assert_eq!(c_batch.sessions.len(), 1);
        assert_eq!(c_batch.sessions[0].state, spawnt_core::SessionState::Idle);
        assert_eq!(c_batch.usage_records.len(), 1);

        let mut aider = AiderAdapter::new();
        let a_ctx = FileCtx {
            source_path: "/workspace/.aider.analytics.jsonl".to_owned(),
            ..FileCtx::default()
        };
        let a_sample = b"{\"timestamp\":\"2026-10-26T12:00:00Z\",\"model\":\"claude-3-5-sonnet\",\"tokens_sent\":800,\"tokens_received\":250,\"cost\":0.005,\"event\":\"chat\"}\n";
        let a_batch = aider.parse(&a_ctx, a_sample);
        assert_eq!(a_batch.parse_errors, 0);
        assert_eq!(a_batch.sessions.len(), 1);
        assert_eq!(a_batch.sessions[0].state, spawnt_core::SessionState::Idle);
        assert_eq!(a_batch.usage_records.len(), 1);
        assert_eq!(a_batch.usage_records[0].cost_usd, 0.005);
    }
}
