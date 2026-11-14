//! Read-only workspace file listing and reading confined to a session's workspace.

use crate::api::{AppState, validate_auth};
use axum::Json;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_store::Database;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_LIST_ENTRIES: usize = 200;

/// Query parameters for listing a directory inside a workspace.
#[derive(Debug, Deserialize)]
pub(crate) struct RepoFilesQuery {
    /// Workspace root path.
    pub repo: String,
    /// Optional relative subdirectory path inside the workspace.
    #[serde(default)]
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct WorkspaceEntry {
    name: String,
    path: String,
    kind: &'static str,
    size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct WorkspaceListing {
    path: String,
    entries: Vec<WorkspaceEntry>,
    truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileAccessError {
    BadPath(&'static str),
    NotFound,
    Io,
}

impl FileAccessError {
    pub(crate) fn status_code(&self) -> StatusCode {
        match self {
            Self::BadPath(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Io => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::BadPath(msg) => msg,
            Self::NotFound => "Path not found in workspace",
            Self::Io => "Could not read workspace path",
        }
    }
}

fn validate_relative_path(raw: &str, allow_empty: bool) -> Result<PathBuf, FileAccessError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." {
        return if allow_empty {
            Ok(PathBuf::new())
        } else {
            Err(FileAccessError::BadPath("File path is required"))
        };
    }
    if trimmed.len() > 1024 || trimmed.chars().any(char::is_control) {
        return Err(FileAccessError::BadPath("Invalid workspace path"));
    }
    let normalized = trimmed.replace('\\', "/");
    let mut clean = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Normal(part) => {
                let name = part.to_string_lossy();
                if name.eq_ignore_ascii_case(".git") || (cfg!(windows) && name.contains(':')) {
                    return Err(FileAccessError::BadPath("Invalid workspace path"));
                }
                clean.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FileAccessError::BadPath(
                    "Path must stay within the session workspace",
                ));
            }
        }
    }
    if clean.as_os_str().is_empty() && !allow_empty {
        return Err(FileAccessError::BadPath("File path is required"));
    }
    Ok(clean)
}

fn canonical_confined_path(root: &Path, rel: &Path) -> Result<(PathBuf, PathBuf), FileAccessError> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| FileAccessError::BadPath("Workspace folder is unavailable"))?;
    if !canonical_root.is_dir() {
        return Err(FileAccessError::BadPath("Workspace folder is unavailable"));
    }
    let target = if rel.as_os_str().is_empty() {
        canonical_root.clone()
    } else {
        canonical_root.join(rel)
    };
    let canonical_target = target
        .canonicalize()
        .map_err(|_| FileAccessError::NotFound)?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(FileAccessError::BadPath(
            "Path escapes the session workspace",
        ));
    }
    Ok((canonical_root, canonical_target))
}

fn list_workspace_dir(root: &Path, rel_raw: &str) -> Result<WorkspaceListing, FileAccessError> {
    let rel = validate_relative_path(rel_raw, true)?;
    let (canonical_root, canonical_dir) = canonical_confined_path(root, &rel)?;
    if !canonical_dir.is_dir() {
        return Err(FileAccessError::BadPath("Target is not a directory"));
    }
    let read_dir = std::fs::read_dir(&canonical_dir).map_err(|_| FileAccessError::Io)?;
    let mut entries = Vec::new();
    for item in read_dir.flatten() {
        let name = item.file_name().to_string_lossy().into_owned();
        if name == ".git" {
            continue;
        }
        let Ok(canonical_entry) = item.path().canonicalize() else {
            continue;
        };
        if !canonical_entry.starts_with(&canonical_root) {
            continue;
        }
        let Ok(metadata) = std::fs::metadata(&canonical_entry) else {
            continue;
        };
        let is_dir = metadata.is_dir();
        if !is_dir && !metadata.is_file() {
            continue;
        }
        let entry_rel = if rel.as_os_str().is_empty() {
            name.clone()
        } else {
            format!("{}/{}", rel.to_string_lossy().replace('\\', "/"), name)
        };
        entries.push(WorkspaceEntry {
            name,
            path: entry_rel,
            kind: if is_dir { "dir" } else { "file" },
            size: (!is_dir).then_some(metadata.len()),
        });
    }
    entries.sort_by(|a, b| {
        let a_dir = a.kind == "dir";
        let b_dir = b.kind == "dir";
        b_dir.cmp(&a_dir).then_with(|| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        })
    });
    let truncated = entries.len() > MAX_LIST_ENTRIES;
    entries.truncate(MAX_LIST_ENTRIES);
    Ok(WorkspaceListing {
        path: rel.to_string_lossy().replace('\\', "/"),
        entries,
        truncated,
    })
}

