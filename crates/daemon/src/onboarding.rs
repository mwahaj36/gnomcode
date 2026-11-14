//! Per-provider setup status for onboarding, Settings > Providers and the doctor (ADR 0061).
//!
//! Detection only: files are checked for existence or non-secret fields (the Claude account
//! email), never token values. Login commands are shown, never run.

use crate::doctor::AgentCliStatus;
use serde::Serialize;
use std::path::PathBuf;

/// Providers onboarding can set up, in display order.
pub const PROVIDERS: [&str; 3] = ["claude", "codex", "antigravity"];

/// How far one step has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    /// Verified complete.
    Done,
    /// Needs the user to run `command`.
    Todo,
    /// Configured; waiting for evidence (a first reading or session).
    Waiting,
    /// Cannot be checked right now (for example a stopped WSL distro).
    Unknown,
}

/// Certainty of a step's verdict (SPEC invariant 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Trust {
    /// Read directly (process exit, config entry, stored row).
    Exact,
    /// Inferred from indirect evidence such as a file's presence.
    Estimated,
    /// Not determined.
    Unknown,
}

/// One checklist step.
#[derive(Debug, Clone, Serialize)]
pub struct Step {
    /// `installed`, `signed_in`, `integrated`, `protocol_approvals`, `limits`, `first_data`.
    pub id: &'static str,
    /// Short title.
    pub label: &'static str,
    /// Current state.
    pub state: StepState,
    /// Optional steps never count against completeness.
    pub optional: bool,
    /// Certainty of `state`.
    pub trust: Trust,
    /// One-line observation or explanation.
    pub note: String,
    /// Command the user runs in their own terminal to finish the step.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Status for one provider, optionally inside a WSL distro.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderStatus {
    /// `claude`, `codex` or `antigravity`.
    pub provider: &'static str,
    /// Display name.
    pub name: &'static str,
    /// WSL distro the checks target, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distro: Option<String>,
    /// CLI version line when installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Signed-in account label (email) when the CLI records one; never a credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// `true` when every required step is done.
    pub ready: bool,
    /// Ordered checklist.
    pub steps: Vec<Step>,
}

/// Everything the evaluation reads, so tests can use temp homes.
pub struct Inputs<'a> {
    /// Provider id.
    pub provider: &'a str,
    /// WSL distro, if the checks target one.
    pub distro: Option<&'a str>,
    /// `false` when the distro is stopped: its files are not read (discovery never boots one).
    pub reachable: bool,
    /// Linux or user home holding `.claude`, `.codex`, `.gemini`.
    pub home: PathBuf,
    /// Claude config dir (`CLAUDE_CONFIG_DIR` or `<home>/.claude`).
    pub claude_dir: PathBuf,
    /// Codex home (`CODEX_HOME` or `<home>/.codex`).
    pub codex_dir: PathBuf,
    /// Gemini home (`GEMINI_HOME` or `<home>/.gemini`).
    pub gemini_dir: PathBuf,
    /// Gnomcode home (opt-in files).
    pub gnomcode_home: PathBuf,
    /// CLI probe result; `None` when not probed.
    pub cli: Option<AgentCliStatus>,
    /// Protocol approvals opted in for this agent.
    pub protocol_approvals: bool,
    /// Latest recorded session for this agent, as a short label.
    pub first_session: Option<String>,
    /// A Claude limits reading exists.
    pub claude_reading: bool,
    /// Target OS family for install commands: `windows`, `macos`, `linux`.
    pub os: &'a str,
}

fn name(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude Code",
        "codex" => "Codex",
        _ => "Antigravity",
    }
}

/// Official install command for `provider` on `os` (a distro always uses Linux).
#[must_use]
pub fn install_command(provider: &str, os: &str) -> String {
    match (provider, os) {
        ("claude", "windows") => "irm https://claude.ai/install.ps1 | iex".into(),
        ("claude", _) => "curl -fsSL https://claude.ai/install.sh | bash".into(),
        ("codex", _) => "npm install -g @openai/codex".into(),
        _ => "Download Antigravity from https://antigravity.google/download".into(),
    }
}

fn login_command(provider: &str) -> &'static str {
    match provider {
        "claude" => "claude  (then type /login)",
        "codex" => "codex login",
        _ => "agy  (then sign in with Google)",
    }
}

fn step(id: &'static str, label: &'static str, state: StepState, trust: Trust) -> Step {
    Step {
        id,
        label,
        state,
        optional: false,
        trust,
        note: String::new(),
        command: None,
    }
}

