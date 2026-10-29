//! Pre-storage activity summary redaction engine (`SPEC §3.1`, Invariant 1).
//!
//! Activity summaries exclude prompts, file contents and secrets. Every short
//! summary (`<= 120` chars) passes through [`redact_summary`] before creating an
//! [`crate::ActivityEvent`]. Approved local conversation storage uses separate
//! chat redaction and injected-context cleanup.

/// Maximum character length of a persisted activity summary (`SPEC §4.1`).
pub const MAX_SUMMARY_CHARS: usize = 120;

/// Removes leading injected agent context while retaining any following user prompt.
#[must_use]
pub fn clean_user_text(mut text: &str) -> &str {
    loop {
        let trimmed = text.trim_start();
        let closing = if trimmed.starts_with("# AGENTS.md instructions for ") {
            "</INSTRUCTIONS>"
        } else if trimmed.starts_with("<environment_context>") {
            "</environment_context>"
        } else if trimmed.starts_with("<user_instructions>") {
            "</user_instructions>"
        } else if trimmed.starts_with("<external_codex_apps_open_page>") {
            "</external_codex_apps_open_page>"
        } else {
            return text;
        };
        let Some((_, remaining)) = trimmed.split_once(closing) else {
            return "";
        };
        text = remaining.trim_start();
    }
}

/// Redacts secrets, tokens, and sensitive environment variables from a tool
/// activity summary, collapses whitespace onto a single line, and truncates to
/// [`MAX_SUMMARY_CHARS`].
#[must_use]
pub fn redact_summary(tool_name: &str, raw_detail: &str) -> String {
    let mut tokens_out = Vec::new();
    let mut mask_next = false;

    for raw_word in raw_detail.split_whitespace() {
        if mask_next {
            tokens_out.push("<REDACTED>".to_owned());
            mask_next = false;
            continue;
        }

        let trimmed = raw_word.trim_matches(|c: char| matches!(c, '\'' | '"' | ',' | ';'));
        let lower = trimmed.to_ascii_lowercase();

        if lower == "bearer"
            || lower == "--token"
            || lower == "--with-token"
            || lower == "--api-key"
            || lower == "--password"
        {
            tokens_out.push(trimmed.to_owned());
            mask_next = true;
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            if is_sensitive_env_key(k) || looks_like_secret_token(v) {
                tokens_out.push(format!("{k}=<REDACTED>"));
                continue;
            }
        }

        if looks_like_secret_token(trimmed) {
            tokens_out.push("<REDACTED>".to_owned());
        } else {
            tokens_out.push(raw_word.to_owned());
        }
    }

    let cleaned_detail = tokens_out.join(" ");
    let clean_tool = tool_name.trim();
    let full = if cleaned_detail.is_empty() {
        clean_tool.to_owned()
    } else if clean_tool.is_empty() {
        cleaned_detail
    } else {
        format!("{clean_tool}: {cleaned_detail}")
    };

    truncate_summary(&full, MAX_SUMMARY_CHARS)
}

/// Redacts secrets, tokens, API keys and credentials from conversation text
/// without truncating the message length.
#[must_use]
pub fn redact_chat_content(raw: &str) -> String {
    let mut tokens_out = Vec::new();
    let mut mask_next = false;

    for raw_word in raw.split_whitespace() {
        if mask_next {
            tokens_out.push("<REDACTED>".to_owned());
            mask_next = false;
            continue;
        }

        let trimmed =
            raw_word.trim_matches(|c: char| matches!(c, '\'' | '"' | ',' | ';' | '(' | ')'));
        let lower = trimmed.to_ascii_lowercase();

        if lower == "bearer"
            || lower == "--token"
            || lower == "--with-token"
            || lower == "--api-key"
            || lower == "--password"
        {
            tokens_out.push(trimmed.to_owned());
            mask_next = true;
            continue;
        }

        if let Some((k, v)) = trimmed.split_once('=') {
            if is_sensitive_env_key(k) || looks_like_secret_token(v) {
                tokens_out.push(format!("{k}=<REDACTED>"));
                continue;
            }
        }

        if looks_like_secret_token(trimmed) {
            tokens_out.push("<REDACTED>".to_owned());
        } else {
            tokens_out.push(raw_word.to_owned());
        }
    }

    tokens_out.join(" ")
}

