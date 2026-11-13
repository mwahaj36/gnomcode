//! Git worktree per session: create, merge, discard and open a pull request (roadmap F1, ADR 0051).
//!
//! A worktree session runs its agent in a separate checkout under the Gnomcode data folder
//! (`<home>/worktrees`, never the system temp folder), on its own branch. Everything here runs
//! `git` (and optionally `gh`) with fixed arguments and paths taken from Gnomcode's own records,
//! never from request text. Nothing forces, rebases or switches the user's branch: a merge
//! happens only when the user asks for it, only into the branch the worktree was cut from, and
//! only when that branch is already checked out and clean in the original repository.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

const GIT_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_BRANCH_CHARS: usize = 120;

/// A refusal or failure, worded for the person using the app.
#[derive(Debug, Error)]
pub(crate) enum WorktreeError {
    #[error("That folder is not inside a git repository, so it cannot be isolated in a worktree.")]
    NotGit,
    #[error("This repository has no commits yet. Make a first commit before isolating a session.")]
    NoCommits,
    #[error("Check out a branch first: HEAD is detached, so there is nothing to merge back into.")]
    DetachedHead,
    #[error("\"{0}\" is not a valid branch name.")]
    InvalidBranch(String),
    #[error("The branch \"{0}\" already exists. Pick another branch name.")]
    BranchExists(String),
    #[error("git is not installed or not on PATH.")]
    GitMissing,
    #[error("git: {0}")]
    Git(String),
    #[error("This session has no worktree.")]
    NotFound,
    #[error("Stop this session first: its agent is still using the worktree.")]
    InUse,
    #[error("Check out \"{base}\" in {repo} first. Gnomcode never switches your branch for you.")]
    BaseNotCheckedOut { base: String, repo: String },
    #[error(
        "{repo} has uncommitted changes to tracked files. Commit or stash them, then try again."
    )]
    DirtyRepo { repo: String },
    #[error("Merge conflicts in: {0}. Nothing was merged and the worktree is untouched.")]
    Conflicts(String),
    #[error(
        "GitHub CLI (gh) is not installed or not signed in. Install it and run `gh auth login`, or push the branch yourself."
    )]
    GhUnavailable,
    #[error("This repository has no \"origin\" remote to push to.")]
    NoRemote,
    #[error("Could not open the pull request: {0}")]
    PullRequest(String),
    #[error("Worktree files could not be read or written: {0}")]
    Io(String),
}