/// Reads a confined workspace file for preview using the `core::file_preview` policy.
///
/// Text is capped and flagged `truncated`; images, PDFs and other binaries are inlined as a
/// `data_url` only up to the inline cap, otherwise only name and size are returned.
pub(crate) fn preview_workspace_file(root: &Path, rel_raw: &str) -> Result<Value, FileAccessError> {
    use gnomcode_core::file_preview::{PreviewKind, classify, encode_base64, read_budget};
    let rel = validate_relative_path(rel_raw, false)?;
    let (_, canonical_file) = canonical_confined_path(root, &rel)?;
    if !canonical_file.is_file() {
        return Err(FileAccessError::BadPath("Target is not a file"));
    }
    let size = std::fs::metadata(&canonical_file)
        .map_err(|_| FileAccessError::Io)?
        .len();
    let rel_display = rel.to_string_lossy().replace('\\', "/");
    let name = rel
        .file_name()
        .map_or_else(|| rel_display.clone(), |n| n.to_string_lossy().into_owned());
    let read = |limit: u64| -> Result<Vec<u8>, FileAccessError> {
        let mut bytes = Vec::new();
        std::fs::File::open(&canonical_file)
            .and_then(|file| {
                std::io::Read::read_to_end(&mut std::io::Read::take(file, limit), &mut bytes)
            })
            .map_err(|_| FileAccessError::Io)?;
        Ok(bytes)
    };
    let head = read(8 * 1024)?;
    let policy = classify(&name, &head);
    let (budget, truncated) = read_budget(&policy, size);
    let mut out = json!({
        "path": rel_display,
        "name": name,
        "size": size,
        "kind": policy.kind,
        "mime_type": policy.mime_type,
        "language": policy.language,
        "truncated": truncated,
    });
    if policy.is_inline_binary() {
        if budget > 0 || size == 0 {
            let bytes = read(budget)?;
            out["data_url"] = json!(format!(
                "data:{};base64,{}",
                policy.mime_type,
                encode_base64(&bytes)
            ));
        }
    } else {
        let mut bytes = read(budget)?;
        if truncated {
            // Cut back to a character boundary so the preview stays valid UTF-8.
            while !bytes.is_empty()
                && std::str::from_utf8(&bytes).is_err_and(|e| e.error_len().is_none())
            {
                bytes.pop();
            }
        }
        let lossy = std::str::from_utf8(&bytes).is_err();
        out["content"] = json!(String::from_utf8_lossy(&bytes));
        out["lossy"] = json!(lossy || policy.kind == PreviewKind::Binary);
    }
    Ok(out)
}

pub(crate) fn resolve_session_workspace(
    db: &Database,
    session_id: &str,
) -> Result<PathBuf, &'static str> {
    if session_id.trim().is_empty() || session_id.len() > 128 {
        return Err("Invalid session");
    }
    let hosted = gnomcode_pty::list_active_sessions();
    if let Some(record) = hosted.iter().find(|r| r.session_id == session_id) {
        let path = PathBuf::from(&record.repo_path);
        if path.is_dir() {
            return Ok(path);
        }
    }
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let native_id = crate::remote::conversation_session_id(session_id, &sessions, &hosted);
    if let Some(record) = hosted.iter().find(|r| {
        sessions
            .iter()
            .find(|s| s.id == native_id)
            .is_some_and(|s| Some(r.child_pid) == s.pid || r.session_id == s.id)
    }) {
        let path = PathBuf::from(&record.repo_path);
        if path.is_dir() {
            return Ok(path);
        }
    }
    let target = db
        .get_session_focus_target(&native_id)
        .map_err(|_| "Session workspace is unavailable")?
        .ok_or("Session not found")?;
    let folder = target
        .repo_root
        .filter(|path| !path.trim().is_empty())
        .ok_or("Session has no workspace folder")?;
    let path = PathBuf::from(folder);
    if path.is_dir() {
        Ok(path)
    } else {
        Err("Session workspace folder is unavailable")
    }
}

/// Lists directory entries inside a workspace, confined to the workspace root.
pub(crate) async fn list_handler(
    headers: HeaderMap,
    Query(query): Query<RepoFilesQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    if query.repo.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let root = PathBuf::from(&query.repo);
    let path = query.path.clone();
    let listing = tokio::task::spawn_blocking(move || list_workspace_dir(&root, &path))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map_err(|err| err.status_code())?;
    Ok(Json(json!({
        "repo": query.repo,
        "path": listing.path,
        "entries": listing.entries,
        "truncated": listing.truncated,
    })))
}