fn is_sensitive_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    upper.contains("KEY")
        || upper.contains("TOKEN")
        || upper.contains("SECRET")
        || upper.contains("PASSWORD")
        || upper.contains("PASSWD")
        || upper.contains("CREDENTIAL")
        || upper.contains("AUTH")
}

fn looks_like_secret_token(val: &str) -> bool {
    let s = val.trim_matches(|c: char| matches!(c, '\'' | '"' | '(' | ')' | '[' | ']'));
    if s.len() < 12 {
        return false;
    }

    let prefixes = [
        "sk-",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "github_pat_",
        "AKIA",
        "ASIA",
        "xoxb-",
        "xoxp-",
        "xoxa-",
        "AIza",
        "eyJ",
        "sbp_",
    ];
    if prefixes.iter().any(|p| s.starts_with(p)) {
        return true;
    }

    // Long contiguous non-path token (>36 chars without slashes or spaces) with mixed char classes
    if s.len() >= 36 && !s.contains('/') && !s.contains('\\') {
        let has_alpha = s.chars().any(|c| c.is_ascii_alphabetic());
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        if has_alpha && has_digit {
            return true;
        }
    }

    false
}

fn truncate_summary(input: &str, max_chars: usize) -> String {
    let count = input.chars().count();
    if count <= max_chars {
        return input.to_owned();
    }
    let keep = max_chars.saturating_sub(3);
    let prefix: String = input.chars().take(keep).collect();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::{redact_chat_content, redact_summary};

    #[test]
    fn redacts_common_api_keys_and_tokens() {
        let s = redact_summary(
            "run_command",
            "curl -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U' https://api.example.com",
        );
        assert!(!s.contains("eyJhbGci"));
        assert!(s.contains("<REDACTED>"));

        let s2 = redact_summary(
            "run_command",
            "OPENAI_API_KEY=sk-proj-1234567890abcdef1234567890abcdef cargo test",
        );
        assert!(!s2.contains("sk-proj-"));
        assert!(s2.contains("<REDACTED>"));

        let s3 = redact_summary(
            "run_command",
            "gh auth login --with-token ghp_1234567890abcdefghijklmnopqrstuvwx",
        );
        assert!(!s3.contains("ghp_12345"));
        assert!(s3.contains("<REDACTED>"));
    }

    #[test]
    fn redacts_env_assignments_for_sensitive_keys() {
        let s = redact_summary(
            "run_command",
            "DATABASE_PASSWORD=my_super_secret_pw_999 psql -h localhost",
        );
        assert!(!s.contains("my_super_secret_pw_999"));
        assert!(s.contains("DATABASE_PASSWORD=<REDACTED>"));
    }

    #[test]
    fn preserves_clean_tool_and_file_summaries() {
        let s = redact_summary("edit_file", "crates/core/src/lib.rs");
        assert_eq!(s, "edit_file: crates/core/src/lib.rs");

        let s2 = redact_summary("run_command", "cargo test --workspace");
        assert_eq!(s2, "run_command: cargo test --workspace");
    }

    #[test]
    fn collapses_whitespace_and_truncates_to_120_chars() {
        let long_arg = "a".repeat(200);
        let s = redact_summary("read_file", &format!("path/to/\n\t{long_arg}"));
        assert!(s.chars().count() <= 120);
        assert!(!s.contains('\n'));
        assert!(s.ends_with("..."));
    }

    #[test]
    fn redacts_chat_content_without_length_truncation() {
        let text = "Here is my key: sk-ant-api03-1234567890abcdefghijklmnopqrstuvwxyz and my database password: DATABASE_PASSWORD=secret1234567890. Please help me refactor this 300 character long sentence which should never be cut short or truncated to 120 characters under any circumstance whatsoever because full user explanations need to be preserved.";
        let redacted = redact_chat_content(text);
        assert!(!redacted.contains("sk-ant-api03"));
        assert!(!redacted.contains("secret1234567890"));
        assert!(redacted.contains("<REDACTED>"));
        assert!(redacted.contains("DATABASE_PASSWORD=<REDACTED>"));
        assert!(redacted.chars().count() > 200);
    }
}