impl WorktreeError {
    pub(crate) fn status(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InUse
            | Self::BaseNotCheckedOut { .. }
            | Self::DirtyRepo { .. }
            | Self::Conflicts(_)
            | Self::BranchExists(_) => StatusCode::CONFLICT,
            Self::NotGit
            | Self::NoCommits
            | Self::DetachedHead
            | Self::InvalidBranch(_)
            | Self::GhUnavailable
            | Self::NoRemote => StatusCode::UNPROCESSABLE_ENTITY,
            Self::GitMissing | Self::Git(_) | Self::PullRequest(_) | Self::Io(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

/// What Gnomcode remembers about one worktree session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorktreeRecord {
    pub(crate) session_id: String,
    /// Top level of the original repository.
    pub(crate) repo: String,
    /// Top level of the worktree checkout.
    pub(crate) path: String,
    /// Folder the agent starts in (the worktree top level, or the same subfolder as requested).
    pub(crate) cwd: String,
    pub(crate) branch: String,
    /// Branch the worktree was cut from, and the only merge target.
    pub(crate) base_branch: String,
    /// Commit the worktree was cut from; the review diff is taken against it.
    pub(crate) base_commit: String,
    pub(crate) created_at: String,
}

/// Where managed worktrees and their records live.
pub(crate) fn managed_root() -> PathBuf {
    gnomcode_platform::gnomcode_home().join("worktrees")
}

fn record_path(root: &Path, session_id: &str) -> PathBuf {
    root.join("records").join(format!("{session_id}.json"))
}

fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        && !id.starts_with('.')
}

fn save_record(root: &Path, record: &WorktreeRecord) -> Result<(), WorktreeError> {
    let path = record_path(root, &record.session_id);
    let bytes = serde_json::to_vec_pretty(record).map_err(|e| WorktreeError::Io(e.to_string()))?;
    std::fs::create_dir_all(path.parent().unwrap_or(root))
        .and_then(|()| std::fs::write(&path, bytes))
        .map_err(|e| WorktreeError::Io(e.to_string()))
}

pub(crate) fn load_record(root: &Path, session_id: &str) -> Option<WorktreeRecord> {
    if !valid_session_id(session_id) {
        return None;
    }
    serde_json::from_slice(&std::fs::read(record_path(root, session_id)).ok()?).ok()
}

fn remove_record(root: &Path, session_id: &str) {
    if valid_session_id(session_id) {
        let _ = std::fs::remove_file(record_path(root, session_id));
    }
}

/// All stored records by session id.
pub(crate) fn record_index(root: &Path) -> HashMap<String, WorktreeRecord> {
    let Ok(entries) = std::fs::read_dir(root.join("records")) else {
        return HashMap::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice::<WorktreeRecord>(&bytes).ok())
        .map(|record| (record.session_id.clone(), record))
        .collect()
}

fn same_or_inside(path: &Path, parent: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    path.starts_with(parent)
}

/// True when any live hosted agent is running inside this worktree.
pub(crate) fn in_use(record: &WorktreeRecord) -> bool {
    gnomcode_pty::list_active_sessions()
        .iter()
        .any(|session| same_or_inside(Path::new(&session.repo_path), Path::new(&record.path)))
}

/// The commit a workspace's review diff should be taken against, when it is a managed worktree.
pub(crate) fn diff_base(root: &Path, workspace: &Path) -> Option<String> {
    record_index(root)
        .into_values()
        .find(|record| same_or_inside(workspace, Path::new(&record.path)))
        .map(|record| record.base_commit)
        .filter(|sha| !sha.is_empty() && sha.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

struct Output {
    ok: bool,
    stdout: String,
    stderr: String,
}

async fn run(program: &str, dir: &Path, args: &[&str]) -> Result<Output, WorktreeError> {
    let mut command = tokio::process::Command::new(program);
    command
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .env("LC_ALL", "C")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            WorktreeError::GitMissing
        } else {
            WorktreeError::Io(error.to_string())
        }
    })?;
    let output = tokio::time::timeout(GIT_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| WorktreeError::Git("timed out".into()))?
        .map_err(|error| WorktreeError::Io(error.to_string()))?;
    Ok(Output {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
    })
}

fn last_line(text: &str) -> String {
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("command failed")
        .chars()
        .take(300)
        .collect()
}

/// Runs git and returns trimmed stdout, or its last error line.
async fn git(dir: &Path, args: &[&str]) -> Result<String, WorktreeError> {
    let out = run("git", dir, args).await?;
    if out.ok {
        Ok(out.stdout)
    } else {
        Err(WorktreeError::Git(last_line(&if out.stderr.is_empty() {
            out.stdout
        } else {
            out.stderr
        })))
    }
}

async fn git_succeeds(dir: &Path, args: &[&str]) -> Result<bool, WorktreeError> {
    Ok(run("git", dir, args).await?.ok)
}