fn unreachable_step(id: &'static str, label: &'static str, distro: &str) -> Step {
    Step {
        note: format!("{distro} is not running; start it to check"),
        ..step(id, label, StepState::Unknown, Trust::Unknown)
    }
}

fn installed_step(inputs: &Inputs<'_>, os: &str) -> Step {
    let base = step(
        "installed",
        "CLI installed",
        StepState::Unknown,
        Trust::Unknown,
    );
    match &inputs.cli {
        Some(cli) if cli.detected => Step {
            state: StepState::Done,
            trust: Trust::Exact,
            note: cli
                .version
                .clone()
                .unwrap_or_else(|| "Found; version not reported".into()),
            ..base
        },
        Some(_) => Step {
            state: StepState::Todo,
            trust: Trust::Exact,
            note: "Not found on PATH".into(),
            command: Some(install_command(inputs.provider, os)),
            ..base
        },
        None => Step {
            note: "Not checked".into(),
            command: Some(install_command(inputs.provider, os)),
            ..base
        },
    }
}

/// Claude account email from `.claude.json` (a non-secret field), if recorded.
fn claude_account(inputs: &Inputs<'_>) -> Option<String> {
    let candidates = [
        inputs.claude_dir.join(".claude.json"),
        inputs.home.join(".claude.json"),
    ];
    candidates.iter().find_map(|path| {
        let value: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        value
            .pointer("/oauthAccount/emailAddress")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })
}

fn signed_in_step(inputs: &Inputs<'_>, account: Option<&str>) -> Step {
    let base = step("signed_in", "Signed in", StepState::Todo, Trust::Estimated);
    let todo = |note: &str| Step {
        note: note.into(),
        command: Some(login_command(inputs.provider).into()),
        ..base.clone()
    };
    match inputs.provider {
        "claude" => {
            let creds = inputs.claude_dir.join(".credentials.json");
            if creds.is_file() || account.is_some() {
                Step {
                    state: StepState::Done,
                    note: account.map_or_else(
                        || "Sign-in recorded".into(),
                        |email| format!("Signed in as {email}"),
                    ),
                    ..base
                }
            } else {
                todo("No sign-in recorded")
            }
        }
        "codex" => {
            if inputs.codex_dir.join("auth.json").is_file() {
                Step {
                    state: StepState::Done,
                    note: "Sign-in recorded".into(),
                    ..base
                }
            } else {
                todo("No sign-in recorded")
            }
        }
        _ => {
            let ran = ["antigravity", "antigravity-cli", "antigravity-ide"]
                .iter()
                .any(|dir| inputs.gemini_dir.join(dir).is_dir());
            if ran {
                Step {
                    state: StepState::Done,
                    note: "Antigravity has run here; sign-in assumed".into(),
                    ..base
                }
            } else {
                todo("Antigravity has not run here yet")
            }
        }
    }
}

