//! Per-agent log parsers that turn raw bytes into normalized events. Pure and deterministic.
//!
//! See `docs/SPEC.md` for the design. This crate is a scaffold; implementation starts in M1.

#[cfg(test)]
mod tests {
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

            // Verify zero personal paths or user identifiers leaked into the fixture.
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
}
