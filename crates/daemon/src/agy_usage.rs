//! Verified, bounded native usage fallback; never an inference request.

use gnomcode_core::QuotaSnapshot;
use std::{process::Command, time::Duration};

/// Keeps the running-server source authoritative, with the verified CLI as fallback.
pub(crate) async fn fallback(
    current: Result<Vec<QuotaSnapshot>, String>,
) -> Result<Vec<QuotaSnapshot>, String> {
    fallback_with(
        current,
        || gnomcode_platform::executables::background_agent_command("agy"),
        Duration::from_secs(8),
    )
    .await
}

async fn output(
    mut command: Command,
    args: &[&str],
    cwd: &std::path::Path,
) -> Result<Vec<u8>, &'static str> {
    use tokio::io::AsyncReadExt as _;
    command
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    let mut command = tokio::process::Command::from(command);
    command.kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|_| "Antigravity CLI is unavailable")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Antigravity CLI output is unavailable")?;
    let mut bytes = Vec::new();
    stdout
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| "Cannot read Antigravity usage")?;
    if bytes.len() > 64 * 1024 {
        return Err("Antigravity usage output is too large");
    }
    if !child
        .wait()
        .await
        .map_err(|_| "Antigravity usage did not finish")?
        .success()
    {
        return Err("Antigravity usage command failed");
    }
    Ok(bytes)
}

async fn fallback_with(
    current: Result<Vec<QuotaSnapshot>, String>,
    command: impl Fn() -> Command,
    deadline: Duration,
) -> Result<Vec<QuotaSnapshot>, String> {
    if current.as_ref().is_ok_and(|rows| !rows.is_empty()) {
        return current;
    }
    tokio::time::timeout(deadline, async {
        let cwd = tempfile::tempdir()
            .map_err(|_| "Cannot create isolated Antigravity probe workspace")?;
        let version = output(command(), &["--version"], cwd.path()).await?;
        // Other versions must be verified before a slash command can be treated as no-inference metadata.
        if std::str::from_utf8(&version).ok().map(str::trim) != Some("1.5.7") {
            return Err("Antigravity usage fallback is verified only for agy 1.5.7");
        }
        let bytes = output(
            command(),
            &["--output-format", "text", "-p=/usage"],
            cwd.path(),
        )
        .await?;
        let text =
            std::str::from_utf8(&bytes).map_err(|_| "Antigravity usage is not valid UTF-8")?;
        let rows = gnomcode_adapters::antigravity::parse_cli_usage(text, chrono::Utc::now());
        if rows.is_empty() {
            return Err("Antigravity returned no recognized usage windows");
        }
        Ok(rows)
    })
    .await
    .map_err(|_| "Antigravity usage probe timed out".to_owned())?
    .map_err(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn usage_probe_prefers_server_and_is_bounded_and_version_gated() {
        let dir = tempfile::tempdir().expect("test directory");
        let exe = dir
            .path()
            .join(format!("fake-agy{}", std::env::consts::EXE_SUFFIX));
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crates")
            .parent()
            .expect("workspace")
            .join("fixtures")
            .join("antigravity")
            .join("cli-1.5.7")
            .join("fake_usage_cli.rs");
        let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&exe)
            .output()
            .expect("compile fixture");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let marker = dir.path().join("calls.txt");
        let command = |mode: &str, version: &str| {
            let mut command = Command::new(&exe);
            command
                .env("GNOMCODE_FAKE_USAGE_MODE", mode)
                .env("GNOMCODE_FAKE_USAGE_VERSION", version)
                .env("GNOMCODE_FAKE_USAGE_MARKER", &marker);
            command
        };
        let existing = QuotaSnapshot {
            provider: "antigravity".into(),
            limit_id: "existing".into(),
            window: gnomcode_core::QuotaWindow::Weekly,
            used_pct: 20.0,
            resets_at: None,
            plan: None,
            basis: gnomcode_core::TrustLevel::Api,
            observed_at: chrono::Utc::now(),
        };
        assert_eq!(
            fallback_with(
                Ok(vec![existing.clone()]),
                || command("", "1.5.7"),
                Duration::from_secs(8)
            )
            .await
            .expect("server reading"),
            vec![existing]
        );
        assert!(
            !marker.exists(),
            "good server reading must not start any CLI"
        );
        let rows = fallback_with(
            Err("no server".into()),
            || command("", "1.5.7"),
            Duration::from_secs(8),
        )
        .await
        .expect("native usage");
        assert_eq!(rows.len(), 5);
        let calls = std::fs::read_to_string(&marker).expect("calls");
        assert!(
            calls
                .lines()
                .next()
                .expect("version call")
                .ends_with("|--version")
        );
        assert!(
            calls
                .lines()
                .nth(1)
                .expect("usage call")
                .ends_with("|--output-format|text|-p=/usage")
        );
        std::fs::remove_file(&marker).expect("reset marker");
        assert!(
            fallback_with(
                Err("no server".into()),
                || command("", "9.9.9"),
                Duration::from_secs(8)
            )
            .await
            .is_err()
        );
        let calls = std::fs::read_to_string(&marker).expect("version only");
        assert_eq!(
            calls.lines().count(),
            1,
            "unknown versions must never receive /usage"
        );
        for mode in ["fail", "malformed", "large"] {
            assert!(
                fallback_with(
                    Err("no server".into()),
                    || command(mode, "1.5.7"),
                    Duration::from_secs(8)
                )
                .await
                .is_err(),
                "{mode}"
            );
        }
        std::fs::remove_file(&marker).expect("reset timeout marker");
        let started = std::time::Instant::now();
        assert!(
            fallback_with(
                Err("no server".into()),
                || command("hang", "1.5.7"),
                Duration::from_secs(3)
            )
            .await
            .is_err()
        );
        assert!(started.elapsed() < Duration::from_secs(6));
        let calls = std::fs::read_to_string(&marker).expect("hung CLI calls");
        let last = calls.lines().last().expect("usage process");
        assert!(last.ends_with("|--output-format|text|-p=/usage"));
        let pid: u32 = last
            .split('|')
            .next()
            .expect("pid")
            .parse()
            .expect("process ID");
        for _ in 0..20 {
            if !gnomcode_platform::process_is_alive(pid) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(
            !gnomcode_platform::process_is_alive(pid),
            "timed-out owned probe must exit"
        );
    }
}
