//! Launch preflight for workspaces inside a WSL distro (ADR 0060): the agent must be
//! installed there, and an explicit launch may start a stopped distro.

use serde_json::Value;
use std::path::Path;

/// Why a launch is refused before anything starts.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// The folder does not exist (Windows) or is not a usable WSL folder.
    BadFolder,
    /// The agent is not installed in the distro; carries the install hint.
    Missing(String),
}

/// Returns `Some("Starting <distro>...")` when this launch boots a stopped distro.
pub(crate) async fn preflight(agent: &str, repo: &Path) -> Result<Option<String>, Refusal> {
    let Some((distro, _)) = gnomcode_pty::wsl::target(repo) else {
        return if repo.is_dir() {
            Ok(None)
        } else {
            Err(Refusal::BadFolder)
        };
    };
    let Ok(inner) = gnomcode_pty::launch_command(agent, repo, None) else {
        return Err(Refusal::BadFolder);
    };
    let probe =
        gnomcode_pty::wsl::probe(&distro, &inner.executable).map_err(|_| Refusal::BadFolder)?;
    let exe = inner.executable.clone();
    tokio::task::spawn_blocking(move || {
        let running = gnomcode_platform::wsl::running_distros()
            .iter()
            .any(|d| d.eq_ignore_ascii_case(&distro));
        // Explicit launch: this call may start the distro (allowed by ADR 0060).
        let output = run_hidden(&probe.executable, &probe.arguments);
        if !output.is_some_and(|out| gnomcode_pty::wsl::probe_found(&out)) {
            return Err(Refusal::Missing(gnomcode_pty::wsl::install_hint(
                &exe, &distro,
            )));
        }
        Ok((!running).then(|| gnomcode_pty::wsl::starting_message(&distro)))
    })
    .await
    .map_err(|_| Refusal::BadFolder)?
}

fn run_hidden(program: &str, args: &[String]) -> Option<String> {
    let mut command = std::process::Command::new(program);
    command.args(args).stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    let out = command.output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Adds `wsl_distro` (and `starting` while a distro boots) to a launch reply.
pub(crate) fn annotate(body: &mut Value, repo: &Path, starting: Option<String>) {
    let Some(fields) = body.as_object_mut() else {
        return;
    };
    if let Some((distro, _)) = gnomcode_pty::wsl::target(repo) {
        fields.insert(
            "wsl_distro".into(),
            gnomcode_core::wsl::distro_label(&distro).into(),
        );
    }
    if let Some(message) = starting {
        fields.insert("starting".into(), message.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn annotates_wsl_replies_only() {
        let mut body = json!({"session_id":"s"});
        annotate(
            &mut body,
            Path::new(r"\\wsl.localhost\Ubuntu\home\a"),
            Some("Starting Ubuntu...".into()),
        );
        assert_eq!(body["wsl_distro"], "WSL: Ubuntu");
        assert_eq!(body["starting"], "Starting Ubuntu...");
        let mut plain = json!({"session_id":"s"});
        annotate(&mut plain, Path::new("relative"), None);
        assert!(plain.get("wsl_distro").is_none());
    }

    #[tokio::test]
    async fn windows_folder_must_exist() {
        let dir = tempfile::tempdir().expect("dir");
        assert_eq!(preflight("codex", dir.path()).await, Ok(None));
        assert_eq!(
            preflight("codex", &dir.path().join("missing")).await,
            Err(Refusal::BadFolder)
        );
    }
}
