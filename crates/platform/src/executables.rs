//! Platform-specific shell selection and PATH discovery without starting processes.

use std::path::Path;

/// Resolves an installed official CLI executable path and leading arguments, if available.
#[must_use]
pub fn resolve_agent_executable(name: &str) -> Option<(String, Vec<String>)> {
    if !executable_available(name) {
        return None;
    }
    let (program, args) = agent_command(name);
    let candidate = Path::new(&program);
    if candidate.is_file() {
        return Some((program, args));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&paths) {
            if let Some(found) = find_executable_in(&directory, name) {
                return Some((found.to_string_lossy().into_owned(), args));
            }
        }
    }
    Some((program, args))
}

/// Parent-session environment variables removed when launching Claude Code.
pub const CLAUDE_PARENT_ENV_VARS: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
];

/// Builds a hidden, shell-free command for a known official coding-agent CLI.
#[must_use]
pub fn background_agent_command(name: &str) -> std::process::Command {
    let (program, args) = agent_command(name);
    let mut command = std::process::Command::new(program);
    command.args(args);
    if matches!(name, "claude" | "claude-code") {
        for key in CLAUDE_PARENT_ENV_VARS {
            command.env_remove(key);
        }
    }
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
    if !on_path(name) {
        let installed = match name {
            "agy" => antigravity_cli(),
            "claude" => claude_cli(),
            "codex" => standard_cli(name),
            _ => None,
        };
        if let Some(binary) = installed {
            #[cfg(windows)]
            if let (Some(dir), Some(package)) = (binary.parent(), npm_package_name(name))
                && let Some(resolved) = resolve_windows_npm_or_binary(dir, package, name)
            {
                return resolved;
            }
            return (binary.to_string_lossy().into_owned(), Vec::new());
        }
    }
    #[cfg(windows)]
    if let (Some(paths), Some(package)) = (std::env::var_os("PATH"), npm_package_name(name)) {
        for prefix in std::env::split_paths(&paths) {
            if let Some(resolved) = resolve_windows_npm_or_binary(&prefix, package, name) {
                return resolved;
            }
        }
    }
    (name.to_owned(), Vec::new())
}

#[cfg(windows)]
fn npm_package_name(name: &str) -> Option<&'static str> {
    match name {
        "claude" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@openai/codex"),
        _ => None,
    }
}

#[cfg(windows)]
fn resolve_windows_npm_or_binary(
    prefix: &Path,
    package: &str,
    name: &str,
) -> Option<(String, Vec<String>)> {
    let native = prefix.join(format!("{name}.exe"));
    if native.is_file() {
        return Some((native.to_string_lossy().into_owned(), Vec::new()));
    }
    if prefix.join(format!("{name}.cmd")).is_file()
        && let Some(entry) = npm_entry_point(prefix, package, name)
    {
        let node = prefix.join("node.exe");
        return Some((
            if node.is_file() {
                node.to_string_lossy().into_owned()
            } else {
                "node.exe".into()
            },
            vec![entry.to_string_lossy().into_owned()],
        ));
    }
    None
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

/// Checks PATH, official CLI installation directories, and Antigravity IDE.
#[must_use]
pub fn executable_available(name: &str) -> bool {
    on_path(name)
        || (name == "agy" && antigravity_cli().is_some())
        || (name == "claude" && claude_cli().is_some())
        || (name == "codex" && standard_cli(name).is_some())
        || (name == "antigravity" && (antigravity_cli().is_some() || antigravity_ide_available()))
}

/// Checks whether the Google Antigravity IDE or 2.0 app is installed on this system.
#[must_use]
pub fn antigravity_ide_available() -> bool {
    #[cfg(windows)]
    {
        if let Some(local_appdata) = std::env::var_os("LOCALAPPDATA") {
            let programs = std::path::PathBuf::from(local_appdata).join("Programs");
            // The 2.0 app installs as `Antigravity`, the IDE 1.x as `Antigravity IDE`.
            if ["Antigravity", "Antigravity IDE"]
                .iter()
                .any(|name| programs.join(name).join(format!("{name}.exe")).is_file())
            {
                return true;
            }
        }
        if let Some(prog_files) = std::env::var_os("ProgramFiles") {
            let path = std::path::PathBuf::from(prog_files)
                .join("Antigravity")
                .join("Antigravity.exe");
            if path.is_file() {
                return true;
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if std::path::Path::new("/Applications/Antigravity.app").exists() {
            return true;
        }
        if let Some(home) = std::env::var_os("HOME")
            && std::path::PathBuf::from(home)
                .join("Applications")
                .join("Antigravity.app")
                .exists()
        {
            return true;
        }
    }
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/usr/bin/antigravity").exists()
            || std::path::Path::new("/opt/Antigravity/antigravity").exists()
        {
            return true;
        }
    }
    let home = crate::user_home_dir();
    home.join(".gemini")
        .join("antigravity")
        .join("brain")
        .is_dir()
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| executable_in(&directory, name))
    })
}

