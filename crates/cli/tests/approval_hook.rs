//! `gnomcode hook` for Claude `PermissionRequest`, and `integrate claude` hook management.

#![allow(clippy::expect_used)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn payload() -> String {
    serde_json::json!({
        "session_id": "sess-hook-1",
        "hook_event_name": "PermissionRequest",
        "permission_mode": "default",
        "tool_name": "Bash",
        "tool_input": {"command": "rm -rf node_modules && echo TOKEN=abcdefghijklmnop1234567890abcdefghijkl", "description": "clean"}
    })
    .to_string()
}

fn run_hook(home: &std::path::Path, input: &str) -> (String, bool, Duration) {
    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_gnomcode"))
        .arg("hook")
        .env("GNOMCODE_HOME", home)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(input.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("output");
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        output.status.success(),
        started.elapsed(),
    )
}

fn write_daemon_info(home: &std::path::Path, port: u16) {
    std::fs::write(
        home.join("daemon.json"),
        serde_json::json!({"pid": 1, "port": port, "token": "t0ken", "started_at": "x"})
            .to_string(),
    )
    .expect("daemon.json");
}

/// Answers each connection with the next canned JSON body; returns the requests it saw.
fn fake_daemon(replies: Vec<&'static str>) -> (u16, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = std::thread::spawn(move || {
        let mut seen = Vec::new();
        for body in replies {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = vec![0_u8; 8192];
            let mut len = 0;
            loop {
                let n = stream.read(&mut buf[len..]).expect("read");
                len += n;
                let text = String::from_utf8_lossy(&buf[..len]);
                if n == 0
                    || text.contains("\r\n\r\n")
                        && text.matches('{').count() == text.matches('}').count()
                {
                    break;
                }
            }
            seen.push(String::from_utf8_lossy(&buf[..len]).into_owned());
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).expect("write");
        }
        seen
    });
    (port, handle)
}

#[test]
fn hook_without_a_daemon_prints_nothing_and_exits_zero() {
    let home = tempfile::tempdir().expect("home");
    let (out, ok, _) = run_hook(home.path(), &payload());
    assert!(ok);
    assert!(out.trim().is_empty());

    // A stale daemon.json whose port is closed fails open quickly too.
    let closed = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = closed.local_addr().expect("addr").port();
    drop(closed);
    write_daemon_info(home.path(), port);
    let (out, ok, elapsed) = run_hook(home.path(), &payload());
    assert!(ok);
    assert!(out.trim().is_empty());
    assert!(elapsed < Duration::from_secs(3), "{elapsed:?}");
}

#[test]
fn hook_prints_claude_decision_only_for_an_explicit_answer() {
    for (outcome, expected) in [
        ("allow", Some("allow")),
        ("deny", Some("deny")),
        ("none", None),
    ] {
        let home = tempfile::tempdir().expect("home");
        let wait = match outcome {
            "allow" => r#"{"outcome":"allow"}"#,
            "deny" => r#"{"outcome":"deny"}"#,
            _ => r#"{"outcome":"none"}"#,
        };
        let (port, daemon) = fake_daemon(vec![
            r#"{"hold":true,"id":"abcdef0123456789","wait_secs":90}"#,
            wait,
        ]);
        write_daemon_info(home.path(), port);
        let (out, ok, _) = run_hook(home.path(), &payload());
        assert!(ok);
        let seen = daemon.join().expect("daemon");
        assert!(seen[0].starts_with("POST /api/v1/approvals "));
        assert!(seen[0].contains("Authorization: Bearer t0ken"));
        assert!(seen[0].contains("\"tool\":\"Bash\""));
        // Only the redacted summary leaves the hook, never the raw secret or tool input.
        assert!(!seen[0].contains("abcdefghijklmnop"));
        assert!(!seen[0].contains("tool_input"));
        assert!(seen[1].starts_with("GET /api/v1/approvals/abcdef0123456789/wait "));
        match expected {
            Some(behavior) => {
                let value: serde_json::Value = serde_json::from_str(out.trim()).expect("json");
                assert_eq!(
                    value["hookSpecificOutput"]["hookEventName"],
                    "PermissionRequest"
                );
                assert_eq!(
                    value["hookSpecificOutput"]["decision"]["behavior"],
                    behavior
                );
            }
            None => assert!(out.trim().is_empty(), "no decision must print nothing"),
        }
    }
}

#[test]
fn hook_stops_waiting_when_the_prompt_is_answered_at_the_terminal() {
    let home = tempfile::tempdir().expect("home");
    let transcript = home.path().join("session.jsonl");
    std::fs::write(&transcript, "{\"type\":\"assistant\"}\n").expect("transcript");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let daemon = std::thread::spawn(move || {
        let (mut first, _) = listener.accept().expect("accept");
        let mut buf = [0_u8; 4096];
        let _ = first.read(&mut buf);
        let body = r#"{"hold":true,"id":"abcdef0123456789"}"#;
        let reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        first.write_all(reply.as_bytes()).expect("reply");
        // The wait request is accepted and then left hanging, like an undecided approval.
        let (mut second, _) = listener.accept().expect("accept wait");
        let _ = second.read(&mut buf);
        std::thread::sleep(Duration::from_secs(8));
    });
    write_daemon_info(home.path(), port);
    let appender = {
        let transcript = transcript.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(1200));
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(transcript)
                .expect("open");
            file.write_all(b"{\"type\":\"progress\"}\n{\"type\":\"user\",\"message\":{\"content\":[{\"type\":\"tool_result\"}]}}\n")
                .expect("append");
        })
    };
    let mut input: serde_json::Value = serde_json::from_str(&payload()).expect("payload");
    input["transcript_path"] = serde_json::json!(transcript.to_string_lossy());
    let (out, ok, elapsed) = run_hook(home.path(), &input.to_string());
    appender.join().expect("appender");
    assert!(ok);
    assert!(
        out.trim().is_empty(),
        "an answer at the terminal prints no decision"
    );
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    drop(daemon);
}