/// Handles E2EE Remote `repo_files` and `repo_file` requests confined to the session workspace.
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let request_type = request.get("type").and_then(Value::as_str).unwrap_or("");
    let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing session"});
    };
    let rel_path = request
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let workspace = {
        let db = db.lock().await;
        match resolve_session_workspace(&db, session_id) {
            Ok(path) => path,
            Err(message) => return json!({"type": "error", "message": message}),
        }
    };
    let session_owned = session_id.to_owned();
    match request_type {
        "repo_files" => {
            match tokio::task::spawn_blocking(move || list_workspace_dir(&workspace, &rel_path))
                .await
            {
                Ok(Ok(listing)) => json!({
                    "type": "repo_files",
                    "session_id": session_owned,
                    "path": listing.path,
                    "entries": listing.entries,
                    "truncated": listing.truncated,
                }),
                Ok(Err(err)) => json!({"type": "error", "message": err.message()}),
                Err(_) => json!({"type": "error", "message": "Could not list workspace files"}),
            }
        }
        "repo_file" => {
            match tokio::task::spawn_blocking(move || preview_workspace_file(&workspace, &rel_path))
                .await
            {
                Ok(Ok(mut preview)) => {
                    preview["type"] = json!("repo_file");
                    preview["session_id"] = json!(session_owned);
                    preview
                }
                Ok(Err(err)) => json!({"type": "error", "message": err.message()}),
                Err(_) => json!({"type": "error", "message": "Could not read workspace file"}),
            }
        }
        _ => json!({"type": "error", "message": "Unsupported workspace file command"}),
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
            title: Some("Test".to_owned()),
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
    async fn remote_repo_files_and_file_stay_confined_and_enforce_size_limit() {
        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::create_dir_all(root.path().join("src")).expect("src");
        std::fs::write(root.path().join("src").join("lib.rs"), "pub fn ok() {}\n").expect("lib");
        std::fs::write(outside.path().join("secret.txt"), "top-secret").expect("secret");
        let huge = vec![b'a'; (gnomcode_core::file_preview::TEXT_PREVIEW_CAP as usize) + 1];
        std::fs::write(root.path().join("huge.txt"), huge).expect("huge");

        let db = seeded_db_with_session("codex:files-test", root.path());

        let list = remote_command(
            &db,
            &json!({"type": "repo_files", "session_id": "codex:files-test", "path": ""}),
        )
        .await;
        assert_eq!(list["type"], "repo_files");
        let entries = list["entries"].as_array().expect("entries");
        assert!(
            entries
                .iter()
                .any(|e| e["name"] == "src" && e["kind"] == "dir")
        );

        let read_ok = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "src/lib.rs"}),
        )
        .await;
        assert_eq!(read_ok["type"], "repo_file");
        assert_eq!(read_ok["content"], "pub fn ok() {}\n");

        let traversal = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "../secret.txt"}),
        )
        .await;
        assert_eq!(traversal["type"], "error");

        std::fs::create_dir_all(root.path().join(".git")).expect("git dir");
        std::fs::write(root.path().join(".git").join("config"), "token").expect("git config");
        let git = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": ".git/config"}),
        )
        .await;
        assert_eq!(git["type"], "error", "git metadata must stay unreadable");

        let too_big = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "huge.txt"}),
        )
        .await;
        assert_eq!(too_big["type"], "repo_file");
        assert_eq!(too_big["truncated"], true);
        assert_eq!(
            too_big["content"].as_str().map(str::len),
            Some(gnomcode_core::file_preview::TEXT_PREVIEW_CAP as usize)
        );

        std::fs::write(root.path().join("pixel.png"), [0x89, b'P', b'N', b'G']).expect("png");
        let image = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "pixel.png"}),
        )
        .await;
        assert_eq!(image["kind"], "image");
        assert_eq!(image["data_url"], "data:image/png;base64,iVBORw==");
        assert!(image.get("content").is_none());

        let big_image = vec![0_u8; (gnomcode_core::file_preview::INLINE_BYTES_CAP as usize) + 1];
        std::fs::write(root.path().join("big.png"), big_image).expect("big png");
        let big = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "big.png"}),
        )
        .await;
        assert_eq!(big["truncated"], true);
        assert!(
            big.get("data_url").is_none(),
            "over-cap bytes are never sent"
        );

        std::fs::write(root.path().join("blob.bin"), [1_u8, 0, 2, 3]).expect("bin");
        let binary = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "blob.bin"}),
        )
        .await;
        assert_eq!(binary["kind"], "binary");
        assert_eq!(binary["size"], 4);
        assert!(binary.get("content").is_none());

        std::fs::write(
            root.path().join("evil.svg"),
            "<svg><script>x</script></svg>",
        )
        .expect("svg");
        let svg = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:files-test", "path": "evil.svg"}),
        )
        .await;
        assert_eq!(svg["kind"], "svg");
        assert!(
            svg.get("content").is_none(),
            "SVG is only sent as image data"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remote_repo_files_rejects_symlinks_escaping_workspace() {
        let root = tempfile::tempdir().expect("workspace");
        let outside = tempfile::tempdir().expect("outside");
        std::fs::write(outside.path().join("secret.txt"), "secret").expect("secret");
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            root.path().join("leak.txt"),
        )
        .expect("symlink");
        let db = seeded_db_with_session("codex:sym-test", root.path());
        let list = remote_command(
            &db,
            &json!({"type": "repo_files", "session_id": "codex:sym-test", "path": ""}),
        )
        .await;
        let entries = list["entries"].as_array().expect("entries");
        assert!(!entries.iter().any(|e| e["name"] == "leak.txt"));
        let read = remote_command(
            &db,
            &json!({"type": "repo_file", "session_id": "codex:sym-test", "path": "leak.txt"}),
        )
        .await;
        assert_eq!(read["type"], "error");
    }
}