/// Creates a worktree for `session_id` from the current branch of the repository at `repo_path`.
pub(crate) async fn create(
    root: &Path,
    repo_path: &Path,
    session_id: &str,
    branch: Option<&str>,
) -> Result<WorktreeRecord, WorktreeError> {
    if !valid_session_id(session_id) {
        return Err(WorktreeError::Io("invalid session id".into()));
    }
    let top = git(repo_path, &["rev-parse", "--show-toplevel"])
        .await
        .map_err(|error| match error {
            WorktreeError::GitMissing => error,
            _ => WorktreeError::NotGit,
        })?;
    let top = PathBuf::from(top);
    let base_commit = git(&top, &["rev-parse", "--verify", "HEAD"])
        .await
        .map_err(|_| WorktreeError::NoCommits)?;
    let base_branch = git(&top, &["symbolic-ref", "--short", "-q", "HEAD"])
        .await
        .map_err(|_| WorktreeError::DetachedHead)?;
    let branch = match branch.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => {
            if name.chars().count() > MAX_BRANCH_CHARS
                || name.starts_with('-')
                || !git_succeeds(&top, &["check-ref-format", "--branch", name]).await?
            {
                return Err(WorktreeError::InvalidBranch(
                    name.chars().take(60).collect(),
                ));
            }
            name.to_owned()
        }
        None => format!("gnomcode/{session_id}"),
    };
    if git_succeeds(
        &top,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )
    .await?
    {
        return Err(WorktreeError::BranchExists(branch));
    }
    let name = top
        .file_name()
        .map_or_else(|| "repo".into(), |name| name.to_string_lossy().into_owned());
    let checkout = root.join("checkouts").join(session_id).join(name);
    std::fs::create_dir_all(checkout.parent().unwrap_or(root))
        .map_err(|error| WorktreeError::Io(error.to_string()))?;
    git(
        &top,
        &[
            "worktree",
            "add",
            "-b",
            &branch,
            &checkout.to_string_lossy(),
            &base_commit,
        ],
    )
    .await?;
    // Start the agent in the same subfolder the user chose, when it exists in the checkout.
    let relative = repo_path
        .canonicalize()
        .ok()
        .zip(top.canonicalize().ok())
        .and_then(|(chosen, top)| chosen.strip_prefix(top).ok().map(Path::to_path_buf))
        .unwrap_or_default();
    let cwd = if checkout.join(&relative).is_dir() {
        checkout.join(&relative)
    } else {
        checkout.clone()
    };
    let record = WorktreeRecord {
        session_id: session_id.to_owned(),
        repo: top.to_string_lossy().into_owned(),
        path: checkout.to_string_lossy().into_owned(),
        cwd: cwd.to_string_lossy().into_owned(),
        branch,
        base_branch,
        base_commit,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    if let Err(error) = save_record(root, &record) {
        let _ = remove_worktree(root, &record, true).await;
        return Err(error);
    }
    Ok(record)
}

/// Removes the worktree folder (with any `target/` or `node_modules/` inside), prunes git's
/// bookkeeping and optionally deletes the branch. Never touches any other branch.
async fn remove_worktree(
    root: &Path,
    record: &WorktreeRecord,
    delete_branch_hard: bool,
) -> Result<(), WorktreeError> {
    let repo = Path::new(&record.repo);
    let path = Path::new(&record.path);
    // Only ever delete inside the managed checkouts folder, even if a record file is edited.
    let managed = root.join("checkouts");
    if !path.starts_with(&managed)
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        || record.branch == record.base_branch
    {
        return Err(WorktreeError::Io(
            "this worktree record points outside the Gnomcode worktree folder".into(),
        ));
    }
    let _ = run(
        "git",
        repo,
        &["worktree", "remove", "--force", &record.path],
    )
    .await;
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|error| {
            WorktreeError::Io(format!(
                "could not delete {}: {error}. Close anything using it and discard again.",
                record.path
            ))
        })?;
    }
    if let Some(parent) = path.parent() {
        // The per-session folder only holds this checkout.
        let _ = std::fs::remove_dir(parent);
    }
    let _ = run("git", repo, &["worktree", "prune"]).await;
    let flag = if delete_branch_hard { "-D" } else { "-d" };
    let _ = run("git", repo, &["branch", flag, &record.branch]).await;
    remove_record(root, &record.session_id);
    Ok(())
}

