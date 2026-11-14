//! Separates harness-injected entries from real user text in user-role chat entries.
//!
//! Agent transcripts store task notifications, reminders, slash-command records, subagent
//! reports, compact summaries and interrupt markers as user-role lines. They are not text the
//! user typed, so they become system events with a subtype.

/// A user-role entry classified as real user text or a system event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserEntry {
    /// Text the user typed, with harness tags stripped.
    User(String),
    /// A harness-injected event.
    System {
        /// Stable subtype: `task_notification`, `reminder`, `command`, `subagent_report`,
        /// `compact_summary` or `interrupt`.
        subtype: &'static str,
        /// One-line summary shown collapsed.
        summary: String,
        /// Optional expandable detail.
        detail: String,
    },
}

const TAGS: [&str; 8] = [
    "task-notification",
    "system-reminder",
    "local-command-caveat",
    "local-command-stdout",
    "command-name",
    "command-message",
    "command-args",
    "agent-message",
];

/// Removes every `<tag ...>...</tag>` block, returning the remaining text and the blocks found.
fn strip_tags(text: &str) -> (String, Vec<(&'static str, String)>) {
    let mut rest = text.to_owned();
    let mut found = Vec::new();
    for tag in TAGS {
        let open = format!("<{tag}");
        let close = format!("</{tag}>");
        while let Some(start) = rest.find(&open) {
            let after = &rest[start + open.len()..];
            if !after.starts_with(['>', ' ', '\n', '\t', '\r']) {
                break;
            }
            let Some(body_start) = after.find('>').map(|i| start + open.len() + i + 1) else {
                break;
            };
            let (inner, end) = match rest[body_start..].find(&close) {
                Some(i) => (
                    rest[body_start..body_start + i].to_owned(),
                    body_start + i + close.len(),
                ),
                None => (rest[body_start..].to_owned(), rest.len()),
            };
            found.push((tag, inner.trim().to_owned()));
            rest.replace_range(start..end, "");
        }
    }
    (rest.trim().to_owned(), found)
}

fn inner<'a>(blocks: &'a [(&'static str, String)], tag: &str) -> Option<&'a str> {
    blocks
        .iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, v)| v.as_str())
}

fn child(body: &str, tag: &str) -> Option<String> {
    let start = body.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = body[start..].find(&format!("</{tag}>"))? + start;
    Some(body[start..end].trim().to_owned()).filter(|s| !s.is_empty())
}

fn one_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    if line.chars().count() > 120 {
        format!("{}...", line.chars().take(117).collect::<String>())
    } else {
        line.to_owned()
    }
}

/// Classifies a user-role entry. `is_meta` and `is_compact_summary` come from the source line.
#[must_use]
pub fn classify_user_entry(text: &str, is_meta: bool, is_compact_summary: bool) -> UserEntry {
    let trimmed = text.trim();
    if is_compact_summary
        || trimmed.starts_with("This session is being continued from a previous conversation")
    {
        return UserEntry::System {
            subtype: "compact_summary",
            summary: "Conversation compacted".to_owned(),
            detail: trimmed.to_owned(),
        };
    }
    if trimmed.starts_with("[Request interrupted by user") {
        return UserEntry::System {
            subtype: "interrupt",
            summary: "Interrupted by you".to_owned(),
            detail: String::new(),
        };
    }
    let (rest, blocks) = strip_tags(trimmed);
    if !rest.is_empty() && !is_meta {
        return UserEntry::User(rest);
    }
    if let Some(name) = inner(&blocks, "command-name") {
        let name = name.trim();
        let command = if name.starts_with('/') {
            name.to_owned()
        } else {
            format!("/{name}")
        };
        let args = inner(&blocks, "command-args").unwrap_or("").trim();
        return UserEntry::System {
            subtype: "command",
            summary: format!("Ran {command}"),
            detail: if args.is_empty() {
                String::new()
            } else {
                format!("{command} {args}")
            },
        };
    }
    if let Some(body) = inner(&blocks, "task-notification") {
        let label = child(body, "summary").or_else(|| child(body, "description"));
        return UserEntry::System {
            subtype: "task_notification",
            summary: label.map_or_else(
                || "Background task finished".to_owned(),
                |l| format!("Background task finished: {}", one_line(&l)),
            ),
            detail: child(body, "result").unwrap_or_default(),
        };
    }
    if let Some(body) = inner(&blocks, "agent-message") {
        return UserEntry::System {
            subtype: "subagent_report",
            summary: "Subagent report".to_owned(),
            detail: body.to_owned(),
        };
    }
    if let Some(body) = inner(&blocks, "local-command-stdout") {
        return UserEntry::System {
            subtype: "command",
            summary: "Command output".to_owned(),
            detail: body.to_owned(),
        };
    }
    UserEntry::System {
        subtype: "reminder",
        summary: "System context".to_owned(),
        detail: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subtype(entry: &UserEntry) -> &str {
        match entry {
            UserEntry::System { subtype, .. } => subtype,
            UserEntry::User(_) => "user",
        }
    }

    #[test]
    fn task_notification_becomes_a_summary_chip() {
        let entry = classify_user_entry(
            "<task-notification>\n<task-id>a1</task-id>\n<summary>Chat rendering done</summary>\n<result>ok</result>\n</task-notification>",
            false,
            false,
        );
        assert_eq!(
            entry,
            UserEntry::System {
                subtype: "task_notification",
                summary: "Background task finished: Chat rendering done".into(),
                detail: "ok".into()
            }
        );
    }

    #[test]
    fn every_tag_type_is_classified() {
        for (text, expected) in [
            ("<system-reminder>be brief</system-reminder>", "reminder"),
            (
                "<local-command-caveat>Caveat</local-command-caveat>",
                "reminder",
            ),
            (
                "<local-command-stdout>done</local-command-stdout>",
                "command",
            ),
            (
                "<command-name>/compact</command-name><command-message>compact</command-message><command-args></command-args>",
                "command",
            ),
            (
                "<agent-message from=\"x\">report</agent-message>",
                "subagent_report",
            ),
            ("[Request interrupted by user]", "interrupt"),
            (
                "This session is being continued from a previous conversation that ran out",
                "compact_summary",
            ),
        ] {
            assert_eq!(
                subtype(&classify_user_entry(text, false, false)),
                expected,
                "{text}"
            );
        }
        assert_eq!(
            subtype(&classify_user_entry("hello", true, false)),
            "reminder"
        );
        assert_eq!(
            subtype(&classify_user_entry("x", false, true)),
            "compact_summary"
        );
    }

    #[test]
    fn command_shows_slash_name() {
        assert!(matches!(
            classify_user_entry("<command-name>compact</command-name>", false, false),
            UserEntry::System { summary, .. } if summary == "Ran /compact"
        ));
    }

    #[test]
    fn mixed_user_text_keeps_text_and_strips_tags() {
        assert_eq!(
            classify_user_entry(
                "<system-reminder>ctx</system-reminder>\nFix the bug",
                false,
                false
            ),
            UserEntry::User("Fix the bug".into())
        );
        assert_eq!(
            classify_user_entry("Use <b>bold</b> please", false, false),
            UserEntry::User("Use <b>bold</b> please".into())
        );
    }
}