fn standard_cli(name: &str) -> Option<std::path::PathBuf> {
    let home = crate::user_home_dir();
    let mut dirs = vec![
        home.join(".local").join("bin"),
        home.join(".npm-global").join("bin"),
    ];
    #[cfg(windows)]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            dirs.push(std::path::PathBuf::from(appdata).join("npm"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let local = std::path::PathBuf::from(local);
            dirs.push(local.join("Microsoft").join("WinGet").join("Links"));
            dirs.push(local.join("Programs").join(name));
        }
        dirs.push(home.join("scoop").join("shims"));
    }
    #[cfg(target_os = "macos")]
    {
        dirs.push(std::path::PathBuf::from("/opt/homebrew/bin"));
        dirs.push(std::path::PathBuf::from("/usr/local/bin"));
    }
    #[cfg(target_os = "linux")]
    {
        dirs.push(std::path::PathBuf::from("/usr/local/bin"));
    }
    dirs.into_iter()
        .find_map(|directory| find_executable_in(&directory, name))
}

fn antigravity_cli() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    let binary = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
        .join("agy")
        .join("bin")
        .join("agy.exe");
    #[cfg(not(windows))]
    let binary = std::path::PathBuf::from(std::env::var_os("HOME")?)
        .join(".local")
        .join("bin")
        .join("agy");
    let directory = binary.parent()?;
    (binary.is_file() && executable_in(directory, "agy")).then_some(binary)
}

/// Finds Claude Code outside PATH: the native installer, then the copy bundled with the Claude desktop app.
fn claude_cli() -> Option<std::path::PathBuf> {
    let file = if cfg!(windows) {
        "claude.exe"
    } else {
        "claude"
    };
    let native = crate::user_home_dir().join(".local").join("bin").join(file);
    if native
        .parent()
        .is_some_and(|dir| executable_in(dir, "claude"))
    {
        return Some(native);
    }
    // Windows desktop app layout: Claude-Data/<install>/claude-code/<version>/<build>/claude.exe.
    // The packaged (MSIX) app keeps it under Packages/Claude_*/LocalCache/Local when seen from outside.
    let local = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let children = |dir: &Path| {
        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>()
    };
    let packaged = children(&local.join("Packages"))
        .into_iter()
        .filter(|package| {
            package
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("Claude_"))
        })
        .map(|package| package.join("LocalCache").join("Local").join("Claude-Data"));
    std::iter::once(local.join("Claude-Data"))
        .chain(packaged)
        .flat_map(|root| children(&root))
        .flat_map(|install| children(&install.join("claude-code")))
        .flat_map(|version| children(&version))
        .map(|build| build.join(file))
        .filter(|binary| binary.is_file())
        .max_by_key(|binary| binary.metadata().and_then(|m| m.modified()).ok())
}

