//! Per-agent log parsers that turn raw bytes into normalized events. Pure and deterministic.
//!
//! See `docs/SPEC.md` §3.5 and §4.2 for the adapter architecture and invariants.

pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod titles;

use gnomcode_core::{ActivityEvent, AgentId, Project, QuotaSnapshot, Session, UsageRecord};
use serde::{Deserialize, Serialize};

/// How an agent writes its log files on disk (`SPEC §3.5`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileKind {
    /// New lines are appended to the end of the file (`.jsonl`).
    AppendOnlyJsonl,
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
    /// Last observed activity when parsing an incremental chunk.
    pub fallback_state: Option<gnomcode_core::SessionState>,
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
    pub chat_events: Vec<gnomcode_core::ChatEvent>,
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

    /// Parses a raw byte chunk into normalized Gnomcode records.
    ///
    /// Must never panic or fail on malformed, truncated, or unknown-schema input:
    /// skip bad lines, increment `parse_errors`, and continue (`AGENTS.md` §5).
    fn parse(&mut self, ctx: &FileCtx, chunk: &[u8]) -> ParsedBatch;
}

#[cfg(test)]
mod antigravity_tests;

#[cfg(test)]
mod tests {
    #[test]
    fn antigravity_cli_usage_has_api_pools_and_remaining_percentages() {
        let path = workspace_root()
            .join("fixtures")
            .join("antigravity")
            .join("cli-1.5.7")
            .join("usage.tsv");
        let text = std::fs::read_to_string(path).expect("anonymized fixture");
        let now = chrono::DateTime::parse_from_rfc3339("2026-11-13T00:00:00Z")
            .expect("time")
            .with_timezone(&chrono::Utc);
        let rows = super::antigravity::parse_cli_usage(&text, now);
        assert_eq!(rows.len(), 5);
        assert_eq!(
            rows.iter().map(|row| row.used_pct).collect::<Vec<_>>(),
            [63.0, 35.0, 0.0, 0.0, 100.0]
        );
        assert!(
            rows.iter()
                .all(|row| row.basis == gnomcode_core::TrustLevel::Api)
        );
        assert_eq!(
            super::antigravity::parse_cli_usage(&text.replace('\n', "\r\n"), now),
            rows
        );
        assert_eq!(
            super::antigravity::parse_cli_usage(&format!("{text}{text}"), now),
            rows
        );
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {insta::assert_json_snapshot!("antigravity_cli_usage", rows);});
    }

    #[test]
    fn antigravity_cli_usage_skips_bad_or_unknown_rows() {
        let now = chrono::Utc::now();
        for percent in ["NaN%", "inf%", "-1%", "101%", "20", "n/a"] {
            assert!(
                super::antigravity::parse_cli_usage(
                    &format!(
                        "Gemini Models\tWeekly Limit Remaining\t{percent}\t2026-11-20T00:00:00Z\n"
                    ),
                    now
                )
                .is_empty()
            );
        }
        for line in [
            "truncated\tWeekly Limit Remaining\t50%",
            "pool\tUnknown\t50%\t2026-11-20T00:00:00Z",
            "pool\tWeekly Limit Remaining\t50%\tnot a date",
            "\tWeekly Limit Remaining\t50%\t2026-11-20T00:00:00Z",
        ] {
            assert!(super::antigravity::parse_cli_usage(line, now).is_empty());
        }
    }
    #[test]
    fn claude_harness_entries_become_system_events() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let batch = ClaudeAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/claude-code/synthetic/harness-entries.jsonl"),
        );
        let users: Vec<_> = batch
            .chat_events
            .iter()
            .filter(|e| e.role == gnomcode_core::ChatRole::User)
            .map(|e| e.content.as_str())
            .collect();
        assert_eq!(users, ["Please add tests"]);
        assert!(
            batch
                .chat_events
                .iter()
                .all(|e| !e.content.contains("<task-notification>")
                    && !e.content.contains("<system-reminder>"))
        );
        let rows: Vec<_> = batch
            .chat_events
            .iter()
            .map(|e| (e.role.as_str(), e.status.clone(), e.content.clone()))
            .collect();
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {insta::assert_debug_snapshot!("claude_harness_entries", rows);});
    }
    #[test]
    fn claude_user_turn_does_not_inherit_the_previous_completion() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let batch = ClaudeAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/claude-code/2026-11-06/new-turn.jsonl"),
        );
        assert_eq!(
            batch.sessions[0].state,
            gnomcode_core::SessionState::Thinking
        );
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {insta::assert_debug_snapshot!("claude_new_turn",(batch.sessions,batch.chat_events));});
    }

    #[test]
    fn codex_progress_and_token_counts_do_not_finish_the_turn() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let mut adapter = CodexAdapter::new();
        let mut ctx = FileCtx::default();
        let bytes = include_bytes!("../../../fixtures/codex/2026-11-06/progress.jsonl");
        let batch = adapter.parse(&ctx, bytes);
        assert_eq!(
            batch.sessions[0].state,
            gnomcode_core::SessionState::Thinking
        );
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {insta::assert_debug_snapshot!("codex_progress",(&batch.sessions,&batch.chat_events));});
        ctx.fallback_state = Some(gnomcode_core::SessionState::Thinking);
        ctx.source_offset = bytes.len() as u64;
        let batch=adapter.parse(&ctx,b"{\"timestamp\":\"2026-11-06T14:00:04Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n");
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
    }

    #[test]
    fn codex_settings_change_does_not_start_a_turn() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let batch = CodexAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/codex/2026-11-06/model-settings.jsonl"),
        );
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {insta::assert_debug_snapshot!("codex_settings",(&batch.sessions,&batch.chat_events));});
        assert_eq!(batch.sessions[0].model.as_deref(), Some("gpt-6-luna"));
        assert!(
            batch
                .chat_events
                .iter()
                .any(|event| event.status.as_deref() == Some("model_changed")
                    || event.status.as_deref() == Some("model_active"))
        );
    }
    #[test]
    fn codex_incremental_settings_preserve_idle_state_and_workspace() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let ctx = FileCtx {
            source_path: "rollout-2026-11-06T18-37-55-01a1b114-8aa7-7793-b4f9-1422ef6159b1.jsonl"
                .into(),
            source_offset: 100,
            fallback_state: Some(gnomcode_core::SessionState::Idle),
            fallback_cwd: Some("/workspace/audit".into()),
            ..FileCtx::default()
        };
        let batch = CodexAdapter::new().parse(&ctx, b"{\"timestamp\":\"2026-11-06T13:40:23Z\",\"type\":\"event_msg\",\"payload\":{\"type\":\"thread_settings_applied\",\"thread_settings\":{\"model\":\"gpt-6-luna\"}}}\n");
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
        assert_eq!(batch.projects[0].repo_root, "/workspace/audit");
    }
    #[test]
    fn codex_attachment_transport_is_not_displayed_as_chat_text() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let text = "# Files mentioned by the user:\n## image.png: C:/example/image.png\nImage attachment: true\n## My request:\nExplain this screenshot.\n<image name=[Image #1] path=\"C:/example/image.png\"> </image>";
        let bytes = include_bytes!("../../../fixtures/codex/2026-11-02/image-wrapper.jsonl");
        let batch = CodexAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(
            batch.chat_events[0].content,
            "Explain this screenshot.\n\n[Image attachment: preview unavailable in imported history]"
        );
        let mut legacy = batch.chat_events;
        legacy[0].content = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            super::codex::clean_stored_conversation(legacy)[0].content,
            "Explain this screenshot.\n\n[Image attachment: preview unavailable in imported history]"
        );
        let bare = include_bytes!("../../../fixtures/codex/2026-11-14/bare-image-tag.jsonl");
        let bare_batch = CodexAdapter::new().parse(&FileCtx::default(), bare);
        assert_eq!(
            bare_batch.chat_events[0].content,
            "Why is the header clipped?\n\n[Image attachment: preview unavailable in imported history]"
        );
        assert!(
            !bare_batch.chat_events[0]
                .content
                .contains("codex-clipboard-fixture")
        );
    }
    #[test]
    fn native_titles_are_retained_by_supported_transcript_adapters() {
        use super::{FileCtx, SourceAdapter};
        let cases: Vec<(Box<dyn SourceAdapter>, &[u8])> = vec![
            (Box::new(super::codex::CodexAdapter::new()), br#"{"type":"session_meta","timestamp":"2026-10-26T12:00:00Z","payload":{"id":"native","cwd":"/workspace","title":"Saved chat name"}}
"#),
            (Box::new(super::claude::ClaudeAdapter::new()), br#"{"type":"user","sessionId":"native","timestamp":"2026-10-26T12:00:00Z","cwd":"/workspace","message":{"role":"user","content":"Question"}}
{"type":"custom-title","sessionId":"native","customTitle":"Saved chat name"}
"#),
        ];
        for (mut adapter, bytes) in cases {
            let batch = adapter.parse(&FileCtx::default(), bytes);
            assert_eq!(
                batch.sessions[0].title.as_deref(),
                Some("Saved chat name"),
                "{}",
                adapter.agent_id().as_str()
            );
        }
    }
    #[test]
    fn claude_desktop_entrypoint_is_a_desktop_surface() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let line = br#"{"type":"user","sessionId":"s","entrypoint":"claude-desktop","timestamp":"2026-11-11T20:40:20Z","cwd":"/w","message":{"role":"user","content":"Hi"}}
"#;
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), line);
        assert_eq!(
            batch.sessions[0].surface,
            gnomcode_core::Surface::DesktopApp
        );
    }
    #[test]
    fn claude_status_limits_are_exact_and_missing_windows_stay_absent() {
        let quota = super::claude::parse_status_limits(
            br#"{"observed_at":"2026-11-12T10:00:00Z","rate_limits":{"five_hour":{"used_percentage":42.5,"resets_at":1794480000}}}"#,
        );
        assert_eq!(quota.len(), 1);
        assert_eq!(quota[0].provider, "claude-code");
        assert_eq!(quota[0].window, gnomcode_core::QuotaWindow::FiveHour);
        assert!((quota[0].used_pct - 42.5).abs() < f64::EPSILON);
        assert_eq!(quota[0].basis, TrustLevel::Exact);
        assert!(quota[0].resets_at.is_some());
        assert!(super::claude::parse_status_limits(b"not json").is_empty());
        assert!(super::claude::parse_status_limits(br#"{"rate_limits":{}}"#).is_empty());
    }

    #[test]
    fn claude_oauth_usage_maps_every_window_and_skips_nulls() {
        let body = serde_json::json!({
            "five_hour": {"utilization": 12.0, "resets_at": "2026-11-12T03:00:00+00:00"},
            "seven_day": {"utilization": 40.5, "resets_at": "2026-11-16T08:00:00+00:00"},
            "seven_day_opus": null,
            "seven_day_sonnet": {"utilization": 3.0, "resets_at": null}
        });
        let quota = super::claude::parse_oauth_usage(&body, chrono::Utc::now());
        let ids: Vec<_> = quota
            .iter()
            .map(|q| (q.limit_id.as_str(), q.window))
            .collect();
        assert_eq!(
            ids,
            [
                ("claude", gnomcode_core::QuotaWindow::FiveHour),
                ("claude", gnomcode_core::QuotaWindow::Weekly),
                ("sonnet", gnomcode_core::QuotaWindow::Weekly)
            ]
        );
        assert!(quota.iter().all(|q| q.basis == TrustLevel::Exact));
        assert!(quota[0].resets_at.is_some() && quota[2].resets_at.is_none());
    }

    #[test]
    fn antigravity_quota_summary_maps_pools_and_windows() {
        let body = serde_json::json!({"response": {"groups": [
            {"displayName": "Gemini Models", "buckets": [
                {"bucketId": "gemini-weekly", "window": "weekly", "remainingFraction": 0.47, "resetTime": "2026-11-15T17:52:22Z"},
                {"bucketId": "gemini-5h", "window": "5h", "remainingFraction": 1, "resetTime": "2026-11-12T07:43:50Z"}
            ]},
            {"displayName": "Gemini 4 Argon", "buckets": [
                {"bucketId": "gemini-argon-wtu-weekly", "window": "weekly", "remainingFraction": 0, "resetTime": "2026-11-16T09:21:14Z"}
            ]},
            {"displayName": "Unknown window", "buckets": [{"window": "daily", "remainingFraction": 1}]}
        ]}});
        let quota = super::antigravity::parse_quota_summary(&body, chrono::Utc::now());
        let pools: Vec<_> = quota
            .iter()
            .map(|q| (q.limit_id.as_str(), q.window, q.used_pct.round() as i64))
            .collect();
        assert_eq!(
            pools,
            [
                ("Gemini Models", gnomcode_core::QuotaWindow::Weekly, 53),
                ("Gemini Models", gnomcode_core::QuotaWindow::FiveHour, 0),
                ("Gemini 4 Argon", gnomcode_core::QuotaWindow::Weekly, 100)
            ]
        );
        assert!(
            quota
                .iter()
                .all(|q| q.basis == TrustLevel::Api && q.resets_at.is_some())
        );
    }

    #[test]
    fn antigravity_user_status_maps_model_quota() {
        let now = chrono::Utc::now();
        let soon = (now + chrono::Duration::hours(3)).to_rfc3339();
        let later = (now + chrono::Duration::days(4)).to_rfc3339();
        let body = serde_json::json!({"userStatus": {"userTier": {"name": "Pro"}, "cascadeModelConfigData": {"clientModelConfigs": [
            {"label": "Gemini 3.8 Flash", "modelOrAlias": {"model": "m1"}, "quotaInfo": {"remainingFraction": 0.75, "resetTime": soon}},
            {"label": "Claude Opus", "modelOrAlias": {"model": "m2"}, "quotaInfo": {"remainingFraction": 0.0, "resetTime": later}},
            {"label": "No quota", "modelOrAlias": {"model": "m3"}}
        ]}}});
        let quota = super::antigravity::parse_user_status(&body, now);
        assert_eq!(quota.len(), 2);
        assert_eq!(quota[0].limit_id, "Gemini 3.8 Flash");
        assert_eq!(quota[0].window, gnomcode_core::QuotaWindow::FiveHour);
        assert!((quota[0].used_pct - 25.0).abs() < 1e-9);
        assert_eq!(quota[1].window, gnomcode_core::QuotaWindow::Weekly);
        assert!((quota[1].used_pct - 100.0).abs() < 1e-9);
        assert_eq!(quota[0].plan.as_deref(), Some("Pro"));
    }

    #[test]
    fn claude_subagent_transcript_is_a_linked_child_session() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let log = br#"{"type":"user","isSidechain":true,"agentId":"a7fe2469649710219","sessionId":"f9e0d79c-2390-4145-b072-d45b7266d56f","timestamp":"2026-11-11T22:10:13Z","cwd":"/w","message":{"role":"user","content":"Read a file"}}
{"type":"assistant","isSidechain":true,"agentId":"a7fe2469649710219","sessionId":"f9e0d79c-2390-4145-b072-d45b7266d56f","timestamp":"2026-11-11T22:10:15Z","cwd":"/w","requestId":"r1","message":{"id":"m1","role":"assistant","model":"claude-haiku-5-5","content":[{"type":"text","text":"stable"}],"usage":{"input_tokens":10,"output_tokens":2}}}
"#;
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), log);
        let child = "claude-code:agent-a7fe2469649710219";
        assert!(batch.sessions.iter().all(|s| s.id == child));
        assert!(batch.usage_records.iter().all(|u| u.session_id == child));
        assert_eq!(
            batch.sessions[0].parent_session_id.as_deref(),
            Some("claude-code:f9e0d79c-2390-4145-b072-d45b7266d56f")
        );
    }

    #[test]
    fn claude_desktop_main_transcript_is_an_unlinked_desktop_session() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let bytes = include_bytes!("../../../fixtures/claude-code/v2.1.321/desktop-main.jsonl");
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.sessions.len(), 1);
        let session = &batch.sessions[0];
        assert_eq!(session.id, "claude-code:id_0001");
        assert_eq!(session.surface, gnomcode_core::Surface::DesktopApp);
        assert_eq!(session.parent_session_id, None);
    }

    #[test]
    fn claude_subagent_bookkeeping_lines_never_reach_the_parent_session() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let bytes = include_bytes!("../../../fixtures/claude-code/v2.1.321/desktop-subagent.jsonl");
        let child = "claude-code:agent-id_0001";
        let parent = "claude-code:id_0003";
        let lines: Vec<&[u8]> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
        // Whole file, then every tail that starts on a marker-less line (what a live tail reads).
        for start in 0..lines.len() {
            let chunk = lines[start..].concat();
            let batch = ClaudeAdapter::new().parse(&FileCtx::default(), &chunk);
            let session = &batch.sessions[0];
            assert_eq!(session.id, child, "tail starting at line {start}");
            assert_eq!(session.parent_session_id.as_deref(), Some(parent));
        }
        // The file name alone identifies the child even when no line carries agentId.
        let ctx = FileCtx {
            source_path: r"C:\claude\projects\p\id_0003\subagents\agent-id_0001.jsonl".to_owned(),
            ..FileCtx::default()
        };
        let bare =
            br#"{"type":"api-request","sessionId":"id_0003","timestamp":"2026-11-12T13:14:40.121Z"}
