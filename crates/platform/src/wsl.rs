//! WSL discovery on Windows (ADR 0060). Distros come from the `Lxss` registry key and
//! only running ones are read; nothing here ever boots a distro. Any failure fails open
//! to "no WSL roots".

use crate::AgentRoot;
use gnomcode_core::AgentId;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::time::{Duration, Instant};

/// How often the running-distro list is rechecked, so a distro started later is
/// picked up without calling `wsl.exe` on every ingest pass.
#[cfg(windows)]
const RECHECK: Duration = Duration::from_secs(60);

/// Parses `reg query HKCU\...\Lxss /s` output into distro names (registry order).
#[must_use]
pub fn parse_lxss_registry(output: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in output.lines() {
        let mut cols = line.split_whitespace();
        if cols.next() == Some("DistributionName") && cols.next() == Some("REG_SZ") {
            let name = cols.collect::<Vec<_>>().join(" ");
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// Decodes `wsl.exe -l --running -q` output (UTF-16LE, or UTF-8 on some builds).
#[must_use]
pub fn parse_running_list(bytes: &[u8]) -> Vec<String> {
    let text = if bytes.len() >= 2 && bytes.len().is_multiple_of(2) && bytes[1] == 0 {
        let units: Vec<u16> = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    };
    text.lines()
        .map(|l| l.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}' || c == '\0'))
        .filter(|l| !l.is_empty() && !l.contains(' '))
        .map(str::to_owned)
        .collect()
}

/// Registered distros that are also running. A stopped distro is skipped so its share is
/// never touched (reading it would boot it).
#[must_use]
pub fn running_registered(registered: &[String], running: &[String]) -> Vec<String> {
    registered
        .iter()
        .filter(|d| running.iter().any(|r| r.eq_ignore_ascii_case(d)))
        .cloned()
        .collect()
}

/// `\\wsl.localhost\<distro>`.
#[must_use]
pub fn distro_share(distro: &str) -> PathBuf {
    PathBuf::from(format!(r"\\wsl.localhost\{distro}"))
}

/// Agent log roots under one Linux home directory (same layout as the Windows ones).
#[must_use]
pub fn home_roots(home: &Path) -> Vec<(AgentId, PathBuf)> {
    let gemini = home.join(".gemini");
    vec![
        (AgentId::ClaudeCode, home.join(".claude").join("projects")),
        (AgentId::Codex, home.join(".codex").join("sessions")),
        (
            AgentId::Codex,
            home.join(".codex").join("archived_sessions"),
        ),
        (
            AgentId::Antigravity,
            gemini.join("antigravity").join("brain"),
        ),
        (
            AgentId::Antigravity,
            gemini
                .join(obfstr::obfstr!("antigravity-ide"))
                .join("brain"),
        ),
        (
            AgentId::Antigravity,
            gemini
                .join(obfstr::obfstr!("antigravity-cli"))
                .join("brain"),
        ),
    ]
}

/// Linux home folders inside a running distro's share: `/root` and each `/home/<user>`.
#[must_use]
pub fn distro_homes(share: &Path) -> Vec<PathBuf> {
    let mut homes = vec![share.join("root")];
    homes.extend(
        std::fs::read_dir(share.join("home"))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir()),
    );
    homes
}

/// Drops roots already listed under another spelling (`wsl$` vs `wsl.localhost`, case).
#[must_use]
pub fn dedupe_roots(roots: Vec<AgentRoot>) -> Vec<AgentRoot> {
    let mut seen: Vec<String> = Vec::new();
    roots
        .into_iter()
        .filter(|r| {
            let key = root_key(&r.path);
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        })
        .collect()
}

fn root_key(path: &Path) -> String {
    let s = path.to_string_lossy();
    match gnomcode_core::wsl::unc_to_linux(&s) {
        Some((d, l)) => format!("wsl:{}:{l}", d.to_lowercase()),
        None => s.into_owned(),
    }
}

/// Whether a path lives on a WSL share. 9P change events are unreliable there, so such
/// roots are polled instead of watched.
#[must_use]
pub fn is_wsl_path(path: &Path) -> bool {
    gnomcode_core::wsl::distro_of(&path.to_string_lossy()).is_some()
}

/// Agent roots of every running distro, cached for 60 s. Empty off Windows, when WSL is
/// absent, when `GNOMCODE_DISABLE_WSL` is set, or when
/// `CLAUDE_CONFIG_DIR`, `CODEX_HOME` and `GEMINI_HOME` are all set.
#[must_use]
pub fn discover_wsl_roots() -> Vec<AgentRoot> {
    #[cfg(windows)]
    {
        static CACHE: Mutex<Option<(Instant, Vec<AgentRoot>)>> = Mutex::new(None);
        // Pinning every agent home (as hermetic tests do) means "only these roots".
        let pinned = ["CLAUDE_CONFIG_DIR", "CODEX_HOME", "GEMINI_HOME"]
            .iter()
            .all(|k| std::env::var_os(k).is_some());
        if pinned || std::env::var_os("GNOMCODE_DISABLE_WSL").is_some() {
            return Vec::new();
        }
        let Ok(mut cache) = CACHE.lock() else {
            return Vec::new();
        };
        if let Some((at, roots)) = cache.as_ref()
            && at.elapsed() < RECHECK
        {
            return roots.clone();
        }
        let roots = scan_windows();
        *cache = Some((Instant::now(), roots.clone()));
        roots
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// Running distros now (uncached). Empty when WSL is absent or nothing runs.
#[must_use]
pub fn running_distros() -> Vec<String> {
    #[cfg(windows)]
    {
        if std::env::var_os("GNOMCODE_DISABLE_WSL").is_some() {
            return Vec::new();
        }
        let registered = registry_distros();
        if registered.is_empty() {
            return Vec::new();
        }
        running_registered(&registered, &wsl_running())
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// Every registered distro, running or not (registry only, never boots anything).
#[must_use]
pub fn registered_distros() -> Vec<String> {
    #[cfg(windows)]
    {
        if std::env::var_os("GNOMCODE_DISABLE_WSL").is_some() {
            return Vec::new();
        }
        registry_distros()
    }
    #[cfg(not(windows))]
    Vec::new()
}

#[cfg(windows)]
fn hidden(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    let mut c = std::process::Command::new(program);
    c.creation_flags(0x0800_0000);
    c
}

#[cfg(windows)]
fn registry_distros() -> Vec<String> {
    hidden("reg.exe")
        .args([
            "query",
            obfstr::obfstr!(r"HKCU\Software\Microsoft\Windows\CurrentVersion\Lxss"),
            "/s",
        ])
        .output()
        .ok()
        .map(|o| parse_lxss_registry(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

/// `wsl.exe -l --running` lists without starting any distro.
#[cfg(windows)]
fn wsl_running() -> Vec<String> {
    hidden("wsl.exe")
        .args(["-l", "--running", "-q"])
        .output()
        .ok()
        .map(|o| parse_running_list(&o.stdout))
        .unwrap_or_default()
}

#[cfg(windows)]
fn scan_windows() -> Vec<AgentRoot> {
    let mut roots = Vec::new();
    for distro in running_distros() {
        for home in distro_homes(&distro_share(&distro)) {
            for (agent, path) in home_roots(&home) {
                roots.push(AgentRoot {
                    agent,
                    exists: path.is_dir(),
                    path,
                });
            }
        }
    }
    dedupe_roots(roots)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REG: &str = "\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\r\n    DefaultDistribution    REG_SZ    {8a}\r\n\r\nHKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{8a}\r\n    State    REG_DWORD    0x1\r\n    DistributionName    REG_SZ    Ubuntu-22.04\r\n    BasePath    REG_SZ    C:\\x\r\n\r\nHKEY_CURRENT_USER\\Lxss\\{9b}\r\n    DistributionName    REG_SZ    Debian\r\n";

    #[test]
    fn parses_registry_distros() {
        assert_eq!(parse_lxss_registry(REG), vec!["Ubuntu-22.04", "Debian"]);
        assert!(parse_lxss_registry("").is_empty());
    }

    #[test]
    fn parses_utf16_and_utf8_running_lists() {
        let utf16: Vec<u8> = "Ubuntu-22.04\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(parse_running_list(&utf16), vec!["Ubuntu-22.04"]);
        assert_eq!(parse_running_list(b"Debian\n"), vec!["Debian"]);
        // The "nothing running" message is a sentence, not a distro.
        assert!(parse_running_list(b"There are no running distributions.\n").is_empty());
    }

    #[test]
    fn stopped_distro_is_skipped() {
        let reg = vec!["Ubuntu-22.04".to_owned(), "Debian".to_owned()];
        assert_eq!(
            running_registered(&reg, &["ubuntu-22.04".to_owned()]),
            vec!["Ubuntu-22.04"]
        );
        assert!(running_registered(&reg, &[]).is_empty());
    }

    #[test]
    fn builds_home_roots_under_share() {
        let home = distro_share("Ubuntu").join("home").join("a");
        let roots = home_roots(&home);
        assert_eq!(roots.len(), 6);
        assert!(roots[0].1.ends_with(Path::new(".claude").join("projects")));
        assert!(roots.iter().all(|(_, p)| is_wsl_path(p)));
        assert!(!is_wsl_path(Path::new("relative/.claude")));
    }

    #[test]
    fn dedupes_two_spellings() {
        let r = |p: &str| AgentRoot {
            agent: AgentId::ClaudeCode,
            path: PathBuf::from(p),
            exists: true,
        };
        let out = dedupe_roots(vec![
            r(r"\\wsl.localhost\Ubuntu\home\a\.claude\projects"),
            r(r"\\wsl$\ubuntu\home\a\.claude\projects"),
            r("other"),
        ]);
        assert_eq!(out.len(), 2);
    }

    #[cfg(windows)]
    #[test]
    #[ignore = "reads this machine's WSL state; run manually"]
    fn live_discovery_is_read_only() {
        let before = running_distros();
        let roots = discover_wsl_roots();
        assert_eq!(
            running_distros(),
            before,
            "discovery must not start a distro"
        );
        eprintln!(
            "registered={:?} running={before:?} existing_roots={:?}",
            registered_distros(),
            roots
                .iter()
                .filter(|r| r.exists)
                .map(|r| &r.path)
                .collect::<Vec<_>>()
        );
    }
}
