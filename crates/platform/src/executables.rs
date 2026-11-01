//! Platform-specific shell selection and PATH discovery without starting processes.

use std::path::Path;

/// Builds a hidden, shell-free command for a known official coding-agent CLI.
#[must_use]
pub fn background_agent_command(name: &str) -> std::process::Command {
    let (program, args) = agent_command(name);
    let mut command = std::process::Command::new(program);
    command.args(args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    command
}

/// Resolves official CLI executables, including Windows npm packages, without a shell.
#[must_use]
pub fn agent_command(name: &str) -> (String, Vec<String>) {
    #[cfg(windows)]
    if let Some(paths) = std::env::var_os("PATH") {
        let package = match name {
            "claude" => "@anthropic-ai/claude-code",
            "codex" => "@openai/codex",
            "gemini" => "@google/gemini-cli",
            _ => return (name.to_owned(), Vec::new()),
        };
        for prefix in std::env::split_paths(&paths) {
            let native = prefix.join(format!("{name}.exe"));
            if native.is_file() {
                return (native.to_string_lossy().into_owned(), Vec::new());
            }
            if prefix.join(format!("{name}.cmd")).is_file() {
                if let Some(entry) = npm_entry_point(&prefix, package, name) {
                    let node = prefix.join("node.exe");
                    return (
                        if node.is_file() {
                            node.to_string_lossy().into_owned()
                        } else {
                            "node.exe".into()
                        },
                        vec![entry.to_string_lossy().into_owned()],
                    );
                }
            }
        }
    }
    (name.to_owned(), Vec::new())
}

#[cfg(any(windows, test))]
fn npm_entry_point(prefix: &Path, package: &str, name: &str) -> Option<std::path::PathBuf> {
    let package = package
        .split('/')
        .fold(prefix.join("node_modules"), |path, component| {
            path.join(component)
        });
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(package.join("package.json")).ok()?).ok()?;
    let bin = manifest.get("bin")?;
    let relative = bin.as_str().or_else(|| bin.get(name)?.as_str())?;
    let entry = std::path::absolute(package.join(relative)).ok()?;
    let resolved = entry.canonicalize().ok()?;
    (resolved.is_file() && resolved.starts_with(package.canonicalize().ok()?)).then_some(entry)
}

/// Checks whether an executable exists on the current process PATH.
#[must_use]
pub fn executable_available(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| executable_in(&directory, name))
    })
}

fn executable_in(directory: &Path, name: &str) -> bool {
    #[cfg(windows)]
    {
        ["exe", "cmd", "bat", ""].iter().any(|extension| {
            directory
                .join(if extension.is_empty() {
                    name.to_owned()
                } else {
                    format!("{name}.{extension}")
                })
                .is_file()
        })
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        directory
            .join(name)
            .metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }
}

/// Returns the interactive local shell and its startup arguments.
#[must_use]
pub fn terminal_shell() -> (String, Vec<String>) {
    #[cfg(windows)]
    {
        ("powershell.exe".to_owned(), vec!["-NoLogo".to_owned()])
    }
    #[cfg(not(windows))]
    {
        (
            std::env::var("SHELL")
                .ok()
                .filter(|shell| !shell.is_empty())
                .unwrap_or_else(|| "/bin/sh".to_owned()),
            Vec::new(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_discovery_requires_an_existing_executable() {
        let current = std::env::current_exe().expect("test executable");
        assert!(executable_in(
            current.parent().expect("parent"),
            current.file_name().expect("name").to_str().expect("utf8")
        ));
        assert!(!executable_in(
            current.parent().expect("parent"),
            "missing-spawnt-test-executable"
        ));
    }

    #[test]
    fn npm_entry_point_uses_node_without_a_command_shell() {
        let prefix = tempfile::tempdir().expect("prefix");
        let package = prefix
            .path()
            .join("node_modules")
            .join("@example")
            .join("agent");
        std::fs::create_dir_all(package.join("bin")).expect("package");
        std::fs::write(
            package.join("package.json"),
            r#"{"bin":{"agent":"bin/agent.js"}}"#,
        )
        .expect("manifest");
        std::fs::write(package.join("bin").join("agent.js"), "// fixture").expect("entry");
        let entry = npm_entry_point(prefix.path(), "@example/agent", "agent").expect("entry point");
        assert_eq!(entry, package.join("bin").join("agent.js"));
        std::fs::write(package.join("package.json"), "broken JSON").expect("manifest");
        assert!(npm_entry_point(prefix.path(), "@example/agent", "agent").is_none());
    }
}
