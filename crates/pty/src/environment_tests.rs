//! Child-environment regression with no global environment mutation or native agent access.

#[tokio::test]
async fn independent_agent_drops_parent_markers_but_preserves_provider_and_restrictions() {
    use super::*;
    if std::env::var_os("GNOMCODE_TEST_AGENT_ENV_CHILD").is_none() {
        let home = tempfile::tempdir().expect("isolated home");
        let binary = home
            .path()
            .join(format!("fake-agent{}", std::env::consts::EXE_SUFFIX));
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates")
            .parent()
            .expect("workspace")
            .join("fixtures")
            .join("platform")
            .join("agent-environment.rs");
        let compiled =
            std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .arg("--edition=2024")
                .arg(source)
                .arg("-o")
                .arg(&binary)
                .output()
                .expect("fixture compiler");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let output = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args(["--exact", "environment_tests::independent_agent_drops_parent_markers_but_preserves_provider_and_restrictions", "--nocapture"])
            .env("GNOMCODE_TEST_AGENT_ENV_CHILD", &binary).env("GNOMCODE_HOME", home.path())
            .env("CLAUDECODE", "1").env("CLAUDE_CODE_SESSION_ID", "test-parent").env("CLAUDE_CODE_CHILD_SESSION", "1")
            .env("CLAUDE_CODE_USE_BEDROCK", "test-provider").env("CLAUDE_CODE_CHROME_MCP_ORG_DENIED", "test-denied")
            .output().expect("isolated test process");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let binary = std::env::var("GNOMCODE_TEST_AGENT_ENV_CHILD").expect("fixture executable");
    let workspace = tempfile::tempdir().expect("workspace");
    let report = workspace.path().join("environment.txt");
    let output = report.to_string_lossy().into_owned();
    let root = workspace.path().to_path_buf();
    let host = tokio::spawn(async move {
        run_pty_host(
            "environment-test",
            "claude",
            &root,
            None,
            Some((&binary, &[output.as_str()])),
        )
        .await
    });
    let mut record = None;
    for _ in 0..100 {
        if report.exists() {
            record = load_session_record("environment-test").ok();
            if record.is_some() {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let record = record.expect("host and environment report");
    stop_session(&record.session_id)
        .await
        .expect("stop owned fake agent");
    host.await.expect("host task").expect("host shutdown");
    assert_eq!(
        std::fs::read_to_string(report).expect("report"),
        "clean=true;provider=true;restriction=true"
    );
    assert_eq!(
        std::env::var("CLAUDECODE").expect("parent marker"),
        "1",
        "parent environment must remain unchanged"
    );
}
