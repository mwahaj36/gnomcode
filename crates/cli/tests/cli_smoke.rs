//! CLI smoke tests.

use std::process::Command;

#[test]
fn spawnt_binary_shows_product_version() {
    {
        let binary = env!("CARGO_BIN_EXE_spawnt");
        let output = Command::new(binary)
            .arg("--version")
            .output()
            .expect("version");
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("spawnt {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn spawnt_cli_runs_offline_with_custom_home() {
    let root = tempfile::tempdir().expect("root");
    {
        let binary = env!("CARGO_BIN_EXE_spawnt");
        let output = Command::new(binary)
            .arg("doctor")
            .env("SPAWNT_HOME", root.path())
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("APPDATA", root.path())
            .env("CLAUDE_CONFIG_DIR", root.path())
            .env("CODEX_HOME", root.path())
            .env("GEMINI_HOME", root.path())
            .output()
            .expect("doctor");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("Diagnostics complete."));
    }
}