/// Undoes a launch whose agent never started.
pub(crate) async fn abort_launch(root: &Path, record: &WorktreeRecord) {
    let _ = remove_worktree(root, record, true).await;
}

/// Saves the agent's uncommitted work on the session branch. Returns whether a commit was made.
async fn commit_all(record: &WorktreeRecord) -> Result<bool, WorktreeError> {
    let path = Path::new(&record.path);
    if git(path, &["status", "--porcelain"]).await?.is_empty() {
        return Ok(false);
    }
    git(path, &["add", "-A"]).await?;
    git(
        path,
        &[
            "commit",
            "-q",
            "-m",
            &format!("Gnomcode: save session work on {}", record.branch),
        ],
    )
    .await
    .map_err(|error| match error {
        WorktreeError::Git(message) => WorktreeError::Git(format!(
            "could not save the session's changes ({message}). Is git user.name and user.email set?"
        )),
        other => other,
    })?;
    Ok(true)
}

async fn commits_ahead(record: &WorktreeRecord) -> Result<u64, WorktreeError> {
    let range = format!("{}..{}", record.base_commit, record.branch);
    git(Path::new(&record.path), &["rev-list", "--count", &range])
        .await?
        .parse()
        .map_err(|_| WorktreeError::Git("unreadable commit count".into()))
}

/// Result of a merge.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Merged {
    /// Merge commit, or `None` when the worktree had no changes.
    pub(crate) commit: Option<String>,
    pub(crate) base_branch: String,
    /// Set when the folder could not be fully removed after a successful merge.
    pub(crate) cleanup_warning: Option<String>,
}

/// Merges the session branch into its base branch, then removes the worktree.
pub(crate) async fn merge(root: &Path, record: &WorktreeRecord) -> Result<Merged, WorktreeError> {
    let repo = Path::new(&record.repo);
    commit_all(record).await?;
    if commits_ahead(record).await? == 0 {
        let cleanup_warning = remove_worktree(root, record, false)
            .await
            .err()
            .map(|e| e.to_string());
        return Ok(Merged {
            commit: None,
            base_branch: record.base_branch.clone(),
            cleanup_warning,
        });
    }
    let current = git(repo, &["symbolic-ref", "--short", "-q", "HEAD"])
        .await
        .unwrap_or_default();
    if current != record.base_branch {
        return Err(WorktreeError::BaseNotCheckedOut {
            base: record.base_branch.clone(),
            repo: record.repo.clone(),
        });
    }
    if !git(repo, &["status", "--porcelain", "--untracked-files=no"])
        .await?
        .is_empty()
    {
        return Err(WorktreeError::DirtyRepo {
            repo: record.repo.clone(),
        });
    }
    let message = format!("Merge {} from a Gnomcode worktree session", record.branch);
    let merged = run(
        "git",
        repo,
        &["merge", "--no-ff", "-m", &message, &record.branch],
    )
    .await?;
    if !merged.ok {
        let conflicted = git(repo, &["diff", "--name-only", "--diff-filter=U"])
            .await
            .unwrap_or_default();
        let _ = run("git", repo, &["merge", "--abort"]).await;
        return Err(if conflicted.is_empty() {
            WorktreeError::Git(last_line(&if merged.stderr.is_empty() {
                merged.stdout
            } else {
                merged.stderr
            }))
        } else {
            WorktreeError::Conflicts(conflicted.lines().collect::<Vec<_>>().join(", "))
        });
    }
    let commit = git(repo, &["rev-parse", "HEAD"]).await?;
    let cleanup_warning = remove_worktree(root, record, false)
        .await
        .err()
        .map(|e| e.to_string());
    Ok(Merged {
        commit: Some(commit),
        base_branch: record.base_branch.clone(),
        cleanup_warning,
    })
}