"#;
        let batch = ClaudeAdapter::new().parse(&ctx, bare);
        assert_eq!(batch.sessions[0].id, child);
        assert_eq!(batch.sessions[0].parent_session_id.as_deref(), Some(parent));
    }

    #[test]
    fn claude_subagent_sidecar_names_the_child_from_its_task_description() {
        use super::claude::{subagent_label_from_meta, subagent_sidecar};
        let described =
            include_bytes!("../../../fixtures/claude-code/v2.1.321/desktop-subagent.meta.json");
        let typed = include_bytes!(
            "../../../fixtures/claude-code/v2.1.321/desktop-subagent-typeonly.meta.json"
        );
        assert_eq!(
            subagent_label_from_meta(described).as_deref(),
            Some("Audit sample project settings")
        );
        assert_eq!(subagent_label_from_meta(typed).as_deref(), Some("explorer"));
        assert_eq!(subagent_label_from_meta(b"{not json"), None);
        assert_eq!(subagent_label_from_meta(br#"{"description":"  "}"#), None);
        let (sidecar, child) =
            subagent_sidecar("/c/projects/p/id_0003/subagents/agent-id_0001.jsonl")
                .expect("sidecar");
        assert!(sidecar.ends_with("agent-id_0001.meta.json"));
        assert_eq!(child, "claude-code:agent-id_0001");
        assert!(subagent_sidecar("/c/projects/p/id_0003.jsonl").is_none());
    }

    #[test]
    fn claude_session_is_never_its_own_parent() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let log = br#"{"type":"api-request","sessionId":"s1","timestamp":"2026-11-12T13:14:40Z"}
{"type":"user","isSidechain":true,"agentId":"a1","sessionId":"s1","timestamp":"2026-11-12T13:14:41Z","cwd":"/w","message":{"role":"user","content":"x"}}
"#;
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), log);
        assert!(
            batch
                .sessions
                .iter()
                .all(|s| s.parent_session_id.as_deref() != Some(&s.id))
        );
    }

    #[test]
    fn antigravity_reply_from_created_subagent_does_not_make_parent_a_child() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let parent_ctx = FileCtx {
            source_path: "/h/.gemini/antigravity/brain/8fce098d-c9f3-405c-af14-d078f0b40115/.system_generated/logs/transcript.jsonl".into(),
            ..FileCtx::default()
        };
        let parent = br#"{"step_index":0,"type":"USER_INPUT","source":"USER","created_at":"2026-10-25T21:30:00Z","content":"Design a logo"}
{"step_index":5,"type":"SYSTEM_MESSAGE","source":"SYSTEM","created_at":"2026-10-25T21:38:30Z","content":"[Message] sender=8650dd28-1fdb-4a8d-9756-1aef8d290e08 priority=HIGH content=done"}
"#;
        let batch = AntigravityAdapter::new().parse(&parent_ctx, parent);
        assert!(batch.sessions.iter().all(|s| s.parent_session_id.is_none()));
        let child_ctx = FileCtx {
            source_path: "/h/.gemini/antigravity/brain/8650dd28-1fdb-4a8d-9756-1aef8d290e08/.system_generated/logs/transcript.jsonl".into(),
            ..FileCtx::default()
        };
        let child = br#"{"step_index":0,"type":"SYSTEM_MESSAGE","source":"SYSTEM","created_at":"2026-10-25T21:38:12Z","content":"[Message] sender=8fce098d-c9f3-405c-af14-d078f0b40115 priority=HIGH content=task"}
