//! Native chat labels from supported transcript and session-index metadata.

use gnomcode_core::AgentId;
use serde_json::Value;
use std::collections::BTreeMap;

/// Reads bounded, redacted native labels without interpreting prompt text as metadata.
#[must_use]
pub fn native_session_titles(agent: AgentId, bytes: &[u8]) -> BTreeMap<String, String> {
    let values = serde_json::from_slice::<Value>(bytes).map_or_else(
        |_| {
            bytes
                .split(|byte| *byte == b'\n')
                .filter_map(|line| serde_json::from_slice(line).ok())
                .collect()
        },
        |value| vec![value],
    );
    let mut labels: BTreeMap<String, (u8, String)> = BTreeMap::new();
    for value in &values {
        let entries = value
            .get("entries")
            .and_then(Value::as_array)
            .map_or_else(|| std::slice::from_ref(value), Vec::as_slice);
        for entry in entries {
            let entry = if agent == AgentId::Codex
                && entry.get("type").and_then(Value::as_str) == Some("session_meta")
            {
                entry.get("payload").unwrap_or(entry)
            } else {
                entry
            };
            let id = match agent {
                AgentId::Codex => entry.get("id"),
                AgentId::ClaudeCode | AgentId::GeminiCli => entry.get("sessionId"),
                _ => None,
            }
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty());
            let (title, priority) = match agent {
                AgentId::Codex => (entry.get("thread_name").or_else(|| entry.get("title")), 1),
                AgentId::ClaudeCode => {
                    if let Some(title) = entry.get("customTitle") {
                        (Some(title), 2)
                    } else {
                        (entry.get("summary"), 1)
                    }
                }
                AgentId::GeminiCli => (entry.get("title").or_else(|| entry.get("summary")), 1),
                _ => (None, 0),
            };
            let Some((id, title)) = id.zip(title.and_then(Value::as_str)) else {
                continue;
            };
            let title: String = gnomcode_core::redact::redact_chat_content(title)
                .trim()
                .chars()
                .take(256)
                .collect();
            if title.is_empty() {
                continue;
            }
            let id = gnomcode_core::Session::compose_id(agent, id);
            if labels.get(&id).is_none_or(|(rank, _)| *rank <= priority) {
                labels.insert(id, (priority, title));
            }
        }
    }
    labels
        .into_iter()
        .map(|(id, (_, title))| (id, title))
        .collect()
}

pub(crate) fn apply_native_titles(batch: &mut crate::ParsedBatch, agent: AgentId, bytes: &[u8]) {
    let labels = native_session_titles(agent, bytes);
    for session in &mut batch.sessions {
        if let Some(title) = labels.get(&session.id) {
            session.title = Some(title.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anonymized_native_codex_index_has_a_saved_label() {
        let labels = native_session_titles(
            AgentId::Codex,
            include_bytes!("../../../fixtures/codex/2026-11-01/session_index.jsonl"),
        );
        insta::assert_json_snapshot!(labels, @r#"
        {
          "codex:id_0001": "[REDACTED]"
        }
        "#);
    }
    #[test]
    fn codex_index_renames_and_malformed_lines_are_handled() {
        let labels = native_session_titles(AgentId::Codex, b"{\"id\":\"saved\",\"thread_name\":\"Old\"}\ninvalid\n{\"id\":\"saved\",\"thread_name\":\"Renamed chat\"}\n");
        assert_eq!(labels["codex:saved"], "Renamed chat");
    }
    #[test]
    fn claude_custom_name_beats_generated_summary_and_secrets_are_redacted() {
        let labels = native_session_titles(AgentId::ClaudeCode, br#"{"entries":[{"sessionId":"saved","customTitle":"My chat --token secret"},{"sessionId":"saved","summary":"Generated summary"}]}"#);
        assert_eq!(labels["claude-code:saved"], "My chat --token <REDACTED>");
    }
}