#[test]
fn hook_does_not_wait_when_the_daemon_says_nobody_can_answer() {
    let home = tempfile::tempdir().expect("home");
    // Warm the freshly built binary first so first-launch scanning is not timed.
    let warm = tempfile::tempdir().expect("warm");
    run_hook(warm.path(), "{}");
    let (port, daemon) = fake_daemon(vec![r#"{"hold":false,"reason":"no_listener"}"#]);
    write_daemon_info(home.path(), port);
    let (out, ok, elapsed) = run_hook(home.path(), &payload());
    assert!(ok);
    assert!(out.trim().is_empty());
    assert_eq!(daemon.join().expect("daemon").len(), 1);
    assert!(elapsed < Duration::from_secs(3));
}

#[test]
fn hook_never_trusts_a_malformed_id_from_the_daemon() {
    let home = tempfile::tempdir().expect("home");
    let (port, daemon) = fake_daemon(vec![r#"{"hold":true,"id":"../../x"}"#]);
    write_daemon_info(home.path(), port);
    let (out, ok, _) = run_hook(home.path(), &payload());
    assert!(ok);
    assert!(out.trim().is_empty());
    assert_eq!(daemon.join().expect("daemon").len(), 1);
}

#[test]
fn hook_survives_garbage_input() {
    let home = tempfile::tempdir().expect("home");
    for input in [
        "",
        "not json",
        "{}",
        r#"{"hook_event_name":"PermissionRequest"}"#,
    ] {
        let (out, ok, _) = run_hook(home.path(), input);
        assert!(ok, "{input}");
        assert!(out.trim().is_empty());
    }
}

#[test]
fn notification_hook_posts_only_permission_prompts() {
    let home = tempfile::tempdir().expect("home");
    let (port, daemon) = fake_daemon(vec![r#"{"ok":true}"#]);
    write_daemon_info(home.path(), port);
    let idle = serde_json::json!({"session_id":"s","hook_event_name":"Notification","notification_type":"idle_prompt","message":"x"});
    let (out, ok, _) = run_hook(home.path(), &idle.to_string());
    assert!(ok && out.trim().is_empty());
    let prompt = serde_json::json!({"session_id":"s","hook_event_name":"Notification","notification_type":"permission_prompt","message":"Claude needs your permission"});
    let (out, ok, _) = run_hook(home.path(), &prompt.to_string());
    assert!(ok && out.trim().is_empty());
    let seen = daemon.join().expect("daemon");
    assert_eq!(seen.len(), 1);
    assert!(seen[0].starts_with("POST /api/v1/approvals/notify "));
    assert!(!seen[0].contains("needs your permission"));
}

fn integrate(config: &std::path::Path, extra: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_gnomcode"))
        .args(["integrate", "claude"])
        .args(extra)
        .env("CLAUDE_CONFIG_DIR", config)
        .env("GNOMCODE_HOME", config)
        .output()
        .expect("integrate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn integrate_backs_up_chains_and_removes_approval_hooks() {
    let config = tempfile::tempdir().expect("config");
    let settings = config.path().join("settings.json");
    let original = serde_json::json!({
        "model": "opus",
        "hooks": {"PermissionRequest": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "mine.sh"}]}]}
    });
    std::fs::write(&settings, original.to_string()).expect("seed");

    integrate(config.path(), &[]);
    let backup: serde_json::Value = serde_json::from_slice(
        &std::fs::read(config.path().join("settings.json.gnomcode.bak")).expect("backup"),
    )
    .expect("backup json");
    assert_eq!(
        backup, original,
        "backup is the file as it was before the change"
    );
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).expect("read")).expect("json");
    assert_eq!(
        after["hooks"]["PermissionRequest"][0]["hooks"][0]["command"],
        "mine.sh"
    );
    assert_eq!(
        after["hooks"]["PermissionRequest"]
            .as_array()
            .expect("array")
            .len(),
        2
    );
    assert_eq!(
        after["hooks"]["Notification"][0]["matcher"],
        "permission_prompt"
    );
    assert_eq!(
        after["hooks"]["PreToolUse"]
            .as_array()
            .expect("array")
            .len(),
        1
    );

    integrate(config.path(), &["--remove"]);
    let removed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).expect("read")).expect("json");
    assert_eq!(removed["model"], "opus");
    assert_eq!(removed["hooks"], original["hooks"]);
    assert!(removed.get("statusLine").is_none());

    let text = integrate(config.path(), &["--no-approvals"]);
    assert!(text.contains("not installed"));
    let skipped: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).expect("read")).expect("json");
    assert!(skipped["hooks"].get("Notification").is_none());
    assert_eq!(
        skipped["hooks"]["PermissionRequest"]
            .as_array()
            .expect("array")
            .len(),
        1
    );
    assert_eq!(
        skipped["hooks"]["PreToolUse"]
            .as_array()
            .expect("array")
            .len(),
        1
    );
}
