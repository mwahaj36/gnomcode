//! Claude Code hook entries written by `gnomcode integrate claude` (ADR 0032).
//!
//! Gnomcode only ever adds or removes its own handlers. Other handlers, including ones in
//! the same matcher group, are left exactly as they were.

use anyhow::{Context, Result};
use gnomcode_core::approval::HOOK_TIMEOUT_SECS;
use serde_json::{Value, json};

/// Events Gnomcode registers a handler for.
const EVENTS: [&str; 3] = ["PreToolUse", "PermissionRequest", "Notification"];

fn is_gnomcode_handler(handler: &Value, exe: &str) -> bool {
    let command = handler.get("command").and_then(Value::as_str).unwrap_or("");
    let ours = command == exe
        || std::path::Path::new(command)
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| ["gnomcode", "gnomcode.exe"].contains(&name));
    ours && handler.get("args") == Some(&json!(["hook"]))
}

fn entry(matcher: Option<&str>, exe: &str, timeout: u64) -> Value {
    let mut group = json!({"hooks": [{"type": "command", "command": exe, "args": ["hook"], "timeout": timeout, "onFailure": "continue"}]});
    if let Some(matcher) = matcher {
        group["matcher"] = json!(matcher);
    }
    group
}

/// Adds (or, with `remove`, removes) Gnomcode's hook handlers in Claude `settings`.
///
/// `approvals` controls the `PermissionRequest` and `Notification` handlers; the
/// `PreToolUse` guardrail handler is always managed. Idempotent.
///
/// # Errors
/// When `hooks` or one of its events has an unexpected shape.
pub(crate) fn apply_hooks(
    settings: &mut Value,
    exe: &str,
    remove: bool,
    approvals: bool,
) -> Result<()> {
    for event in EVENTS {
        if let Some(groups) = settings.pointer_mut(&format!("/hooks/{event}")) {
            let groups = groups
                .as_array_mut()
                .with_context(|| format!("Claude {event} hooks must be an array"))?;
            for group in groups.iter_mut() {
                if let Some(handlers) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                    handlers.retain(|handler| !is_gnomcode_handler(handler, exe));
                }
            }
            groups.retain(|group| {
                group
                    .get("hooks")
                    .and_then(Value::as_array)
                    .is_none_or(|handlers| !handlers.is_empty())
            });
        }
    }
    if !remove {
        let wanted = [
            ("PreToolUse", entry(None, exe, 1), true),
            (
                "PermissionRequest",
                entry(None, exe, HOOK_TIMEOUT_SECS),
                approvals,
            ),
            (
                "Notification",
                entry(Some("permission_prompt"), exe, 2),
                approvals,
            ),
        ];
        for (event, group, enabled) in wanted {
            if !enabled {
                continue;
            }
            if settings.get("hooks").is_none() {
                settings["hooks"] = json!({});
            }
            let hooks = settings["hooks"]
                .as_object_mut()
                .context("Claude hooks must be an object")?;
            hooks
                .entry(event)
                .or_insert_with(|| json!([]))
                .as_array_mut()
                .with_context(|| format!("Claude {event} hooks must be an array"))?
                .push(group);
        }
    }
    // Drop event arrays that Gnomcode emptied; never touch ones it did not.
    if let Some(hooks) = settings.get_mut("hooks").and_then(Value::as_object_mut) {
        for event in EVENTS {
            if hooks
                .get(event)
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                hooks.remove(event);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = "C:/tools/gnomcode.exe";

    fn gnomcode_count(settings: &Value, event: &str) -> usize {
        settings["hooks"][event].as_array().map_or(0, |groups| {
            groups
                .iter()
                .flat_map(|g| g["hooks"].as_array().cloned().unwrap_or_default())
                .filter(|h| is_gnomcode_handler(h, EXE))
                .count()
        })
    }

    #[test]
    fn adds_all_three_hooks_once_and_is_idempotent() {
        let mut settings = json!({});
        apply_hooks(&mut settings, EXE, false, true).unwrap();
        apply_hooks(&mut settings, EXE, false, true).unwrap();
        for event in EVENTS {
            assert_eq!(gnomcode_count(&settings, event), 1, "{event}");
        }
        let perm = &settings["hooks"]["PermissionRequest"][0]["hooks"][0];
        assert_eq!(perm["timeout"], HOOK_TIMEOUT_SECS);
        assert_eq!(perm["onFailure"], "continue");
        assert_eq!(
            settings["hooks"]["Notification"][0]["matcher"],
            "permission_prompt"
        );
    }

    #[test]
    fn existing_hooks_are_chained_not_replaced() {
        let mut settings = json!({"hooks": {
            "PermissionRequest": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "mine.sh"}]}],
            "Notification": [{"hooks": [
                {"type": "command", "command": "notify.sh"},
                {"type": "command", "command": EXE, "args": ["hook"]}
            ]}]
        }, "theme": "dark"});
        apply_hooks(&mut settings, EXE, false, true).unwrap();
        assert_eq!(
            settings["hooks"]["PermissionRequest"][0]["hooks"][0]["command"],
            "mine.sh"
        );
        assert_eq!(gnomcode_count(&settings, "PermissionRequest"), 1);
        // The user's handler that shared a group with an older Gnomcode handler survives.
        assert_eq!(
            settings["hooks"]["Notification"][0]["hooks"][0]["command"],
            "notify.sh"
        );
        apply_hooks(&mut settings, EXE, true, true).unwrap();
        assert_eq!(
            settings["hooks"]["PermissionRequest"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            settings["hooks"]["PermissionRequest"][0]["hooks"][0]["command"],
            "mine.sh"
        );
        assert_eq!(
            settings["hooks"]["Notification"][0]["hooks"][0]["command"],
            "notify.sh"
        );
        assert_eq!(settings["theme"], "dark");
        for event in EVENTS {
            assert_eq!(gnomcode_count(&settings, event), 0);
        }
        assert!(settings["hooks"].get("PreToolUse").is_none());
    }

    #[test]
    fn remove_leaves_other_events_and_unrelated_handlers_untouched() {
        let original = json!({"hooks": {"PostToolUse": [{"hooks": [{"type": "command", "command": "fmt.sh"}]}]}});
        let mut settings = original.clone();
        apply_hooks(&mut settings, EXE, false, true).unwrap();
        apply_hooks(&mut settings, EXE, true, true).unwrap();
        assert_eq!(settings, original);
    }

    #[test]
    fn opting_out_of_approvals_removes_only_those_hooks() {
        let mut settings = json!({});
        apply_hooks(&mut settings, EXE, false, true).unwrap();
        apply_hooks(&mut settings, EXE, false, false).unwrap();
        assert_eq!(gnomcode_count(&settings, "PreToolUse"), 1);
        assert_eq!(gnomcode_count(&settings, "PermissionRequest"), 0);
        assert_eq!(gnomcode_count(&settings, "Notification"), 0);
    }

    #[test]
    fn malformed_event_is_an_error_not_a_rewrite() {
        let mut settings = json!({"hooks": {"PermissionRequest": "nope"}});
        assert!(apply_hooks(&mut settings, EXE, false, true).is_err());
    }
}
