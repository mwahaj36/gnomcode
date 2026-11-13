//! Bounded, allowlisted attachment uploads staged inside a session's workspace.

use crate::api::{AppState, validate_auth};
use crate::workspace_files::resolve_session_workspace;
use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use gnomcode_store::Database;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

const MAX_FILE_BYTES: usize = 15 * 1024 * 1024;
const MAX_SESSION_BYTES: u64 = 200 * 1024 * 1024;
const ALLOWED_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "txt", "md", "log", "json", "csv", "pdf",
];

/// Request body for local HTTP attachment upload.
#[derive(Debug, Deserialize)]
pub(crate) struct UploadAttachmentRequest {
    /// Original filename (sanitized to a safe basename on the daemon).
    pub filename: String,
    /// Optional MIME type reported by the client.
    #[serde(default)]
    pub mime_type: String,
    /// Declared byte size.
    #[serde(default)]
    pub size: Option<u64>,
    /// Base64-encoded file bytes.
    pub data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UploadError {
    BadRequest(&'static str),
    UnsupportedType,
    TooLarge(&'static str),
    Io,
}

impl UploadError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest(_) => StatusCode::BAD_REQUEST,
            Self::UnsupportedType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::TooLarge(_) => StatusCode::PAYLOAD_TOO_LARGE,
            Self::Io => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::BadRequest(msg) => msg,
            Self::UnsupportedType => {
                "Unsupported attachment type (allowed: png, jpg, jpeg, webp, gif, txt, md, log, json, csv, pdf)"
            }
            Self::TooLarge(msg) => msg,
            Self::Io => "Could not save attachment",
        }
    }
}

fn sanitize_filename(raw: &str) -> Result<String, UploadError> {
    let base = raw
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    if base.is_empty() || base == "." || base == ".." {
        return Err(UploadError::BadRequest("Invalid attachment filename"));
    }
    let mut clean = String::with_capacity(base.len().min(120));
    for ch in base.chars().take(120) {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            clean.push(ch);
        } else {
            clean.push('_');
        }
    }
    let ext = clean
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    if ext.is_empty() || !ALLOWED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(UploadError::UnsupportedType);
    }
    Ok(clean)
}

fn decode_base64(input: &str) -> Result<Vec<u8>, UploadError> {
    let stripped = input
        .trim()
        .split_once(";base64,")
        .map_or(input.trim(), |(_, rest)| rest);
    if stripped.is_empty() {
        return Err(UploadError::BadRequest("Attachment is empty"));
    }
    // Quick upper-bound check before allocating.
    if stripped.len() > MAX_FILE_BYTES.div_ceil(3) * 4 + 64 {
        return Err(UploadError::TooLarge(
            "Attachment exceeds 15 MiB per-file limit",
        ));
    }
    let mut out = Vec::with_capacity((stripped.len() / 4) * 3);
    let mut buf = [0u8; 4];
    let mut count = 0usize;
    let mut pad = 0usize;
    for byte in stripped.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => {
                pad += 1;
                0
            }
            _ => return Err(UploadError::BadRequest("Invalid base64 attachment payload")),
        };
        if pad > 0 && byte != b'=' {
            return Err(UploadError::BadRequest("Invalid base64 attachment payload"));
        }
        buf[count] = val;
        count += 1;
        if count == 4 {
            out.push((buf[0] << 2) | (buf[1] >> 4));
            if pad < 2 {
                out.push((buf[1] << 4) | (buf[2] >> 2));
            }
            if pad == 0 {
                out.push((buf[2] << 6) | buf[3]);
            }
            if out.len() > MAX_FILE_BYTES {
                return Err(UploadError::TooLarge(
                    "Attachment exceeds 15 MiB per-file limit",
                ));
            }
            count = 0;
            pad = 0;
        }
    }
    if count != 0 || out.is_empty() {
        return Err(UploadError::BadRequest("Invalid base64 attachment payload"));
    }
    Ok(out)
}

fn dir_used_bytes(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.metadata().ok())
        .filter(|meta| meta.is_file())
        .map(|meta| meta.len())
        .sum()
}

fn random_prefix() -> Result<String, UploadError> {
    let mut bytes = [0u8; 6];
    getrandom::fill(&mut bytes).map_err(|_| UploadError::Io)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn stage_attachment(
    workspace: &Path,
    filename: &str,
    declared_size: Option<u64>,
    data_base64: &str,
) -> Result<(String, String, u64), UploadError> {
    let clean_name = sanitize_filename(filename)?;
    let bytes = decode_base64(data_base64)?;
    let actual_size = bytes.len() as u64;
    if let Some(expected) = declared_size {
        if expected != actual_size {
            return Err(UploadError::BadRequest("Attachment size mismatch"));
        }
    }

    let canonical_root = workspace
        .canonicalize()
        .map_err(|_| UploadError::BadRequest("Workspace folder is unavailable"))?;
    let attach_dir = canonical_root.join(".gnomcode").join("attachments");
    std::fs::create_dir_all(&attach_dir).map_err(|_| UploadError::Io)?;
    let canonical_dir = attach_dir.canonicalize().map_err(|_| UploadError::Io)?;
    if !canonical_dir.starts_with(&canonical_root) {
        return Err(UploadError::BadRequest(
            "Attachment directory escapes workspace",
        ));
    }

    let used = dir_used_bytes(&canonical_dir);
    if used.saturating_add(actual_size) > MAX_SESSION_BYTES {
        return Err(UploadError::TooLarge(
            "Session attachments exceed 200 MiB limit",
        ));
    }

    let stored_name = format!("{}-{clean_name}", random_prefix()?);
    let target = canonical_dir.join(&stored_name);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|_| UploadError::Io)?;
    std::io::Write::write_all(&mut file, &bytes).map_err(|_| UploadError::Io)?;
    let rel_path = format!(".gnomcode/attachments/{stored_name}");
    Ok((clean_name, rel_path, actual_size))
}

