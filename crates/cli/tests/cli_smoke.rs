//! CLI smoke tests.

use std::process::Command;

#[test]
fn gnomcode_binary_shows_product_version() {
    {
        let binary = env!("CARGO_BIN_EXE_gnomcode");
        let output = Command::new(binary)
            .arg("--version")
            .output()
            .expect("version");
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("gnomcode {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

#[test]
fn gnomcode_cli_runs_offline_with_custom_home() {
    let root = tempfile::tempdir().expect("root");
    {
        let binary = env!("CARGO_BIN_EXE_gnomcode");
        let output = Command::new(binary)
            .arg("doctor")
            .env("GNOMCODE_HOME", root.path())
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
    let output = Command::new(env!("CARGO_BIN_EXE_gnomcode"))
        .args(["doctor", "--network"])
        .env("GNOMCODE_HOME", root.path())
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