"#;
        let batch = AntigravityAdapter::new().parse(&child_ctx, child);
        assert_eq!(
            batch.sessions[0].parent_session_id.as_deref(),
            Some("antigravity:8fce098d-c9f3-405c-af14-d078f0b40115")
        );
    }

    #[test]
    fn codex_forked_rollout_keeps_its_own_identity() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let log = br#"{"type":"session_meta","ordinal":0,"timestamp":"2026-11-09T19:56:59Z","payload":{"id":"01a1c1e2-a983-7782-a159-fb9a11f1cb56","forked_from_id":"01a1c1df-7203-7a32-97f4-1de4edf53491","parent_thread_id":"01a1c1df-7203-7a32-97f4-1de4edf53491","cwd":"/w"}}
{"type":"session_meta","ordinal":1,"timestamp":"2026-11-09T19:56:59Z","payload":{"id":"01a1c1df-7203-7a32-97f4-1de4edf53491","cwd":"/w"}}
{"type":"event_msg","ordinal":2,"timestamp":"2026-11-09T19:57:19Z","payload":{"type":"token_count","info":{"total_token_usage":{"output_tokens":5},"last_token_usage":{"output_tokens":5}}}}
"#;
        let batch = CodexAdapter::new().parse(&FileCtx::default(), log);
        let child = "codex:01a1c1e2-a983-7782-a159-fb9a11f1cb56";
        assert!(batch.usage_records.iter().all(|u| u.session_id == child));
        assert!(batch.sessions.iter().all(|s| s.id == child));
        assert_eq!(
            batch.sessions[0].parent_session_id.as_deref(),
            Some("codex:01a1c1df-7203-7a32-97f4-1de4edf53491")
        );
    }

    #[test]
    fn codex_reemitted_token_count_is_counted_once() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let event = |ordinal: u32, total: u32, last: u32| {
            format!(
                r#"{{"type":"event_msg","ordinal":{ordinal},"timestamp":"2026-11-11T10:00:0{ordinal}Z","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{total},"output_tokens":{total}}},"last_token_usage":{{"input_tokens":{last},"output_tokens":{last}}}}}}}}}"#
            )
        };
        let log = format!(
            "{{\"type\":\"session_meta\",\"timestamp\":\"2026-11-11T10:00:00Z\",\"payload\":{{\"id\":\"01a11180-e7a8-7d12-9e69-9f6fadc2399b\",\"cwd\":\"/w\"}}}}
{}
{}
{}
",
            event(1, 10, 10),
            event(2, 10, 10),
            event(3, 25, 15)
        );
        let batch = CodexAdapter::new().parse(&FileCtx::default(), log.as_bytes());
        let output: u64 = batch.usage_records.iter().map(|u| u.output_tokens).sum();
        assert_eq!(output, 25);
    }

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
        assert!(
            batch
                .chat_events
                .iter()
                .any(|event| event.role == gnomcode_core::ChatRole::User
                    && event.content == "[REDACTED]")
        );
        assert_eq!(
            batch
                .chat_events
                .iter()
                .filter(|event| event.role == gnomcode_core::ChatRole::User)
                .count(),
            1
        );
    }

    #[test]
    fn codex_legacy_context_cleanup_preserves_prompts_and_cursor_ids() {
        use gnomcode_core::{ChatEvent, ChatRole};
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
    fn codex_subagent_rollout_preserves_unique_id_and_parent_and_title() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let log = concat!(
            "{\"timestamp\":\"2026-11-04T14:05:57.542Z\",\"ordinal\":0,\"type\":\"session_meta\",\"payload\":{",
            "\"session_id\":\"01a1a67d-c0e4-7492-a427-923d843b7ed2\",",
            "\"id\":\"01a1a6e1-7ada-7251-90a6-4b52ff6f50dc\",",
            "\"parent_thread_id\":\"01a1a67d-c0e4-7492-a427-923d843b7ed2\",",
            "\"agent_nickname\":\"Goodall\",",
            "\"agent_path\":\"/root/control\",",
            "\"base_instructions\":{\"text\":\"Codex subagent instructions\"}",
            "}}\n",
            "{\"timestamp\":\"2026-11-04T14:05:59.800Z\",\"ordinal\":9,\"type\":\"turn_context\",\"payload\":{\"model\":\"gpt-6-luna\"}}\n",
        );
        let ctx = FileCtx {
            source_path: "rollout-2026-11-04T19-05-57-01a1a6e1-7ada-7251-90a6-4b52ff6f50dc.jsonl"
                .into(),
            ..FileCtx::default()
        };
        let mut adapter = CodexAdapter::new();
        let batch = adapter.parse(&ctx, log.as_bytes());
        assert_eq!(batch.sessions.len(), 1);
        let session = &batch.sessions[0];
        assert_eq!(session.id, "codex:01a1a6e1-7ada-7251-90a6-4b52ff6f50dc");
        assert_eq!(
            session.parent_session_id.as_deref(),
            Some("codex:01a1a67d-c0e4-7492-a427-923d843b7ed2")
        );
        assert_eq!(session.model.as_deref(), Some("gpt-6-luna"));
        assert_eq!(
            session.title.as_deref(),
            Some("Goodall (control) · Subagent")
        );
    }
    #[test]
    fn codex_tool_calls_at_offset_do_not_create_phantom_sessions() {
        use super::{FileCtx, SourceAdapter, codex::CodexAdapter};
        let log = concat!(
            "{\"timestamp\":\"2026-11-04T12:17:12.314Z\",\"ordinal\":20,\"type\":\"response_item\",\"payload\":{\"type\":\"custom_tool_call\",\"id\":\"ctc_02cdd58d84ae694a016aeb223ec0d887d0b7b6297c5329f7f8\",\"status\":\"completed\",\"call_id\":\"call_ea03230f6e7f447587755c15c40bcb2a\",\"name\":\"exec\",\"input\":\"const r = 1;\"}}\n",
            "{\"timestamp\":\"2026-11-04T12:17:15.314Z\",\"ordinal\":21,\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"id\":\"fc_068a987355b27ec4016aeb66cf091c87d0a66e72a48a18d122\",\"name\":\"shell\",\"arguments\":\"{}\"}}\n",
        );
        let ctx = FileCtx {
            source_path: "rollout-2026-11-04T17-17-01-01a1a67d-c0e4-7492-a427-923d843b7ed2.jsonl"
                .into(),
            source_offset: 5000,
            ..FileCtx::default()
        };
        let mut adapter = CodexAdapter::new();
        let batch = adapter.parse(&ctx, log.as_bytes());
        for sess in &batch.sessions {
            assert!(
                !sess.id.contains("ctc_") && !sess.id.contains("fc_"),
                "Tool call ID must never be used as a session ID: {}",
                sess.id
            );
            assert_eq!(sess.id, "codex:01a1a67d-c0e4-7492-a427-923d843b7ed2");
        }
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
        assert_eq!(first.chat_events[0].role, gnomcode_core::ChatRole::User);
        assert_eq!(
            first.chat_events[1].role,
            gnomcode_core::ChatRole::Assistant
        );
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
        assert_eq!(batch.chat_events[0].role, gnomcode_core::ChatRole::User);
        assert_eq!(
            batch.chat_events[1].role,
            gnomcode_core::ChatRole::Assistant
        );
        assert!(
            batch
                .chat_events
                .iter()
                .all(|event| event.content == "[REDACTED]")
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
        assert_eq!(cleaned[1].role, gnomcode_core::ChatRole::Tool);
        assert!(cleaned[1].content.is_empty());
    }

    #[test]
    fn antigravity_cli_1_4_transcript_reuses_the_existing_adapter() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let bytes = include_bytes!("../../../fixtures/antigravity/cli-1.4.12/sample.jsonl");
        let context = FileCtx {
            source_path:
                "/fixture/antigravity-cli/brain/cli-test/.system_generated/logs/transcript.jsonl"
                    .into(),
            fallback_cwd: Some("/workspace".into()),
            ..FileCtx::default()
        };
        let batch = AntigravityAdapter::new().parse(&context, bytes);
        assert_eq!(batch.parse_errors, 0);
        assert_eq!(batch.sessions[0].surface, gnomcode_core::Surface::Cli);
        assert_eq!(batch.chat_events.len(), 1);
        assert_eq!(batch.chat_events[0].content, "[REDACTED]");
        insta::with_settings!({snapshot_path => std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("snapshots")}, {
            insta::assert_debug_snapshot!("antigravity_cli_1_4", (batch.sessions, batch.usage_records, batch.chat_events));
        });
    }

    use super::{
        FileCtx, SourceAdapter, antigravity::AntigravityAdapter, claude::ClaudeAdapter,
        codex::CodexAdapter,
    };
    use gnomcode_core::{AgentId, Surface, TrustLevel};
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
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
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
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
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
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Idle);
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
    fn missing_model_never_invents_a_priced_provider_model() {
        let cases: Vec<(Box<dyn SourceAdapter>, &[u8])> = vec![
            (
                Box::new(ClaudeAdapter::new()),
                include_bytes!("../../../fixtures/claude-code/synthetic/missing-model.jsonl"),
            ),
            (
                Box::new(CodexAdapter::new()),
                include_bytes!("../../../fixtures/codex/synthetic/missing-model.jsonl"),
            ),
            (
                Box::new(super::antigravity::AntigravityAdapter::new()),
                include_bytes!("../../../fixtures/antigravity/synthetic/missing-model.jsonl"),
            ),
        ];
        for (mut adapter, bytes) in cases {
            let batch = adapter.parse(&FileCtx::default(), bytes);
            assert_eq!(
                batch.usage_records.len(),
                1,
                "{}",
                adapter.agent_id().as_str()
            );
            let usage = &batch.usage_records[0];
            assert_eq!(usage.model, "unknown", "{}", adapter.agent_id().as_str());
            assert_eq!(
                usage.cost_basis,
                TrustLevel::Unknown,
                "{}",
                adapter.agent_id().as_str()
            );
            assert!(
                batch.sessions[0].model.is_none(),
                "{}",
                adapter.agent_id().as_str()
            );
        }
    }

    #[test]
    fn all_adapters_reject_truncated_input_without_storing_chat() {
        let adapters: Vec<Box<dyn SourceAdapter>> = vec![
            Box::new(ClaudeAdapter::new()),
            Box::new(CodexAdapter::new()),
            Box::new(super::antigravity::AntigravityAdapter::new()),
        ];
        for mut adapter in adapters {
            let batch = adapter.parse(&FileCtx::default(), b"{\"truncated\":\r\n");
            assert!(batch.parse_errors > 0, "{}", adapter.agent_id().as_str());
            assert!(
                batch.chat_events.is_empty(),
                "{}",
                adapter.agent_id().as_str()
            );
        }
    }

    #[test]
    fn claude_usage_limit_fixture_marks_error_rate_limited_and_quota() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let bytes = include_bytes!("../../../fixtures/claude-code/2026-11-13/usage-limit.jsonl");
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.sessions.len(), 1);
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Error);
        assert!(
            batch
                .activity_events
                .iter()
                .any(|e| e.kind == gnomcode_core::ActivityKind::RateLimited)
        );
        let snap = batch
            .quota_snapshots
            .iter()
            .find(|q| q.provider == "claude-code")
            .expect("rejected quota snapshot");
        assert_eq!(snap.used_pct, 100.0);
        assert_eq!(snap.basis, TrustLevel::Exact);
        assert_eq!(
            snap.resets_at,
            chrono::DateTime::from_timestamp(1_794_507_600, 0)
        );
    }

    #[test]
    fn claude_api_error_and_stream_json_rate_limit_emit_error_or_rate_limited() {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let auth_err = br#"{"type":"assistant","sessionId":"s-auth","timestamp":"2026-11-13T00:00:00Z","isApiErrorMessage":true,"apiErrorStatus":401,"error":"authentication_failed","message":{"role":"assistant","content":[{"type":"text","text":"401 Unauthorized"}]}}"#;
        let b1 = ClaudeAdapter::new().parse(&FileCtx::default(), auth_err);
        assert_eq!(b1.sessions[0].state, gnomcode_core::SessionState::Error);
        assert!(b1.activity_events.iter().any(|e| {
            e.kind == gnomcode_core::ActivityKind::Error
                && e.summary_redacted
                    .as_deref()
                    .is_some_and(|s| s.contains("claude auth login"))
        }));

        let rl_ev = br#"{"type":"rate_limit_event","sessionId":"s-rl","timestamp":"2026-11-13T00:00:01Z","rate_limit_info":{"status":"rejected","resetsAt":1794507600,"rateLimitType":"seven_day"}}"#;
        let b2 = ClaudeAdapter::new().parse(&FileCtx::default(), rl_ev);
        assert_eq!(b2.sessions[0].state, gnomcode_core::SessionState::Error);
        assert!(
            b2.activity_events
                .iter()
                .any(|e| e.kind == gnomcode_core::ActivityKind::RateLimited)
        );
        assert_eq!(
            b2.quota_snapshots[0].window,
            gnomcode_core::QuotaWindow::Weekly
        );

        let res_err = br#"{"type":"result","sessionId":"s-res","timestamp":"2026-11-13T00:00:02Z","is_error":true,"result":"API error 529 overloaded"}"#;
        let b3 = ClaudeAdapter::new().parse(&FileCtx::default(), res_err);
        assert_eq!(b3.sessions[0].state, gnomcode_core::SessionState::Error);
        assert!(
            b3.activity_events
                .iter()
                .any(|e| e.kind == gnomcode_core::ActivityKind::Error)
        );
    }

    #[test]
    fn antigravity_resource_exhausted_records_the_model_and_no_quota_row() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let bytes = include_bytes!(
            "../../../fixtures/antigravity/synthetic/resource-exhausted-model.jsonl"
        );
        let batch = AntigravityAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Error);
        let event = batch
            .activity_events
            .iter()
            .find(|e| e.kind == gnomcode_core::ActivityKind::RateLimited)
            .expect("rate limited event");
        assert_eq!(event.tool_name.as_deref(), Some("Claude Opus 5.5 (High)"));
        assert!(
            batch.quota_snapshots.is_empty(),
            "the error does not say which pool is spent"
        );
    }

    #[test]
    fn antigravity_cancelled_or_denied_tool_step_resolves_the_open_call() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let pending = AntigravityAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/antigravity/synthetic/tool-pending.jsonl"),
        );
        assert_eq!(pending.sessions[0].state, gnomcode_core::SessionState::Tool);
        let cancelled = AntigravityAdapter::new().parse(
            &FileCtx::default(),
            include_bytes!("../../../fixtures/antigravity/synthetic/tool-cancelled.jsonl"),
        );
        assert_eq!(
            cancelled.sessions[0].state,
            gnomcode_core::SessionState::Idle
        );
        assert!(
            cancelled
                .activity_events
                .iter()
                .any(|e| e.kind == gnomcode_core::ActivityKind::ToolEnd)
        );
    }

    #[test]
    fn antigravity_resource_exhausted_without_a_known_model_records_none() {
        use super::{FileCtx, SourceAdapter, antigravity::AntigravityAdapter};
        let bytes = include_bytes!(
            "../../../fixtures/antigravity/synthetic/resource-exhausted-no-model.jsonl"
        );
        let batch = AntigravityAdapter::new().parse(&FileCtx::default(), bytes);
        assert_eq!(batch.sessions[0].state, gnomcode_core::SessionState::Error);
        let event = batch
            .activity_events
            .iter()
            .find(|e| e.kind == gnomcode_core::ActivityKind::RateLimited)
            .expect("rate limited event");
        assert_eq!(event.tool_name, None);
        assert!(batch.quota_snapshots.is_empty());
    }

    #[test]
    fn antigravity_quota_summary_lists_the_models_each_pool_covers() {
        let body: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/antigravity/cli-1.5.8/quota-summary.json"
        ))
        .expect("fixture json");
        let members = super::antigravity::parse_quota_pool_members(&body);
        let of = |pool: &str| -> Vec<&str> {
            members
                .iter()
                .filter(|m| m.0 == pool)
                .map(|m| m.1.as_str())
                .collect()
        };
        assert_eq!(
            of("Claude and GPT models"),
            [
                "Claude and GPT models",
                "Claude Opus",
                "Claude Sonnet",
                "GPT-OSS"
            ]
        );
        assert_eq!(
            of("Gemini Models"),
            ["Gemini Models", "Gemini Flash", "Gemini Pro"]
        );
        assert_eq!(
            of("Gemini 4 Argon"),
            ["Gemini 4 Argon"],
            "a group with no model list keeps only its own name"
        );
    }

    #[test]
    fn claude_v2_1_325_header_lines_without_cwd_or_timestamp_adopt_first_turn_project_and_timestamp()
     {
        use super::{FileCtx, SourceAdapter, claude::ClaudeAdapter};
        let cwd = r"C:\Users\Wahaj\AppData\Local\Temp\gnomcode-live-handoff-repo";
        let payload = concat!(
            r#"{"type":"mode","mode":"normal","sessionId":"35596450-15f1-48f8-a249-a5d9b05cf8d5"}"#,
            "\n",
            r#"{"type":"permission-mode","permissionMode":"auto","sessionId":"35596450-15f1-48f8-a249-a5d9b05cf8d5"}"#,
            "\n",
            r#"{"type":"session-owner","accountUuid":"acct-1","organizationUuid":"org-1","sessionId":"35596450-15f1-48f8-a249-a5d9b05cf8d5"}"#,
            "\n",
            r#"{"type":"atis-latch","atis":"","sessionId":"35596450-15f1-48f8-a249-a5d9b05cf8d5"}"#,
            "\n",
            r#"{"type":"user","sessionId":"35596450-15f1-48f8-a249-a5d9b05cf8d5","timestamp":"2026-11-14T17:12:50.161Z","cwd":"C:\\Users\\Wahaj\\AppData\\Local\\Temp\\gnomcode-live-handoff-repo","gitBranch":"master","uuid":"u-1","message":{"role":"user","content":"Reply with the exact single word PINEAPPLE and nothing else."}}"#,
            "\n"
        );
        let batch = ClaudeAdapter::new().parse(&FileCtx::default(), payload.as_bytes());
        assert_eq!(batch.sessions.len(), 1);
        let expected_project = gnomcode_core::Project::from_origin(None, cwd, Some("master"));
        assert_eq!(batch.sessions[0].project_id, expected_project.id);
        assert_eq!(
            batch.sessions[0].started_at.to_rfc3339(),
            "2026-11-14T17:12:50.161+00:00"
        );
        assert_eq!(batch.projects.len(), 1);
        assert_eq!(batch.projects[0].id, expected_project.id);
    }
}