/// Removes the worktree and deletes its branch. The base branch is never touched.
pub(crate) async fn discard(root: &Path, record: &WorktreeRecord) -> Result<(), WorktreeError> {
    remove_worktree(root, record, true).await
}

fn first_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|word| word.starts_with("https://"))
        .map(|word| word.trim_end_matches(['.', ',', ')']).to_owned())
}

/// Pushes the session branch (never forced) and opens a pull request with `gh`.
/// Returns the pull request URL. The worktree stays in place.
pub(crate) async fn open_pr(record: &WorktreeRecord, gh: &str) -> Result<String, WorktreeError> {
    let path = Path::new(&record.path);
    let ready = match run(gh, path, &["auth", "status"]).await {
        Ok(out) => out.ok,
        Err(WorktreeError::GitMissing) => false,
        Err(error) => return Err(error),
    };
    if !ready {
        return Err(WorktreeError::GhUnavailable);
    }
    if !git_succeeds(path, &["remote", "get-url", "origin"]).await? {
        return Err(WorktreeError::NoRemote);
    }
    commit_all(record).await?;
    if commits_ahead(record).await? == 0 {
        return Err(WorktreeError::PullRequest(
            "this worktree has no changes yet".into(),
        ));
    }
    git(path, &["push", "-u", "origin", &record.branch]).await?;
    let created = run(
        gh,
        path,
        &[
            "pr",
            "create",
            "--fill",
            "--base",
            &record.base_branch,
            "--head",
            &record.branch,
        ],
    )
    .await?;
    let text = format!("{}\n{}", created.stdout, created.stderr);
    match (created.ok, first_url(&text)) {
        (true, Some(url)) => Ok(url),
        (false, Some(url)) if text.contains("already exists") => Ok(url),
        _ => Err(WorktreeError::PullRequest(last_line(&text))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(dir: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_owned()
    }

    /// File contents with line endings normalised, since git may write CRLF on Windows.
    fn text(path: &Path) -> String {
        std::fs::read_to_string(path)
            .expect("read")
            .lines()
            .map(|line| format!("{line}\n"))
            .collect()
    }

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("repo");
        sh(dir.path(), &["init", "-q", "-b", "main"]);
        sh(dir.path(), &["config", "user.email", "t@example.com"]);
        sh(dir.path(), &["config", "user.name", "Test"]);
        sh(dir.path(), &["config", "commit.gpgsign", "false"]);
        std::fs::create_dir_all(dir.path().join("src")).expect("src");
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("a");
        std::fs::write(dir.path().join(".gitignore"), "target/\n").expect("ignore");
        std::fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").expect("lib");
        sh(dir.path(), &["add", "-A"]);
        sh(dir.path(), &["commit", "-q", "-m", "init"]);
        dir
    }

    #[tokio::test]
    async fn discard_refuses_records_that_point_outside_the_managed_folder() {
        let root = tempfile::tempdir().expect("root");
        let victim = tempfile::tempdir().expect("victim");
        std::fs::write(victim.path().join("keep.txt"), "keep").expect("file");
        let record = WorktreeRecord {
            session_id: "s1".into(),
            repo: victim.path().to_string_lossy().into_owned(),
            path: victim.path().to_string_lossy().into_owned(),
            cwd: victim.path().to_string_lossy().into_owned(),
            branch: "gnomcode/s1".into(),
            base_branch: "main".into(),
            base_commit: "0".repeat(40),
            created_at: String::new(),
        };
        assert!(discard(root.path(), &record).await.is_err());
        assert!(victim.path().join("keep.txt").exists());
        let same_branch = WorktreeRecord {
            path: root
                .path()
                .join("checkouts")
                .join("s1")
                .to_string_lossy()
                .into_owned(),
            branch: "main".into(),
            ..record
        };
        assert!(discard(root.path(), &same_branch).await.is_err());
    }

    #[tokio::test]
    async fn create_makes_a_managed_checkout_on_its_own_branch() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(root.path(), repo.path(), "claude-1", None)
            .await
            .expect("create");
        assert_eq!(record.branch, "gnomcode/claude-1");
        assert_eq!(record.base_branch, "main");
        assert_eq!(record.base_commit, sh(repo.path(), &["rev-parse", "HEAD"]));
        let path = PathBuf::from(&record.path);
        assert!(path.starts_with(root.path().join("checkouts")));
        assert!(path.join("a.txt").is_file());
        assert_eq!(
            sh(&path, &["symbolic-ref", "--short", "HEAD"]),
            record.branch
        );
        assert_eq!(load_record(root.path(), "claude-1"), Some(record));
        // The original checkout is untouched.
        assert_eq!(
            sh(repo.path(), &["symbolic-ref", "--short", "HEAD"]),
            "main"
        );
    }

    #[tokio::test]
    async fn create_starts_the_agent_in_the_chosen_subfolder() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(
            root.path(),
            &repo.path().join("src"),
            "codex-1",
            Some("feature/x"),
        )
        .await
        .expect("create");
        assert_eq!(record.branch, "feature/x");
        assert!(Path::new(&record.cwd).ends_with("src"));
        assert!(Path::new(&record.cwd).starts_with(&record.path));
    }

    #[tokio::test]
    async fn create_refuses_bad_input_without_changing_the_repository() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        for bad in ["bad name", "-x", "a..b", "x.lock", &"y".repeat(200)] {
            assert!(
                matches!(
                    create(root.path(), repo.path(), "s1", Some(bad)).await,
                    Err(WorktreeError::InvalidBranch(_))
                ),
                "{bad}"
            );
        }
        assert!(matches!(
            create(root.path(), repo.path(), "s1", Some("main")).await,
            Err(WorktreeError::BranchExists(_))
        ));
        let plain = tempfile::tempdir().expect("plain");
        assert!(matches!(
            create(root.path(), plain.path(), "s2", None).await,
            Err(WorktreeError::NotGit)
        ));
        let empty = tempfile::tempdir().expect("empty");
        sh(empty.path(), &["init", "-q"]);
        assert!(matches!(
            create(root.path(), empty.path(), "s3", None).await,
            Err(WorktreeError::NoCommits)
        ));
        sh(repo.path(), &["checkout", "-q", "--detach"]);
        assert!(matches!(
            create(root.path(), repo.path(), "s4", None).await,
            Err(WorktreeError::DetachedHead)
        ));
        assert!(record_index(root.path()).is_empty());
        assert!(!root.path().join("checkouts").join("s1").join("x").exists());
    }

    async fn worked(repo: &Path, root: &Path, id: &str) -> WorktreeRecord {
        let record = create(root, repo, id, None).await.expect("create");
        let path = PathBuf::from(&record.path);
        std::fs::write(path.join("new.txt"), "agent work\n").expect("new");
        std::fs::write(path.join("a.txt"), "one\ntwo\n").expect("edit");
        std::fs::create_dir_all(path.join("target/debug")).expect("target");
        std::fs::write(path.join("target/debug/big.bin"), vec![0u8; 4096]).expect("bin");
        record
    }

    #[tokio::test]
    async fn merge_saves_work_merges_into_base_and_cleans_up_everything() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let before = sh(repo.path(), &["rev-parse", "HEAD"]);
        let record = worked(repo.path(), root.path(), "claude-2").await;
        let merged = merge(root.path(), &record).await.expect("merge");
        assert_eq!(merged.base_branch, "main");
        assert!(merged.cleanup_warning.is_none());
        let head = sh(repo.path(), &["rev-parse", "HEAD"]);
        assert_eq!(merged.commit.as_deref(), Some(head.as_str()));
        // History is preserved: the previous tip is an ancestor, nothing was rewritten.
        sh(
            repo.path(),
            &["merge-base", "--is-ancestor", &before, "HEAD"],
        );
        assert_eq!(text(&repo.path().join("new.txt")), "agent work\n");
        assert_eq!(text(&repo.path().join("a.txt")), "one\ntwo\n");
        assert!(
            !repo.path().join("target").exists(),
            "build output must not be committed"
        );
        assert!(
            !Path::new(&record.path).exists(),
            "worktree and target/ removed"
        );
        assert!(!Path::new(&record.path).parent().expect("parent").exists());
        assert!(sh(repo.path(), &["branch", "--list", &record.branch]).is_empty());
        assert!(!sh(repo.path(), &["worktree", "list"]).contains("checkouts"));
        assert!(load_record(root.path(), "claude-2").is_none());
    }

    #[tokio::test]
    async fn merge_with_no_changes_just_cleans_up() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(root.path(), repo.path(), "codex-2", None)
            .await
            .expect("create");
        let merged = merge(root.path(), &record).await.expect("merge");
        assert!(merged.commit.is_none());
        assert!(!Path::new(&record.path).exists());
        assert!(sh(repo.path(), &["branch", "--list", &record.branch]).is_empty());
    }

    #[tokio::test]
    async fn merge_refuses_unsafe_repository_state_and_keeps_the_worktree() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = worked(repo.path(), root.path(), "claude-3").await;
        // Original checkout on another branch: never switched for the user.
        sh(repo.path(), &["checkout", "-q", "-b", "other"]);
        assert!(matches!(
            merge(root.path(), &record).await,
            Err(WorktreeError::BaseNotCheckedOut { .. })
        ));
        assert_eq!(
            sh(repo.path(), &["symbolic-ref", "--short", "HEAD"]),
            "other"
        );
        // On the base branch but with uncommitted tracked changes.
        sh(repo.path(), &["checkout", "-q", "main"]);
        std::fs::write(repo.path().join("a.txt"), "dirty\n").expect("dirty");
        assert!(matches!(
            merge(root.path(), &record).await,
            Err(WorktreeError::DirtyRepo { .. })
        ));
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).expect("a"),
            "dirty\n"
        );
        assert!(Path::new(&record.path).is_dir());
        assert!(load_record(root.path(), "claude-3").is_some());
    }

    #[tokio::test]
    async fn merge_conflict_is_aborted_and_reported() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(root.path(), repo.path(), "claude-4", None)
            .await
            .expect("create");
        std::fs::write(Path::new(&record.path).join("a.txt"), "from agent\n").expect("agent");
        std::fs::write(repo.path().join("a.txt"), "from user\n").expect("user");
        sh(repo.path(), &["commit", "-q", "-am", "user edit"]);
        let tip = sh(repo.path(), &["rev-parse", "HEAD"]);
        let error = merge(root.path(), &record).await.expect_err("conflict");
        assert!(
            matches!(&error, WorktreeError::Conflicts(files) if files.contains("a.txt")),
            "{error}"
        );
        assert_eq!(sh(repo.path(), &["rev-parse", "HEAD"]), tip);
        assert!(
            sh(
                repo.path(),
                &["status", "--porcelain", "--untracked-files=no"]
            )
            .is_empty()
        );
        assert!(Path::new(&record.path).is_dir());
    }

    #[tokio::test]
    async fn discard_removes_checkout_build_dirs_and_branch_but_not_base() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let before = sh(repo.path(), &["rev-parse", "HEAD"]);
        let record = worked(repo.path(), root.path(), "gemini-1").await;
        discard(root.path(), &record).await.expect("discard");
        assert!(!Path::new(&record.path).exists());
        assert!(sh(repo.path(), &["branch", "--list", &record.branch]).is_empty());
        assert_eq!(sh(repo.path(), &["rev-parse", "HEAD"]), before);
        assert!(!repo.path().join("new.txt").exists());
        assert!(load_record(root.path(), "gemini-1").is_none());
    }

    #[tokio::test]
    async fn diff_base_identifies_worktree_paths_only() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(root.path(), repo.path(), "claude-5", None)
            .await
            .expect("create");
        assert_eq!(
            diff_base(root.path(), Path::new(&record.path)),
            Some(record.base_commit.clone())
        );
        assert_eq!(
            diff_base(root.path(), &Path::new(&record.path).join("src")),
            Some(record.base_commit)
        );
        assert_eq!(diff_base(root.path(), repo.path()), None);
    }

    #[tokio::test]
    async fn abort_launch_rolls_everything_back() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = create(root.path(), repo.path(), "claude-6", None)
            .await
            .expect("create");
        abort_launch(root.path(), &record).await;
        assert!(!Path::new(&record.path).exists());
        assert!(sh(repo.path(), &["branch", "--list", &record.branch]).is_empty());
        assert!(record_index(root.path()).is_empty());
    }

    fn fake_gh(dir: &Path, ready: bool) -> String {
        #[cfg(windows)]
        {
            let path = dir.join("gh.cmd");
            let script = format!(
                "@echo off\r\nif \"%1\"==\"auth\" exit /b {}\r\necho https://github.com/o/r/pull/7\r\n",
                i32::from(!ready)
            );
            std::fs::write(&path, script).expect("gh");
            path.to_string_lossy().into_owned()
        }
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let path = dir.join("gh");
            let script = format!(
                "#!/bin/sh\nif [ \"$1\" = auth ]; then exit {}; fi\necho https://github.com/o/r/pull/7\n",
                i32::from(!ready)
            );
            std::fs::write(&path, script).expect("gh");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
            path.to_string_lossy().into_owned()
        }
    }

    #[tokio::test]
    async fn open_pr_needs_gh_and_a_remote_then_pushes_without_force() {
        let repo = repo();
        let root = tempfile::tempdir().expect("root");
        let record = worked(repo.path(), root.path(), "claude-7").await;
        let tools = tempfile::tempdir().expect("tools");
        // gh missing or signed out: clear error, nothing pushed, nothing committed.
        assert!(matches!(
            open_pr(&record, "gnomcode-no-such-gh-binary").await,
            Err(WorktreeError::GhUnavailable)
        ));
        let signed_out = fake_gh(tools.path(), false);
        assert!(matches!(
            open_pr(&record, &signed_out).await,
            Err(WorktreeError::GhUnavailable)
        ));
        let gh = fake_gh(tools.path(), true);
        assert!(matches!(
            open_pr(&record, &gh).await,
            Err(WorktreeError::NoRemote)
        ));
        // With a remote it commits the work, pushes the branch and returns the URL.
        let remote = tempfile::tempdir().expect("remote");
        sh(remote.path(), &["init", "-q", "--bare"]);
        sh(
            repo.path(),
            &["remote", "add", "origin", &remote.path().to_string_lossy()],
        );
        let url = open_pr(&record, &gh).await.expect("pr");
        assert_eq!(url, "https://github.com/o/r/pull/7");
        assert!(!sh(remote.path(), &["branch", "--list", &record.branch]).is_empty());
        // The worktree is kept for further work.
        assert!(Path::new(&record.path).is_dir());
        assert!(sh(Path::new(&record.path), &["status", "--porcelain"]).is_empty());
    }

    #[test]
    fn first_url_finds_the_pull_request_link() {
        assert_eq!(
            first_url("Creating pull request\nhttps://github.com/o/r/pull/9\n").as_deref(),
            Some("https://github.com/o/r/pull/9")
        );
        assert_eq!(first_url("nothing here"), None);
    }

    #[test]
    fn records_reject_path_like_session_ids() {
        let root = tempfile::tempdir().expect("root");
        assert!(load_record(root.path(), "../etc").is_none());
        assert!(load_record(root.path(), "").is_none());
        assert!(!valid_session_id(".hidden"));
        assert!(valid_session_id("claude-123"));
    }
}
