//! Phase 2 PTY launcher, models, and process lifecycle / fail-open tests.

use std::process::Command;
use std::time::Duration;

#[test]
fn models_command_runs_offline_and_lists_catalog() {
    let root = tempfile::tempdir().expect("root");
    let binary = env!("CARGO_BIN_EXE_spawnt");
    let output = Command::new(binary)
        .arg("models")
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("models command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("claude-code"));
    assert!(stdout.contains("codex"));
    assert!(stdout.contains("gemini-cli"));
}

#[test]
fn launch_command_rejects_unsafe_permission_escalation_flags() {
    let repo = tempfile::tempdir().expect("repo");
    for bad_flag in ["--dangerously-skip-permissions", "-yolo", ""] {
        assert!(spawnt_pty::launch_command("claude", repo.path(), Some(bad_flag)).is_err());
    }
}

#[tokio::test]
async fn fail_open_agent_process_survives_independent_lifecycle() {
    let dir = tempfile::tempdir().expect("dir");
    let session_id = format!("failopen-{}", std::process::id());

    #[cfg(windows)]
    let override_cmd = Some(("cmd.exe", &["/c", "ping -n 30 127.0.0.1 >nul"][..]));
    #[cfg(not(windows))]
    let override_cmd = Some(("sh", &["-c", "sleep 30"][..]));

    let s_id = session_id.clone();
    let repo = dir.path().to_path_buf();

    // Start independent PTY host
    let host_handle = tokio::spawn(async move {
        let _ = spawnt_pty::run_pty_host(&s_id, "claude", &repo, None, override_cmd).await;
    });

    // Wait for session record
    let mut record = None;
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Ok(r) = spawnt_pty::load_session_record(&session_id) {
            record = Some(r);
            break;
        }
    }
    let record = record.expect("session record exists");
    let child_pid = record.child_pid;
    assert!(child_pid > 0);

    // Verify child is running
    assert!(spawnt_pty::is_pid_alive(child_pid));

    // Stop session cleanly via authenticated stop endpoint
    spawnt_pty::stop_session(&session_id)
        .await
        .expect("stop session");
    let _ = host_handle.await;

    // Verify cleanup
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(spawnt_pty::load_session_record(&session_id).is_err());
}

#[test]
fn policy_command_gets_and_sets_routing_mode() {
    let root = tempfile::tempdir().expect("root");
    let binary = env!("CARGO_BIN_EXE_spawnt");

    // 1. Get default policy
    let get_out = Command::new(binary)
        .arg("policy")
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("get policy");
    assert!(get_out.status.success());
    let get_str = String::from_utf8_lossy(&get_out.stdout);
    assert!(get_str.contains("ask_first"));

    // 2. Set policy to automatic
    let set_out = Command::new(binary)
        .args(["policy", "--set", "automatic"])
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("set policy");
    assert!(set_out.status.success());
    let set_str = String::from_utf8_lossy(&set_out.stdout);
    assert!(set_str.contains("automatic"));

    // 3. Confirm policy persisted
    let confirm_out = Command::new(binary)
        .arg("policy")
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("confirm policy");
    assert!(confirm_out.status.success());
    let confirm_str = String::from_utf8_lossy(&confirm_out.stdout);
    assert!(confirm_str.contains("automatic"));
}

#[test]
fn switch_and_resume_commands_execute() {
    let root = tempfile::tempdir().expect("root");
    let binary = env!("CARGO_BIN_EXE_spawnt");

    // 1. Switch
    let switch_out = Command::new(binary)
        .args(["switch", "sess-test", "--model", "o3"])
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("switch");
    assert!(switch_out.status.success());
    let switch_str = String::from_utf8_lossy(&switch_out.stdout);
    assert!(switch_str.contains("Zero-token handoff compiled"));
    assert!(switch_str.contains("o3"));

    // 2. Resume
    let resume_out = Command::new(binary)
        .args(["resume", "sess-test", "--prompt", "continue work"])
        .env("SPAWNT_HOME", root.path())
        .output()
        .expect("resume");
    assert!(resume_out.status.success());
    let resume_str = String::from_utf8_lossy(&resume_out.stdout);
    assert!(resume_str.contains("armed in Resume Queue"));
}
