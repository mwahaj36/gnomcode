//! Read-only inspection of session artifacts (created/edited files, HTML, Markdown, images).

use crate::api::{AppState, validate_auth};
use crate::workspace_files::resolve_session_workspace;
use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_store::Database;
use serde::Serialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_ARTIFACTS: usize = 30;
const MAX_TEXT_ARTIFACT_BYTES: u64 = 256 * 1024;
const MAX_IMAGE_ARTIFACT_BYTES: u64 = 512 * 1024;
const MAX_GIT_BYTES: u64 = 1_048_576;
const GIT_TIMEOUT_SECS: u64 = 10;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct ArtifactItem {
    id: String,
    title: String,
    path: String,
    kind: &'static str,
    mime_type: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_url: Option<String>,
}

fn encode_base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn classify_path(rel_path: &str) -> (&'static str, &'static str) {
    let lower = rel_path.to_ascii_lowercase();
    if lower.ends_with(".html") || lower.ends_with(".htm") || lower.ends_with(".xhtml") {
        ("html", "text/html")
    } else if lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdx") {
        ("markdown", "text/markdown")
    } else if lower.ends_with(".png") {
        ("image", "image/png")
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        ("image", "image/jpeg")
    } else if lower.ends_with(".webp") {
        ("image", "image/webp")
    } else if lower.ends_with(".gif") {
        ("image", "image/gif")
    } else if lower.ends_with(".svg") {
        ("image", "image/svg+xml")
    } else {
        ("other", "text/plain")
    }
}

async fn git_porcelain_z(repo: &Path) -> Result<Vec<(String, &'static str)>, &'static str> {
    use tokio::io::AsyncReadExt as _;
    let mut command = tokio::process::Command::new("git");
    command
        .args([
            "--no-pager",
            "-c",
            "core.quotepath=off",
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "-z",
        ])
        .current_dir(repo)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("LC_ALL", "C")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    let mut child = command
        .spawn()
        .map_err(|_| "Could not inspect workspace changes")?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Could not inspect workspace changes")?;
    let bytes = tokio::time::timeout(std::time::Duration::from_secs(GIT_TIMEOUT_SECS), async {
        let mut buf = Vec::new();
        stdout
            .take(MAX_GIT_BYTES + 1)
            .read_to_end(&mut buf)
            .await
            .map_err(|_| "Could not inspect workspace changes")?;
        if buf.len() as u64 > MAX_GIT_BYTES {
            let _ = child.kill().await;
            return Err("Workspace status exceeds 1 MiB limit");
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "Could not inspect workspace changes")?;
        if !status.success() {
            return Err("Workspace is not a git repository");
        }
        Ok(buf)
    })
    .await
    .map_err(|_| "Git status timed out")??;

    let mut items = Vec::new();
    let mut entries = bytes.split(|&b| b == 0).peekable();
    while let Some(raw) = entries.next() {
        if raw.len() < 4 {
            continue;
        }
        let code = &raw[..2];
        if code.contains(&b'D') {
            continue;
        }
        let status_label = if code == b"??" || code.contains(&b'A') {
            "created"
        } else {
            "modified"
        };
        let path_bytes = if code[0] == b'R' || code[0] == b'C' {
            entries.next().unwrap_or(&raw[3..])
        } else {
            &raw[3..]
        };
        if let Ok(path) = std::str::from_utf8(path_bytes) {
            items.push((path.replace('\\', "/"), status_label));
        }
    }
    Ok(items)
}

async fn collect_artifacts(workspace: &Path) -> Result<(Vec<ArtifactItem>, usize), &'static str> {
    let canonical_root = workspace
        .canonicalize()
        .map_err(|_| "Workspace folder is unavailable")?;
    if !canonical_root.is_dir() {
        return Err("Workspace folder is unavailable");
    }
    let mut changed = git_porcelain_z(&canonical_root).await?;
    let changes_count = changed.len();

    // Prioritize rich artifact outputs (image, html, markdown) before plain files.
    changed.sort_by_key(|(path, _)| match classify_path(path).0 {
        "image" => 0,
        "html" => 1,
        "markdown" => 2,
        _ => 3,
    });

    let mut artifacts = Vec::new();
    for (rel_path, status_label) in changed.into_iter().take(MAX_ARTIFACTS) {
        let candidate = canonical_root.join(&rel_path);
        let Ok(canonical_file) = candidate.canonicalize() else {
            continue;
        };
        if !canonical_file.starts_with(&canonical_root) || !canonical_file.is_file() {
            continue;
        }
        let Ok(metadata) = std::fs::metadata(&canonical_file) else {
            continue;
        };
        let (kind, mime_type) = classify_path(&rel_path);
        let title = Path::new(&rel_path)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| rel_path.clone());

        if kind == "image" {
            if metadata.len() > MAX_IMAGE_ARTIFACT_BYTES {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&canonical_file) {
                let encoded = encode_base64(&bytes);
                artifacts.push(ArtifactItem {
                    id: rel_path.clone(),
                    title,
                    path: rel_path,
                    kind,
                    mime_type,
                    status: status_label,
                    content: None,
                    data_url: Some(format!("data:{mime_type};base64,{encoded}")),
                });
            }
        } else if metadata.len() <= MAX_TEXT_ARTIFACT_BYTES {
            if let Ok(content) = std::fs::read_to_string(&canonical_file) {
                artifacts.push(ArtifactItem {
                    id: rel_path.clone(),
                    title,
                    path: rel_path,
                    kind,
                    mime_type,
                    status: status_label,
                    content: Some(content),
                    data_url: None,
                });
            }
        }
    }
    Ok((artifacts, changes_count))
}

