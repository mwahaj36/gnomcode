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

#[test]
fn doctor_json_reports_all_checks_and_exits_nonzero_on_fail_without_leaking_tokens() {
    let root = tempfile::tempdir().expect("root");
    let expired_ms = (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp_millis();
    std::fs::write(
        root.path().join(".credentials.json"),
        serde_json::json!({
            "claudeAiOauth": {
                "accessToken": "super-secret-oauth-token",
                "refreshToken": "super-secret-refresh-token",
                "expiresAt": expired_ms
            }
        })
        .to_string(),
    )
    .expect("creds");

    let output = Command::new(env!("CARGO_BIN_EXE_gnomcode"))
        .args(["doctor", "--json"])
        .env("GNOMCODE_HOME", root.path())
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .env("APPDATA", root.path())
        .env("CLAUDE_CONFIG_DIR", root.path())
        .env("CODEX_HOME", root.path())
        .env("GEMINI_HOME", root.path())
        .output()
        .expect("doctor --json");
    assert!(!output.status.success(), "expired sign-in must fail doctor");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("super-secret-oauth-token"));
    assert!(!stdout.contains("super-secret-refresh-token"));
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("doctor json");
    assert_eq!(parsed["status"], "fail");
    assert_eq!(parsed["ok"], false);
    let checks = parsed["checks"].as_array().expect("checks array");
    assert_eq!(checks.len(), 7);
    let signin = checks
        .iter()
        .find(|c| c["id"] == "claude_signin")
        .expect("claude_signin check");
    assert_eq!(signin["status"], "fail");
    assert!(signin["fix"].as_str().is_some_and(|s| !s.is_empty()));
}

#[test]
fn statusline_records_only_limits_and_never_fails() {
    use std::io::Write as _;
    let home = tempfile::tempdir().expect("home");
    let run = |input: &str| {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gnomcode"))
            .arg("statusline")
            .env("GNOMCODE_HOME", home.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("statusline");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(input.as_bytes())
            .expect("write");
        child.wait_with_output().expect("output")
    };
    let output = run(
        r#"{"session_id":"secret-session","cwd":"/private","rate_limits":{"five_hour":{"used_percentage":58,"resets_at":1794480000},"seven_day":{"used_percentage":20}}}"#,
    );
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "5h 42% left · week 80% left"
    );
    let saved = std::fs::read_to_string(home.path().join("claude-limits.json")).expect("limits");
    assert!(
        saved.contains("rate_limits")
            && !saved.contains("secret-session")
            && !saved.contains("/private")
    );
    assert!(run("not json").status.success());
}

#[test]
fn integrate_keeps_a_custom_status_line_and_removes_only_its_own() {
    let config = tempfile::tempdir().expect("config");
    let settings = config.path().join("settings.json");
    let integrate = |remove: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_gnomcode"));
        command
            .args(["integrate", "claude"])
            .env("CLAUDE_CONFIG_DIR", config.path());
        if remove {
            command.arg("--remove");
        }
        assert!(command.output().expect("integrate").status.success());
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(&settings).expect("settings"))
            .expect("json")
    };
    let installed = integrate(false);
    assert!(
        installed["statusLine"]["command"]
            .as_str()
            .is_some_and(|command| command.ends_with(" statusline"))
    );
    assert!(integrate(true).get("statusLine").is_none());

    std::fs::write(
        &settings,
        r#"{"statusLine":{"type":"command","command":"my-line"}}"#,
    )
    .expect("custom");
    assert_eq!(integrate(false)["statusLine"]["command"], "my-line");
    assert_eq!(integrate(true)["statusLine"]["command"], "my-line");
}