/// Stages an uploaded attachment inside a session's workspace.
pub(crate) async fn post_handler(
    headers: HeaderMap,
    AxumPath(session_id): AxumPath<String>,
    State(state): State<AppState>,
    Json(payload): Json<UploadAttachmentRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let workspace = {
        let db = state.db.lock().await;
        resolve_session_workspace(&db, &session_id).map_err(|_| StatusCode::NOT_FOUND)?
    };
    let (clean_name, rel_path, size) = tokio::task::spawn_blocking(move || {
        stage_attachment(
            &workspace,
            &payload.filename,
            payload.size,
            &payload.data_base64,
        )
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|err| err.status_code())?;

    Ok(Json(json!({
        "session_id": session_id,
        "filename": clean_name,
        "mime_type": payload.mime_type,
        "size": size,
        "workspace_path": rel_path,
    })))
}

/// Handles E2EE Remote `upload_attachment` requests.
pub(crate) async fn remote_command(db: &Arc<Mutex<Database>>, request: &Value) -> Value {
    let Some(session_id) = request.get("session_id").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing session"});
    };
    let request_id = request
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let Some(filename) = request.get("filename").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing filename", "request_id": request_id});
    };
    let Some(data_base64) = request.get("data_base64").and_then(Value::as_str) else {
        return json!({"type": "error", "message": "Missing attachment payload", "request_id": request_id});
    };
    let mime_type = request
        .get("mime_type")
        .and_then(Value::as_str)
        .unwrap_or("application/octet-stream")
        .to_owned();
    let declared_size = request.get("size").and_then(Value::as_u64);

    let workspace = {
        let db = db.lock().await;
        match resolve_session_workspace(&db, session_id) {
            Ok(path) => path,
            Err(message) => {
                return json!({"type": "error", "message": message, "request_id": request_id});
            }
        }
    };
    let filename_owned = filename.to_owned();
    let data_owned = data_base64.to_owned();
    match tokio::task::spawn_blocking(move || {
        stage_attachment(&workspace, &filename_owned, declared_size, &data_owned)
    })
    .await
    {
        Ok(Ok((clean_name, rel_path, size))) => json!({
            "type": "attachment_ack",
            "session_id": session_id,
            "request_id": request_id,
            "filename": clean_name,
            "mime_type": mime_type,
            "size": size,
            "workspace_path": rel_path,
        }),
        Ok(Err(err)) => json!({
            "type": "error",
            "message": err.message(),
            "request_id": request_id,
        }),
        Err(_) => json!({
            "type": "error",
            "message": "Could not save attachment",
            "request_id": request_id,
        }),
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
            title: Some("Attachment Test".to_owned()),
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
    async fn remote_upload_attachment_saves_sanitized_file_and_enforces_limits() {
        let repo = tempfile::tempdir().expect("repo");
        let db = seeded_db_with_session("codex:attach-test", repo.path());

        let ack = remote_command(
            &db,
            &json!({
                "type": "upload_attachment",
                "session_id": "codex:attach-test",
                "request_id": "req-1",
                "filename": "../../evil path/note.txt",
                "mime_type": "text/plain",
                "size": 5,
                "data_base64": "aGVsbG8="
            }),
        )
        .await;
        assert_eq!(ack["type"], "attachment_ack");
        assert_eq!(ack["request_id"], "req-1");
        assert_eq!(ack["filename"], "note.txt");
        let rel = ack["workspace_path"].as_str().expect("workspace_path");
        assert!(rel.starts_with(".gnomcode/attachments/"));
        assert_eq!(
            std::fs::read_to_string(repo.path().join(rel)).expect("saved"),
            "hello"
        );

        let rejected_ext = remote_command(
            &db,
            &json!({
                "type": "upload_attachment",
                "session_id": "codex:attach-test",
                "request_id": "req-2",
                "filename": "payload.exe",
                "mime_type": "application/octet-stream",
                "size": 5,
                "data_base64": "aGVsbG8="
            }),
        )
        .await;
        assert_eq!(rejected_ext["type"], "error");
        assert_eq!(rejected_ext["request_id"], "req-2");
    }
}