fn find_executable_in(directory: &Path, name: &str) -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        ["exe", "cmd", "bat", ""].iter().find_map(|extension| {
            let candidate = directory.join(if extension.is_empty() {
                name.to_owned()
            } else {
                format!("{name}.{extension}")
            });
            candidate.is_file().then_some(candidate)
        })
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let candidate = directory.join(name);
        candidate
            .metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .then_some(candidate)
    }
}

fn executable_in(directory: &Path, name: &str) -> bool {
    find_executable_in(directory, name).is_some()
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
    #[test]
    fn background_metadata_commands_remove_only_proven_parent_context() {
        let command = super::background_agent_command("claude");
        let removed: Vec<_> = command
            .get_envs()
            .filter_map(|(key, value)| {
                value
                    .is_none()
                    .then_some(key.to_string_lossy().into_owned())
            })
            .collect();
        assert_eq!(
            removed,
            [
                "CLAUDECODE",
                "CLAUDE_CODE_CHILD_SESSION",
                "CLAUDE_CODE_SESSION_ID"
            ]
        );
    }
    use super::*;

    #[test]
    fn agy_standard_install_works_without_path_entry() {
        const CHILD: &str = "GNOMCODE_TEST_AGY_INSTALL";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = std::path::PathBuf::from(root);
            let expected = if cfg!(windows) {
                root.join("agy").join("bin").join("agy.exe")
            } else {
                root.join(".local").join("bin").join("agy")
            };
            assert!(executable_available("agy"));
            assert_eq!(
                agent_command("agy"),
                (expected.to_string_lossy().into_owned(), vec![])
            );
            assert!(!executable_available("missing-gnomcode-test-executable"));
            return;
        }
        let root = tempfile::tempdir().expect("install root");
        let directory = if cfg!(windows) {
            root.path().join("agy").join("bin")
        } else {
            root.path().join(".local").join("bin")
        };
        std::fs::create_dir_all(&directory).expect("bin directory");
        let binary = directory.join(if cfg!(windows) { "agy.exe" } else { "agy" });
        std::fs::write(&binary, "fixture").expect("binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700))
                .expect("executable");
        }
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "executables::tests::agy_standard_install_works_without_path_entry",
            ])
            .env(CHILD, root.path())
            .env("LOCALAPPDATA", root.path())
            .env("HOME", root.path())
            .env("PATH", root.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }

    #[test]
    fn claude_bundled_with_desktop_app_is_found_without_path_entry() {
        const CHILD: &str = "GNOMCODE_TEST_CLAUDE_BUNDLE";
        if let Some(root) = std::env::var_os(CHILD) {
            let expected = std::path::PathBuf::from(root)
                .join("Claude-Data")
                .join("claude_x")
                .join("claude-code")
                .join("2.1.321")
                .join("build")
                .join("claude.exe");
            assert!(executable_available("claude"));
            assert_eq!(
                agent_command("claude"),
                (expected.to_string_lossy().into_owned(), vec![])
            );
            return;
        }
        if !cfg!(windows) {
            return;
        }
        let root = tempfile::tempdir().expect("root");
        let build = root
            .path()
            .join("Claude-Data")
            .join("claude_x")
            .join("claude-code")
            .join("2.1.321")
            .join("build");
        std::fs::create_dir_all(&build).expect("build dir");
        std::fs::write(build.join("claude.exe"), "fixture").expect("binary");
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "executables::tests::claude_bundled_with_desktop_app_is_found_without_path_entry",
            ])
            .env(CHILD, root.path())
            .env("LOCALAPPDATA", root.path())
            .env("USERPROFILE", root.path())
            .env("HOME", root.path())
            .env("PATH", root.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }

    #[test]
    fn claude_bundled_with_packaged_desktop_app_is_found() {
        const CHILD: &str = "GNOMCODE_TEST_CLAUDE_PACKAGE";
        let build = |root: &Path| {
            root.join("Packages")
                .join("Claude_pzs8sxrjxfjjc")
                .join("LocalCache")
                .join("Local")
                .join("Claude-Data")
                .join("claude_x")
                .join("claude-code")
                .join("2.1.321")
                .join("build")
        };
        if let Some(root) = std::env::var_os(CHILD) {
            let expected = build(Path::new(&root)).join("claude.exe");
            assert_eq!(
                agent_command("claude"),
                (expected.to_string_lossy().into_owned(), vec![])
            );
            return;
        }
        if !cfg!(windows) {
            return;
        }
        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir_all(build(root.path())).expect("build dir");
        std::fs::write(build(root.path()).join("claude.exe"), "fixture").expect("binary");
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "executables::tests::claude_bundled_with_packaged_desktop_app_is_found",
            ])
            .env(CHILD, root.path())
            .env("LOCALAPPDATA", root.path())
            .env("USERPROFILE", root.path())
            .env("HOME", root.path())
            .env("PATH", root.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }

    #[test]
    fn path_discovery_requires_an_existing_executable() {
        let current = std::env::current_exe().expect("test executable");
        assert!(executable_in(
            current.parent().expect("parent"),
            current.file_name().expect("name").to_str().expect("utf8")
        ));
        assert!(!executable_in(
            current.parent().expect("parent"),
            "missing-gnomcode-test-executable"
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

    #[test]
    fn resolve_agent_executable_returns_none_when_missing_and_path_when_present() {
        assert!(resolve_agent_executable("missing-gnomcode-test-executable").is_none());
        let current = std::env::current_exe().expect("test executable");
        let dir = current.parent().expect("parent");
        let stem = current.file_stem().expect("stem").to_string_lossy();
        const CHILD: &str = "GNOMCODE_TEST_RESOLVE_EXE";
        if let Some(target) = std::env::var_os(CHILD) {
            let target = target.to_string_lossy();
            let resolved = resolve_agent_executable(&target).expect("resolved");
            assert!(
                resolved.0.contains(target.as_ref()),
                "expected {} to contain {}",
                resolved.0,
                target
            );
            return;
        }
        let status = std::process::Command::new(&current)
            .args([
                "--exact",
                "executables::tests::resolve_agent_executable_returns_none_when_missing_and_path_when_present",
            ])
            .env(CHILD, stem.as_ref())
            .env("PATH", dir)
            .status()
            .expect("isolated test");
        assert!(status.success());
    }

    #[test]
    fn codex_standard_install_works_without_path_entry() {
        const CHILD: &str = "GNOMCODE_TEST_CODEX_INSTALL";
        if let Some(root) = std::env::var_os(CHILD) {
            let root = std::path::PathBuf::from(root);
            let expected = root.join(".local").join("bin").join(if cfg!(windows) {
                "codex.exe"
            } else {
                "codex"
            });
            assert!(executable_available("codex"));
            assert_eq!(
                agent_command("codex"),
                (expected.to_string_lossy().into_owned(), vec![])
            );
            assert_eq!(
                resolve_agent_executable("codex"),
                Some((expected.to_string_lossy().into_owned(), vec![]))
            );
            return;
        }
        let root = tempfile::tempdir().expect("install root");
        let empty_path = tempfile::tempdir().expect("empty path");
        let directory = root.path().join(".local").join("bin");
        std::fs::create_dir_all(&directory).expect("bin directory");
        let binary = directory.join(if cfg!(windows) { "codex.exe" } else { "codex" });
        std::fs::write(&binary, "fixture").expect("binary");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700))
                .expect("executable");
        }
        let status = std::process::Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "--exact",
                "executables::tests::codex_standard_install_works_without_path_entry",
            ])
            .env(CHILD, root.path())
            .env("HOME", root.path())
            .env("USERPROFILE", root.path())
            .env("LOCALAPPDATA", root.path())
            .env("APPDATA", root.path())
            .env("PATH", empty_path.path())
            .status()
            .expect("isolated test");
        assert!(status.success());
    }
}
