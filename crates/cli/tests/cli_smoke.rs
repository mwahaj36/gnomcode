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

#[test]
fn doctor_network_discloses_opt_in_remote_and_pricing_without_connecting() {
    let root = tempfile::tempdir().expect("root");
    let output = Command::new(env!("CARGO_BIN_EXE_spawnt"))
        .args(["doctor", "--network"])
        .env("SPAWNT_HOME", root.path())
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("APPDATA", root.path())
        .env("CLAUDE_CONFIG_DIR", root.path())
        .env("CODEX_HOME", root.path())
        .env("GEMINI_HOME", root.path())
        .output()
        .expect("doctor");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Remote relay"));
    assert!(text.contains("pricing"));
    assert!(text.contains("agent CLI"));
}

#[test]
fn aider_discovery_requires_the_explicit_log_override() {
    let root = tempfile::tempdir().expect("root");
    let aider_dir = root.path().join(".aider");
    std::fs::create_dir(&aider_dir).expect("dir");
    let log = aider_dir.join("analytics.jsonl");
    std::fs::write(&log, "{}\n").expect("log");
    let run = |enabled: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_spawnt"));
        command
            .arg("doctor")
            .env("SPAWNT_HOME", root.path())
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("APPDATA", root.path())
            .env("CLAUDE_CONFIG_DIR", root.path())
            .env("CODEX_HOME", root.path())
            .env("GEMINI_HOME", root.path())
            .env_remove("AIDER_ANALYTICS_LOG");
        if enabled {
            command.env("AIDER_ANALYTICS_LOG", &log);
        }
        command.output().expect("doctor")
    };
    let disabled = run(false);
    assert!(disabled.status.success());
    assert!(!String::from_utf8_lossy(&disabled.stdout).contains("aider"));
    let enabled = run(true);
    assert!(enabled.status.success());
    assert!(String::from_utf8_lossy(&enabled.stdout).contains("aider"));
}
