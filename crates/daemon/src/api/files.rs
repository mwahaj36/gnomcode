//! Read-only repository file and diff handlers.

use super::*;

/// Query parameters for reading a repository file.
#[derive(Debug, Deserialize)]
pub struct RepoFileQuery {
    /// Absolute or relative path to the repo root.
    pub repo: String,
    /// Relative path to the file inside the repo.
    pub path: String,
}

pub(super) async fn repo_file_get_handler(
    headers: HeaderMap,
    Query(query): Query<RepoFileQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if query.repo.trim().is_empty() || query.path.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Same confinement and preview policy as Remote: no traversal, no `.git`, no symlink escape.
    let root = std::path::PathBuf::from(&query.repo);
    let path = query.path.clone();
    let preview = tokio::task::spawn_blocking(move || {
        crate::workspace_files::preview_workspace_file(&root, &path)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|err| err.status_code())?;
    Ok(Json(preview))
}

/// Query parameters for inspecting git workspace diffs.
#[derive(Debug, Deserialize)]
pub struct RepoDiffQuery {
    /// Path to the repository root.
    pub repo: String,
}

pub(super) async fn repo_diff_get_handler(
    headers: HeaderMap,
    Query(query): Query<RepoDiffQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let repo_path = std::path::Path::new(&query.repo);
    let Ok(canonical_repo) = repo_path.canonicalize() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    if !canonical_repo.is_dir() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let (diff_str, diff_lossy) =
        git_output(&canonical_repo, &["diff", "--no-ext-diff", "HEAD"]).await?;
    let (status_str, status_lossy) =
        git_output(&canonical_repo, &["status", "--porcelain=v1", "-z"]).await?;
    // shortcut: -z entries become lines, so names containing newlines split; fine for a viewer.
    let status_str = status_str.replace('\0', "\n");

    Ok(Json(json!({
        "repo": query.repo,
        "diff": diff_str,
        "status": status_str,
        "lossy": diff_lossy || status_lossy
    })))
}

/// Runs a read-only git command; returns stdout decoded lossily and whether bytes were replaced.
pub(super) async fn git_output(
    repo: &std::path::Path,
    args: &[&str],
) -> Result<(String, bool), StatusCode> {
    use tokio::io::AsyncReadExt as _;
    const LIMIT: u64 = 1_048_576;
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "--no-pager",
            "-c",
            "safe.directory=*",
            "-c",
            "core.quotepath=off",
            "-c",
            "core.fsmonitor=false",
        ])
        .args(args)
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .spawn()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let stdout = child
        .stdout
        .take()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let mut bytes = Vec::new();
        stdout
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if bytes.len() as u64 > LIMIT {
            let _ = child.kill().await;
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        let status = child
            .wait()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !status.success() {
            return Err(StatusCode::BAD_REQUEST);
        }
        Ok(match String::from_utf8(bytes) {
            Ok(text) => (text, false),
            Err(error) => (String::from_utf8_lossy(error.as_bytes()).into_owned(), true),
        })
    })
    .await
    .map_err(|_| StatusCode::GATEWAY_TIMEOUT)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use gnomcode_store::Database;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tower::ServiceExt as _;

    #[tokio::test]
    async fn git_output_reports_changes_and_untracked_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git")
        };
        assert!(git(&["init", "-q"]).status.success());
        std::fs::write(dir.path().join("tracked"), "before\n").expect("file");
        assert!(git(&["add", "tracked"]).status.success());
        assert!(
            git(&[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "initial"
            ])
            .status
            .success()
        );
        std::fs::write(dir.path().join("tracked"), "after\n").expect("file");
        std::fs::write(dir.path().join("untracked"), "new\n").expect("file");
        let (diff, _) = super::git_output(dir.path(), &["diff", "--no-ext-diff", "HEAD"])
            .await
            .expect("diff");
        let (status, _) = super::git_output(dir.path(), &["status", "--porcelain"])
            .await
            .expect("status");
        assert!(diff.contains("+after"));
        assert!(status.contains("?? untracked"));
    }

    #[tokio::test]
    async fn git_output_handles_non_ascii_names_and_non_utf8_diffs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
        };
        if !git(&["init", "-q"]).is_ok_and(|o| o.status.success()) {
            return; // git missing
        }
        std::fs::write(dir.path().join("caf\u{e9}.txt"), b"ok\n").expect("file");
        std::fs::write(dir.path().join("bin.txt"), b"a\n").expect("file");
        assert!(git(&["add", "-A"]).expect("git").status.success());
        assert!(
            git(&[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "initial"
            ])
            .expect("git")
            .status
            .success()
        );
        std::fs::write(dir.path().join("bin.txt"), b"b\xff\xfe\n").expect("file");
        std::fs::write(dir.path().join("na\u{ef}ve.txt"), b"new\n").expect("file");
        let (diff, lossy) = super::git_output(dir.path(), &["diff", "--no-ext-diff", "HEAD"])
            .await
            .expect("lossy diff must not fail");
        assert!(lossy && diff.contains('\u{fffd}'));
        let (status, _) = super::git_output(dir.path(), &["status", "--porcelain=v1", "-z"])
            .await
            .expect("status");
        assert!(
            status.contains("na\u{ef}ve.txt"),
            "unquoted name: {status:?}"
        );
    }

    #[tokio::test]
    async fn repository_diff_rejects_oversized_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = |args: &[&str]| {
            std::process::Command::new("git")
                .args(args)
                .current_dir(dir.path())
                .output()
                .expect("git")
        };
        assert!(git(&["init", "-q"]).status.success());
        std::fs::write(dir.path().join("file"), "original\n").expect("file");
        assert!(git(&["add", "file"]).status.success());
        assert!(
            git(&[
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-qm",
                "initial"
            ])
            .status
            .success()
        );
        std::fs::write(dir.path().join("file"), "a".repeat(1_100_000)).expect("file");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let mut uri = reqwest::Url::parse("http://127.0.0.1:41738/api/v1/repos/diff").expect("url");
        uri.query_pairs_mut()
            .append_pair("repo", &dir.path().to_string_lossy());
        let request = Request::builder()
            .uri(uri.as_str())
            .header("Host", "127.0.0.1:41738")
            .header("Authorization", "Bearer local-test-token")
            .body(Body::empty())
            .expect("request");
        assert_eq!(
            app.oneshot(request).await.expect("response").status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn repository_diff_rejects_a_directory_that_is_not_a_git_repository() {
        let dir = tempfile::tempdir().expect("tempdir");
        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "local-test-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let mut uri = reqwest::Url::parse("http://127.0.0.1:41738/api/v1/repos/diff").expect("url");
        uri.query_pairs_mut()
            .append_pair("repo", &dir.path().to_string_lossy());
        let request = Request::builder()
            .uri(uri.as_str())
            .header("Host", "127.0.0.1:41738")
            .header("Authorization", "Bearer local-test-token")
            .body(Body::empty())
            .expect("request");
        assert_ne!(
            app.oneshot(request).await.expect("response").status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn repo_file_endpoint_rejects_path_traversal() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let inside_file = temp_dir.path().join("safe.txt");
        std::fs::write(&inside_file, "clean content").expect("write safe");

        let outside_dir = tempfile::tempdir().expect("outside");
        let outside_file = outside_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "secret credentials").expect("write secret");

        let app = build_router(AppState {
            db: Arc::new(Mutex::new(Database::open_in_memory().expect("database"))),
            bearer_token: "file-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let repo_param = temp_dir.path().to_string_lossy().replace('\\', "/");

        // 1. Path traversal attempt with .. -> 400 Bad Request
        let bad_uri = format!("/api/v1/repos/file?repo={}&path=../secret.txt", repo_param);
        let req_bad = Request::builder()
            .uri(&bad_uri)
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer file-token")
            .body(Body::empty())
            .expect("request");
        let res_bad = app.clone().oneshot(req_bad).await.expect("res");
        assert_eq!(res_bad.status(), StatusCode::BAD_REQUEST);

        // 2. Safe file read -> 200 OK
        let safe_uri = format!("/api/v1/repos/file?repo={}&path=safe.txt", repo_param);
        let req_safe = Request::builder()
            .uri(&safe_uri)
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer file-token")
            .body(Body::empty())
            .expect("request");
        let res_safe = app.oneshot(req_safe).await.expect("res");
        assert_eq!(res_safe.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res_safe.into_body(), 16384)
            .await
            .expect("body");
        let val: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(val["content"], "clean content");
    }
}
