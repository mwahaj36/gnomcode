//! Subagent display names and the parent state rolled up from running children.

/// Generic labels that carry no information about what the subagent does.
const GENERIC: [&str; 3] = ["subagent", "unknown", "untitled"];

/// Display name for a subagent: its recorded name or task description, never the bare word
/// "Subagent". `ordinal` is the 1-based position among its siblings, used as the fallback.
#[must_use]
pub fn subagent_name(title: Option<&str>, ordinal: usize) -> String {
    let title = title.map_or("", str::trim);
    let name = title.strip_suffix("· Subagent").unwrap_or(title).trim();
    if name.is_empty() || GENERIC.contains(&name.to_lowercase().as_str()) {
        format!("Subagent {ordinal}")
    } else {
        name.to_owned()
    }
}

/// Parent label while `running` children execute, for example `Working · 5 subagents running`.
#[must_use]
pub fn working_label(running: usize) -> String {
    match running {
        0 => "Working".to_owned(),
        1 => "Working · 1 subagent running".to_owned(),
        n => format!("Working · {n} subagents running"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_use_the_recorded_description_and_fall_back_to_the_ordinal() {
        assert_eq!(
            subagent_name(Some("Wire watcher into daemon"), 1),
            "Wire watcher into daemon"
        );
        assert_eq!(subagent_name(Some("explorer · Subagent"), 2), "explorer");
        assert_eq!(subagent_name(Some("Subagent"), 3), "Subagent 3");
        assert_eq!(subagent_name(Some("  unknown "), 4), "Subagent 4");
        assert_eq!(subagent_name(None, 5), "Subagent 5");
        assert_eq!(subagent_name(Some("· Subagent"), 6), "Subagent 6");
    }

    #[test]
    fn working_label_counts_running_children() {
        assert_eq!(working_label(0), "Working");
        assert_eq!(working_label(1), "Working · 1 subagent running");
        assert_eq!(working_label(5), "Working · 5 subagents running");
    }
}