fn integrated_step(inputs: &Inputs<'_>) -> Step {
    let check = crate::doctor::check_claude_integration(&inputs.claude_dir);
    let hook = check
        .details
        .as_ref()
        .and_then(|d| d.get("hook_installed"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let base = step("integrated", "Gnomcode hook", StepState::Todo, Trust::Exact);
    if hook {
        Step {
            state: StepState::Done,
            note: check.summary,
            ..base
        }
    } else {
        Step {
            note: "Adds the hook, limits status line and phone approvals; backs up settings.json first; undo with --remove".into(),
            command: Some("gnomcode integrate claude".into()),
            ..base
        }
    }
}

fn protocol_step(inputs: &Inputs<'_>) -> Step {
    let base = Step {
        optional: true,
        ..step(
            "protocol_approvals",
            "Phone and console approvals",
            StepState::Todo,
            Trust::Exact,
        )
    };
    if inputs.protocol_approvals {
        Step {
            state: StepState::Done,
            note: "New launches run in protocol mode".into(),
            ..base
        }
    } else {
        Step {
            note: "Optional: answer approval prompts from your phone or the console".into(),
            command: Some(format!(
                "gnomcode protocol-approvals enable {}",
                inputs.provider
            )),
            ..base
        }
    }
}

fn limits_step(inputs: &Inputs<'_>) -> Step {
    let consent = inputs.gnomcode_home.join("vendor-limits.json");
    let enabled = crate::vendor_limits::enabled_in(&consent, "claude")
        || inputs
            .gnomcode_home
            .join(gnomcode_ingest::CLAUDE_LIMITS_FILE)
            .is_file();
    let base = Step {
        optional: true,
        ..step("limits", "Exact limits", StepState::Todo, Trust::Exact)
    };
    match (enabled, inputs.claude_reading) {
        (_, true) => Step {
            state: StepState::Done,
            note: "Limits reading received".into(),
            ..base
        },
        (true, false) => Step {
            state: StepState::Waiting,
            note: "Enabled; waiting for the first reading (polled every 5 minutes)".into(),
            ..base
        },
        (false, false) => Step {
            note: "Optional: read plan limits from Anthropic with your existing sign-in".into(),
            command: Some("gnomcode limits enable claude".into()),
            ..base
        },
    }
}

fn first_data_step(inputs: &Inputs<'_>) -> Step {
    let base = step(
        "first_data",
        "First session seen",
        StepState::Waiting,
        Trust::Exact,
    );
    match &inputs.first_session {
        Some(label) => Step {
            state: StepState::Done,
            note: label.clone(),
            ..base
        },
        None => Step {
            note: "Start a session; it appears here once ingested".into(),
            command: Some(
                match inputs.provider {
                    "claude" => "claude",
                    "codex" => "codex",
                    _ => "agy",
                }
                .into(),
            ),
            ..base
        },
    }
}

/// Builds the checklist for one provider. Pure apart from reading small local files.
#[must_use]
pub fn evaluate(inputs: &Inputs<'_>) -> ProviderStatus {
    let provider = PROVIDERS
        .iter()
        .copied()
        .find(|p| *p == inputs.provider)
        .unwrap_or("antigravity");
    let os = if inputs.distro.is_some() {
        "linux"
    } else {
        inputs.os
    };
    let account = if provider == "claude" && inputs.reachable {
        claude_account(inputs)
    } else {
        None
    };
    let mut steps = vec![installed_step(inputs, os)];
    let distro = inputs.distro.unwrap_or("");
    let file_step = |id, label, build: &dyn Fn() -> Step| {
        if inputs.reachable {
            build()
        } else {
            unreachable_step(id, label, distro)
        }
    };
    steps.push(file_step("signed_in", "Signed in", &|| {
        signed_in_step(inputs, account.as_deref())
    }));
    if provider == "claude" {
        steps.push(file_step("integrated", "Gnomcode hook", &|| {
            integrated_step(inputs)
        }));
        steps.push(limits_step(inputs));
    } else {
        steps.push(protocol_step(inputs));
    }
    steps.push(first_data_step(inputs));
    let ready = steps
        .iter()
        .all(|s| s.optional || s.state == StepState::Done);
    ProviderStatus {
        provider,
        name: name(provider),
        distro: inputs.distro.map(str::to_owned),
        version: inputs.cli.as_ref().and_then(|c| c.version.clone()),
        account,
        ready,
        steps,
    }
}

fn env_dir(var: &str, fallback: PathBuf) -> PathBuf {
    std::env::var_os(var)
        .filter(|v| !v.is_empty())
        .map_or(fallback, PathBuf::from)
}

fn current_os() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn agent_id(provider: &str) -> (&'static str, &'static str) {
    match provider {
        "claude" => ("claude", "claude-code"),
        "codex" => ("codex", "codex"),
        _ => ("agy", "antigravity"),
    }
}

/// Short CLI probe cache so 2-second polling does not spawn `--version` every time.
static CLI_CACHE: std::sync::Mutex<Vec<(String, std::time::Instant, AgentCliStatus)>> =
    std::sync::Mutex::new(Vec::new());
const CLI_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(6);

async fn probe_cli(
    exe: &'static str,
    distro: Option<&str>,
    reachable: bool,
) -> Option<AgentCliStatus> {
    let key = format!("{exe}@{}", distro.unwrap_or(""));
    if let Ok(cache) = CLI_CACHE.lock()
        && let Some((_, at, hit)) = cache.iter().find(|(k, _, _)| *k == key)
        && at.elapsed() < CLI_CACHE_TTL
    {
        return Some(hit.clone());
    }
    let status = match distro {
        None => crate::doctor::probe_one_cli(exe).await,
        Some(_) if !reachable => return None,
        Some(distro) => {
            let probe = gnomcode_pty::wsl::probe(distro, exe).ok()?;
            let mut command = tokio::process::Command::new(&probe.executable);
            command
                .args(&probe.arguments)
                .stdin(std::process::Stdio::null())
                .kill_on_drop(true);
            // The daemon has no console: without this flag Windows flashes a console window for wsl.exe.
            #[cfg(windows)]
            command.creation_flags(0x0800_0000);
            let out = tokio::time::timeout(std::time::Duration::from_secs(4), command.output())
            .await
            .ok()?
            .ok()?;
            let found = gnomcode_pty::wsl::probe_found(&String::from_utf8_lossy(&out.stdout));
            AgentCliStatus {
                agent: exe.to_owned(),
                detected: found,
                binary: None,
                version: None,
                error: None,
            }
        }
    };
    if let Ok(mut cache) = CLI_CACHE.lock() {
        cache.retain(|(k, _, _)| *k != key);
        cache.push((key, std::time::Instant::now(), status.clone()));
    }
    Some(status)
}

fn wsl_home(distro: &str) -> Option<PathBuf> {
    let share = gnomcode_platform::wsl::distro_share(distro);
    let homes = gnomcode_platform::wsl::distro_homes(&share);
    homes
        .iter()
        .find(|h| {
            h.join(".claude").is_dir() || h.join(".codex").is_dir() || h.join(".gemini").is_dir()
        })
        .or_else(|| homes.iter().rev().find(|h| h.is_dir()))
        .cloned()
}

fn valid_distro(distro: &str) -> bool {
    !distro.is_empty()
        && !distro.starts_with('-')
        && distro
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
}

/// Live status for `provider`, optionally inside a running WSL distro.
pub async fn status(
    db: &std::sync::Arc<tokio::sync::Mutex<gnomcode_store::Database>>,
    provider: &str,
    distro: Option<&str>,
) -> Option<ProviderStatus> {
    if !PROVIDERS.contains(&provider) || distro.is_some_and(|d| !valid_distro(d)) {
        return None;
    }
    let (exe, agent) = agent_id(provider);
    let (reachable, home) = match distro {
        None => (true, Some(gnomcode_platform::user_home_dir())),
        Some(d) => {
            let owned = d.to_owned();
            tokio::task::spawn_blocking(move || {
                let running = gnomcode_platform::wsl::running_distros()
                    .iter()
                    .any(|r| r.eq_ignore_ascii_case(&owned));
                (running, running.then(|| wsl_home(&owned)).flatten())
            })
            .await
            .unwrap_or((false, None))
        }
    };
    let reachable = reachable && home.is_some();
    let home = home.unwrap_or_default();
    let (claude_dir, codex_dir, gemini_dir) = if distro.is_some() {
        (
            home.join(".claude"),
            home.join(".codex"),
            home.join(".gemini"),
        )
    } else {
        (
            crate::vendor_limits::claude_config_dir(),
            env_dir("CODEX_HOME", home.join(".codex")),
            env_dir("GEMINI_HOME", home.join(".gemini")),
        )
    };
    let cli = probe_cli(exe, distro, reachable).await;
    let (first_session, claude_reading) = {
        let guard = db.lock().await;
        let first = guard
            .list_sessions_paginated(Some(agent), None, 0, 1)
            .ok()
            .and_then(|(rows, _)| rows.into_iter().next())
            .map(|row| format!("{} in {}", row.model, row.project_name));
        let reading = guard
            .get_latest_quota_snapshots()
            .ok()
            .is_some_and(|rows| rows.iter().any(|r| r.provider == "claude-code"));
        (first, reading)
    };
    Some(evaluate(&Inputs {
        provider,
        distro,
        reachable,
        home,
        claude_dir,
        codex_dir,
        gemini_dir,
        gnomcode_home: gnomcode_platform::gnomcode_home(),
        cli,
        protocol_approvals: gnomcode_pty::protocol_host::protocol_approvals_enabled(provider),
        first_session,
        claude_reading,
        os: current_os(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn inputs<'a>(provider: &'a str, home: &Path) -> Inputs<'a> {
        Inputs {
            provider,
            distro: None,
            reachable: true,
            home: home.to_path_buf(),
            claude_dir: home.join(".claude"),
            codex_dir: home.join(".codex"),
            gemini_dir: home.join(".gemini"),
            gnomcode_home: home.join(".gnomcode"),
            cli: Some(AgentCliStatus {
                agent: provider.into(),
                detected: false,
                binary: None,
                version: None,
                error: None,
            }),
            protocol_approvals: false,
            first_session: None,
            claude_reading: false,
            os: "windows",
        }
    }

    fn state(status: &ProviderStatus, id: &str) -> StepState {
        status
            .steps
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.state)
            .unwrap_or(StepState::Unknown)
    }

    #[test]
    fn empty_home_needs_install_and_sign_in_with_commands() {
        let dir = tempfile::tempdir().unwrap();
        let s = evaluate(&inputs("claude", dir.path()));
        assert!(!s.ready);
        assert_eq!(state(&s, "installed"), StepState::Todo);
        assert_eq!(
            s.steps[0].command.as_deref(),
            Some("irm https://claude.ai/install.ps1 | iex")
        );
        assert_eq!(state(&s, "signed_in"), StepState::Todo);
        assert_eq!(state(&s, "integrated"), StepState::Todo);
        assert_eq!(state(&s, "limits"), StepState::Todo);
        assert_eq!(state(&s, "first_data"), StepState::Waiting);
    }

    #[test]
    fn claude_fully_set_up_is_ready_and_shows_account_not_token() {
        let dir = tempfile::tempdir().unwrap();
        let claude = dir.path().join(".claude");
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(
            claude.join(".credentials.json"),
            r#"{"claudeAiOauth":{"accessToken":"SECRET","expiresAt":1}}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join(".claude.json"),
            r#"{"oauthAccount":{"emailAddress":"dev@example.com"}}"#,
        )
        .unwrap();
        std::fs::write(
            claude.join("settings.json"),
            r#"{"hooks":{"PreToolUse":[{"hooks":[{"command":"gnomcode","args":["hook"]}]}]}}"#,
        )
        .unwrap();
        let mut i = inputs("claude", dir.path());
        i.cli.as_mut().unwrap().detected = true;
        i.cli.as_mut().unwrap().version = Some("2.0.0 (Claude Code)".into());
        i.first_session = Some("opus in demo".into());
        i.claude_reading = true;
        let s = evaluate(&i);
        assert!(s.ready, "{s:?}");
        assert_eq!(s.account.as_deref(), Some("dev@example.com"));
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("SECRET"));
        assert_eq!(state(&s, "limits"), StepState::Done);
    }

    #[test]
    fn claude_limits_enabled_waits_for_first_reading() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join(".gnomcode");
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(home.join("vendor-limits.json"), r#"{"claude":true}"#).unwrap();
        let s = evaluate(&inputs("claude", dir.path()));
        assert_eq!(state(&s, "limits"), StepState::Waiting);
    }

    #[test]
    fn codex_sign_in_and_protocol_approvals_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let mut i = inputs("codex", dir.path());
        let s = evaluate(&i);
        assert_eq!(state(&s, "signed_in"), StepState::Todo);
        let proto = s
            .steps
            .iter()
            .find(|x| x.id == "protocol_approvals")
            .unwrap();
        assert!(proto.optional);
        assert_eq!(
            proto.command.as_deref(),
            Some("gnomcode protocol-approvals enable codex")
        );
        std::fs::create_dir_all(dir.path().join(".codex")).unwrap();
        std::fs::write(dir.path().join(".codex").join("auth.json"), "{}").unwrap();
        i.protocol_approvals = true;
        let s = evaluate(&i);
        assert_eq!(state(&s, "signed_in"), StepState::Done);
        assert_eq!(state(&s, "protocol_approvals"), StepState::Done);
    }

    #[test]
    fn antigravity_sign_in_is_estimated_from_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".gemini").join("antigravity-cli")).unwrap();
        let s = evaluate(&inputs("antigravity", dir.path()));
        let step = s.steps.iter().find(|x| x.id == "signed_in").unwrap();
        assert_eq!(step.state, StepState::Done);
        assert_eq!(step.trust, Trust::Estimated);
        assert!(s.steps.iter().all(|x| x.id != "limits"));
    }

    #[test]
    fn stopped_distro_reports_unknown_and_linux_install_command() {
        let dir = tempfile::tempdir().unwrap();
        let mut i = inputs("claude", dir.path());
        i.distro = Some("Ubuntu");
        i.reachable = false;
        i.cli = None;
        let s = evaluate(&i);
        assert_eq!(s.distro.as_deref(), Some("Ubuntu"));
        assert_eq!(state(&s, "signed_in"), StepState::Unknown);
        assert_eq!(state(&s, "integrated"), StepState::Unknown);
        assert_eq!(
            s.steps[0].command.as_deref(),
            Some("curl -fsSL https://claude.ai/install.sh | bash")
        );
    }

    #[test]
    fn rejects_unsafe_distro_names() {
        assert!(valid_distro("Ubuntu-22.04"));
        assert!(!valid_distro("-x"));
        assert!(!valid_distro("a b"));
    }
}