/// Returns read-only artifacts and change count for a session's workspace.
pub(crate) async fn get_handler(
    headers: HeaderMap,
    AxumPath(session_id): AxumPath<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let workspace = {
        let db = state.db.lock().await;
        resolve_session_workspace(&db, &session_id).map_err(|_| StatusCode::NOT_FOUND)?
    };
    let (artifacts, changes_count) = collect_artifacts(&workspace)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(Json(json!({
        "session_id": session_id,
        "artifacts": artifacts,
        "changes_count": changes_count,
    })))
}

/// Handles E2EE Remote `artifacts` requests for a session's workspace.
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing session"});
    };
    let workspace = {
        let db = db.lock().await;
        match resolve_session_workspace(&db, session_id) {
            Ok(path) => path,
            Err(message) => return json!({"type": "error", "message": message}),
        }
    };
    match collect_artifacts(&workspace).await {
        Ok((artifacts, changes_count)) => json!({
            "type": "artifacts",
            "session_id": session_id,
            "artifacts": artifacts,
            "changes_count": changes_count,
        }),
        Err(message) => json!({"type": "error", "message": message}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnomcode_core::{AgentId, Project, Session, SessionState, Surface};

    fn seeded_db_with_session(session_id: &str, repo: &Path) -> Arc<Mutex<Database>> {
        let mut db = Database::open_in_memory().expect("in-memory db");
        let project = Project::from_origin(None, &repo.to_string_lossy(), None);
        let now = chrono::Utc::now();
        let session = Session {
            id: session_id.to_owned(),
            agent: AgentId::Codex,
            surface: Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            pid: None,
            title: Some("Artifacts Test".to_owned()),
            model: None,
            state: SessionState::Idle,
            started_at: now,
            last_event_at: now,
        };
        let checkpoint = gnomcode_store::SourceCheckpoint {
            file_path: "fixture".into(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fixture".into(),
            last_ingested_at: now,
        };
        db.commit_batch(&[project], &[session], &[], &[], &[], &checkpoint)
            .expect("commit");
        db.record_session_cwd(session_id, &repo.to_string_lossy())
            .expect("cwd");
        Arc::new(Mutex::new(db))
    }

    #[tokio::test]
    async fn remote_artifacts_collects_html_markdown_and_image_outputs() {
        let repo = tempfile::tempdir().expect("repo");
        let git = |args: &[&str]| {
            assert!(
                std::process::Command::new("git")
                    .args(args)
                    .current_dir(repo.path())
                    .status()
                    .expect("git")
                    .success()
            );
        };
        git(&["init"]);
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test"]);
        std::fs::write(repo.path().join("notes.md"), "# Before\n").expect("md");
        git(&["add", "notes.md"]);
        git(&["commit", "-m", "init"]);

        std::fs::write(repo.path().join("notes.md"), "# After\n").expect("md edit");
        std::fs::write(repo.path().join("report.html"), "<h1>Hi</h1>").expect("html");
        std::fs::write(repo.path().join("chart.png"), [137u8, 80, 78, 71]).expect("png");

        let db = seeded_db_with_session("codex:artifacts-test", repo.path());
        let response = remote_command(
            &db,
            &json!({"type": "artifacts", "session_id": "codex:artifacts-test"}),
        )
        .await;
        assert_eq!(response["type"], "artifacts");
        assert_eq!(response["changes_count"], 3);
        let items = response["artifacts"].as_array().expect("artifacts");
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|item| {
            item["kind"] == "image"
                && item["data_url"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("data:image/png;base64,"))
        }));
        assert!(
            items
                .iter()
                .any(|item| item["kind"] == "html" && item["content"] == "<h1>Hi</h1>")
        );
        assert!(
            items
                .iter()
                .any(|item| item["kind"] == "markdown" && item["status"] == "modified")
        );
    }
}
