//! WSL launch and resume commands (ADR 0060). A workspace spelled
//! `\\wsl.localhost\<distro>\...` runs as `wsl.exe -d <distro> --cd <linux path> --exec <agent>`.
//! Everything here is pure; skip-permission flags are never added.

use crate::{Error, LaunchCommand};
use std::path::Path;

/// `(distro, linux folder)` when the workspace lives in a WSL distro.
#[must_use]
pub fn target(repo: &Path) -> Option<(String, String)> {
    gnomcode_core::wsl::unc_to_linux(&repo.to_string_lossy())
}

fn valid_distro(distro: &str) -> bool {
    !distro.is_empty()
        && !distro.starts_with('-')
        && distro
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

/// Wraps an agent command so it runs inside `distro` at `linux_cwd`.
///
/// # Errors
/// Returns [`Error::WorkspaceNotFound`] for an unsafe distro name or folder.
pub fn wrap(distro: &str, linux_cwd: &str, inner: &LaunchCommand) -> Result<LaunchCommand, Error> {
    if !valid_distro(distro) || !linux_cwd.starts_with('/') {
        return Err(Error::WorkspaceNotFound(format!("{distro}:{linux_cwd}")));
    }
    let mut arguments = vec![
        "-d".to_owned(),
        distro.to_owned(),
        "--cd".to_owned(),
        linux_cwd.to_owned(),
        "--exec".to_owned(),
        inner.executable.clone(),
    ];
    arguments.extend(inner.arguments.iter().cloned());
    Ok(LaunchCommand {
        executable: "wsl.exe".to_owned(),
        arguments,
    })
}

/// `wsl.exe -d <distro> --exec sh -c 'command -v "$1"' sh <agent>`: the same non-login
/// environment the launch uses.
///
/// # Errors
/// Returns [`Error::WorkspaceNotFound`] for an unsafe distro name.
pub fn probe(distro: &str, executable: &str) -> Result<LaunchCommand, Error> {
    if !valid_distro(distro) {
        return Err(Error::WorkspaceNotFound(distro.to_owned()));
    }
    Ok(LaunchCommand {
        executable: "wsl.exe".to_owned(),
        arguments: [
            "-d",
            distro,
            "--exec",
            "sh",
            "-c",
            "command -v \"$1\"",
            "sh",
            executable,
        ]
        .map(str::to_owned)
        .to_vec(),
    })
}

/// Whether `command -v` output names a Linux install. A `/mnt/<drive>/` hit is the
/// Windows copy reached through interop, which is not an install in the distro.
#[must_use]
pub fn probe_found(stdout: &str) -> bool {
    stdout
        .lines()
        .next()
        .map(str::trim)
        .is_some_and(|p| p.starts_with('/') && !p.starts_with("/mnt/"))
}

/// Plain install hint for an agent missing from a distro.
#[must_use]
pub fn install_hint(executable: &str, distro: &str) -> String {
    let how = match executable {
        "claude" => "curl -fsSL https://claude.ai/install.sh | bash",
        "codex" => "npm install -g @openai/codex",
        "agy" => "install the Antigravity CLI for Linux",
        _ => "install it",
    };
    format!("{executable} is not installed in {distro}. In a {distro} terminal run: {how}")
}

/// Status line shown while an explicit launch boots a stopped distro.
#[must_use]
pub fn starting_message(distro: &str) -> String {
    format!("Starting {distro}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn agent(exe: &str, args: &[&str]) -> LaunchCommand {
        LaunchCommand {
            executable: exe.to_owned(),
            arguments: args.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    #[test]
    fn wraps_launch_and_resume() {
        let repo = PathBuf::from(r"\\wsl.localhost\Ubuntu-22.04\home\a\proj");
        let (distro, cwd) = target(&repo).expect("wsl");
        let cmd = wrap(&distro, &cwd, &agent("claude", &["--resume", "abc"])).expect("ok");
        assert_eq!(cmd.executable, "wsl.exe");
        assert_eq!(
            cmd.arguments,
            [
                "-d",
                "Ubuntu-22.04",
                "--cd",
                "/home/a/proj",
                "--exec",
                "claude",
                "--resume",
                "abc"
            ]
        );
        let all = cmd.arguments.join(" ");
        assert!(!all.contains("dangerously") && !all.contains("yolo"));
        assert!(target(Path::new("relative")).is_none());
    }

    #[test]
    fn rejects_unsafe_distro_names() {
        let a = agent("codex", &[]);
        assert!(wrap("-d", "/x", &a).is_err());
        assert!(wrap("Ub untu", "/x", &a).is_err());
        assert!(wrap("Ubuntu", "relative", &a).is_err());
        assert!(probe("--exec", "codex").is_err());
    }

    #[test]
    fn probe_and_hint() {
        let p = probe("Ubuntu", "agy").expect("ok");
        assert_eq!(p.arguments.last().map(String::as_str), Some("agy"));
        assert!(probe_found("/usr/bin/claude\n"));
        assert!(probe_found("/home/a/.local/bin/codex"));
        assert!(!probe_found(""));
        assert!(!probe_found("/mnt/c/Users/a/AppData/Roaming/npm/codex"));
        assert!(install_hint("codex", "Ubuntu").contains("not installed in Ubuntu"));
        assert_eq!(starting_message("Ubuntu"), "Starting Ubuntu...");
    }
}
