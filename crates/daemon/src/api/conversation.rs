//! Conversation listing and chat history handlers.

use super::*;

/// Query parameters for retrieving chat events.
#[derive(Debug, Deserialize)]
pub struct ChatQuery {
    /// Maximum number of events to return.
    pub limit: Option<usize>,
    /// Return events older than this event ID.
    pub before: Option<String>,
}

pub(super) async fn chat_get_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(query): Query<ChatQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let requested = id.clone();
    let id = crate::remote::conversation_session_id(
        &id,
        &sessions,
        &gnomcode_pty::list_active_sessions(),
    );
    match crate::remote::conversation_chat(
        &db,
        &requested,
        &id,
        &sessions,
        query.limit,
        query.before.as_deref(),
    ) {
        Ok(events) => {
            let mut cleaned = gnomcode_adapters::codex::clean_stored_conversation(
                gnomcode_adapters::antigravity::clean_stored_conversation(events),
            );
            rewrite_codex_image_refs(&db, &requested, &mut cleaned);
            Ok(Json(cleaned))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

const IMAGE_PLACEHOLDER: &str = "[Image attachment: preview unavailable in imported history]";
const MAX_IMPORTED_IMAGE_BYTES: u64 = 5 * 1024 * 1024;

fn rewrite_codex_image_refs(
    db: &gnomcode_store::Database,
    conversation_id: &str,
    events: &mut [gnomcode_core::ChatEvent],
) {
    let encoded_conv = urlencoding_segment(conversation_id);
    for event in events {
        if event.role != gnomcode_core::ChatRole::User
            || !event.content.contains(IMAGE_PLACEHOLDER)
            || !event.session_id.starts_with("codex:")
        {
            continue;
        }
        let Some(tags) = read_event_codex_image_tags(db, &event.session_id, event.turn_index)
        else {
            continue;
        };
        if tags.is_empty() {
            continue;
        }
        let encoded_evt = urlencoding_segment(&event.event_id);
        let mut idx = 0usize;
        while idx < tags.len() && event.content.contains(IMAGE_PLACEHOLDER) {
            let replacement = format!(
                "[Attachment: /api/v1/conversations/{encoded_conv}/images/{encoded_evt}/{idx}]"
            );
            event.content = event.content.replacen(IMAGE_PLACEHOLDER, &replacement, 1);
            idx += 1;
        }
    }
}

fn urlencoding_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for byte in raw.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn read_event_codex_image_tags(
    db: &gnomcode_store::Database,
    session_id: &str,
    turn_index: i64,
) -> Option<Vec<(String, String)>> {
    let raw_sid = session_id.strip_prefix("codex:")?;
    let rollout_path = db.find_codex_rollout_path(raw_sid).ok().flatten()?;
    let offset = u64::try_from(turn_index).ok()?;
    let mut file = gnomcode_platform::open_shared_read(std::path::Path::new(&rollout_path)).ok()?;
    use std::io::{BufRead, Seek, SeekFrom};
    file.seek(SeekFrom::Start(offset)).ok()?;
    let mut line = String::new();
    std::io::BufReader::new(file).read_line(&mut line).ok()?;
    let parsed: Value = serde_json::from_str(&line).ok()?;
    let content = parsed.get("payload")?.get("content")?.as_array()?;
    let mut tags = Vec::new();
    for part in content {
        if part.get("type").and_then(Value::as_str) == Some("input_text")
            && let Some(text) = part.get("text").and_then(Value::as_str)
        {
            let (_, found) = gnomcode_core::redact::strip_image_tags(text);
            tags.extend(found);
        }
    }
    Some(tags)
}

fn image_mime_for_ext(ext: &str) -> Option<&'static str> {
    match ext {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

fn is_allowed_imported_image_path(
    candidate: &std::path::Path,
    repo_root: Option<&str>,
) -> Option<std::path::PathBuf> {
    let canonical = std::fs::canonicalize(candidate).ok()?;
    let meta = std::fs::metadata(&canonical).ok()?;
    if !meta.is_file() || meta.len() > MAX_IMPORTED_IMAGE_BYTES {
        return None;
    }
    let mut roots = vec![gnomcode_platform::gnomcode_home(), std::env::temp_dir()];
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        roots.push(std::path::PathBuf::from(codex_home));
    } else if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"))
    {
        roots.push(std::path::PathBuf::from(home).join(".codex"));
    }
    if let Some(repo) = repo_root.filter(|r| !r.trim().is_empty()) {
        roots.push(std::path::PathBuf::from(repo));
    }
    let allowed = roots
        .into_iter()
        .filter_map(|root| std::fs::canonicalize(root).ok())
        .any(|root| canonical.starts_with(&root));
    allowed.then_some(canonical)
}

pub(super) async fn conversation_image_handler(
    headers: HeaderMap,
    Path((id, event_id, index)): Path<(String, String, usize)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let (session_id, turn_index, repo_root) = {
        let db = state.db.lock().await;
        let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
        let resolved_id = crate::remote::conversation_session_id(
            &id,
            &sessions,
            &gnomcode_pty::list_active_sessions(),
        );
        let repo_root = sessions
            .iter()
            .find(|s| s.id == resolved_id || s.id == id)
            .and_then(|s| {
                db.list_projects()
                    .ok()
                    .and_then(|ps| ps.into_iter().find(|p| p.id == s.project_id))
            })
            .map(|p| p.repo_root);
        let (sid, offset_str) = event_id.rsplit_once(':').ok_or(StatusCode::NOT_FOUND)?;
        let turn_index = offset_str
            .parse::<i64>()
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let tags =
            read_event_codex_image_tags(&db, sid, turn_index).ok_or(StatusCode::NOT_FOUND)?;
        let (label, raw_path) = tags.get(index).cloned().ok_or(StatusCode::NOT_FOUND)?;
        (label, raw_path, repo_root)
    };
    let label = session_id;
    let raw_path = turn_index;
    let canonical =
        is_allowed_imported_image_path(std::path::Path::new(&raw_path), repo_root.as_deref())
            .ok_or(StatusCode::NOT_FOUND)?;
    let ext = canonical
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mime = image_mime_for_ext(&ext).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let bytes = tokio::fs::read(&canonical)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let data_url = format!(
        "data:{mime};base64,{}",
        gnomcode_core::file_preview::encode_base64(&bytes)
    );
    Ok(Json(json!({
        "available": true,
        "kind": "image",
        "name": label,
        "mime_type": mime,
        "size": bytes.len(),
        "data_url": data_url,
    })))
}

pub(super) async fn chat_delete_handler(
    headers: HeaderMap,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let _control = crate::CONVERSATION_CONTROL.lock().await;
    let db = state.db.lock().await;
    let sessions = db.list_active_sessions(usize::MAX).unwrap_or_default();
    let hosted = gnomcode_pty::list_active_sessions();
    let native = crate::remote::active_conversation_id(&db, &id, &sessions, &hosted)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let host = sessions
        .iter()
        .find(|session| session.id == native)
        .and_then(|session| crate::remote::session_host(session, &sessions, &hosted))
        .map_or(id.as_str(), |record| record.session_id.as_str());
    match db.delete_session_chat_with_host(&native, host) {
        Ok(deleted_count) => Ok(Json(json!({
            "ok": true,
            "session_id": id,
            "deleted_count": deleted_count
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub(super) async fn list_conversations_handler(
    headers: HeaderMap,
    State(state): State<AppState>,
    Query(params): Query<SessionQueryParams>,
) -> Result<impl IntoResponse, StatusCode> {
    validate_auth(&headers, &state.bearer_token)?;
    let db = state.db.lock().await;
    let sessions = db
        .list_active_sessions(usize::MAX)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut rows =
        crate::remote::conversation_sessions(&db, &sessions, &gnomcode_pty::list_active_sessions())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    crate::remote::nest_subagents(&mut rows);
    let page = crate::remote::conversation_page(
        rows,
        &json!({"page":params.page,"limit":params.limit,"agent":params.agent,"status":params.status,"project":params.project,"search":params.search,"source":params.source,"include_subagents":params.include_subagents}),
    );
    if params.page.is_none() {
        Ok(Json(page["sessions"].clone()))
    } else {
        Ok(Json(page))
    }
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
    async fn chat_endpoints_retrieve_and_hard_delete_history() {
        let db = Database::open_in_memory().expect("database");
        let project = gnomcode_core::Project::from_origin(None, "C:\\repo", None);
        let session = gnomcode_core::Session {
            id: "chat-test-session".to_owned(),
            agent: gnomcode_core::AgentId::ClaudeCode,
            surface: gnomcode_core::Surface::Cli,
            project_id: project.id.clone(),
            parent_session_id: None,
            model: Some("claude-3-5-sonnet".to_owned()),
            title: None,
            state: gnomcode_core::SessionState::Idle,
            pid: Some(100),
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now(),
        };
        let cp = gnomcode_store::SourceCheckpoint {
            file_path: "/path/to/c.jsonl".to_owned(),
            last_read_offset: 0,
            file_size: 0,
            fingerprint: "fp".to_owned(),
            last_ingested_at: chrono::Utc::now(),
        };
        let mut db_mut = db;
        db_mut
            .commit_batch(&[project], &[session], &[], &[], &[], &cp)
            .expect("commit");

        let evt = gnomcode_core::ChatEvent {
            event_id: "evt-test-1".to_owned(),
            session_id: "chat-test-session".to_owned(),
            turn_index: 1,
            role: gnomcode_core::ChatRole::User,
            content: "Please inspect tests".to_owned(),
            tool_name: None,
            status: None,
            created_at: chrono::Utc::now(),
        };
        db_mut.insert_chat_event(&evt).expect("insert chat");

        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db_mut)),
            bearer_token: "chat-token".to_owned(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });

        let titles = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/sessions?range=all")
                    .header("Host", "127.0.0.1")
                    .header("Authorization", "Bearer chat-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let titles: serde_json::Value = serde_json::from_slice(
            &axum::body::to_bytes(titles.into_body(), 131072)
                .await
                .expect("body"),
        )
        .expect("json");
        assert_eq!(titles["sessions"][0]["title"], "Please inspect tests");

        // 1. GET chat events
        let req_get = Request::builder()
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_get = app.clone().oneshot(req_get).await.expect("res");
        assert_eq!(res_get.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res_get.into_body(), 16384)
            .await
            .expect("body");
        let events: Vec<gnomcode_core::ChatEvent> = serde_json::from_slice(&body).expect("json");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].content, "Please inspect tests");

        // 2. DELETE chat events (hard deletion)
        let req_del = Request::builder()
            .method("DELETE")
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_del = app.clone().oneshot(req_del).await.expect("res");
        assert_eq!(res_del.status(), StatusCode::OK);

        // 3. GET chat events after delete -> empty
        let req_get_after = Request::builder()
            .uri("/api/v1/sessions/chat-test-session/chat")
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer chat-token")
            .body(Body::empty())
            .expect("request");

        let res_get_after = app.oneshot(req_get_after).await.expect("res");
        assert_eq!(res_get_after.status(), StatusCode::OK);
        let body_after = axum::body::to_bytes(res_get_after.into_body(), 16384)
            .await
            .expect("body");
        let events_after: Vec<gnomcode_core::ChatEvent> =
            serde_json::from_slice(&body_after).expect("json");
        assert!(events_after.is_empty());
    }

    #[tokio::test]
    async fn codex_imported_image_serves_allowlisted_temp_file_and_rejects_missing_or_outside() {
        let dir = tempfile::tempdir().expect("tempdir");
        let img_path =
            std::env::temp_dir().join(format!("codex-clipboard-test-{}.png", std::process::id()));
        let png_bytes = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
        std::fs::write(&img_path, png_bytes).expect("write temp png");

        let rollout_path = dir.path().join("rollout-img-test-raw.jsonl");
        let line0 = serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-11-14T04:10:00Z",
            "payload": { "id": "img-test-raw", "cwd": dir.path().to_string_lossy() }
        });
        let line1 = serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-11-14T04:10:01Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{
                    "type": "input_text",
                    "text": format!("<image name=[Image #1] path=\"{}\">\n</image>\nInspect this", img_path.display())
                }]
            }
        });
        let line0_str = format!("{line0}\n");
        let offset = i64::try_from(line0_str.len()).expect("offset");
        std::fs::write(&rollout_path, format!("{line0_str}{line1}\n")).expect("write rollout");

        let mut db = Database::open_in_memory().expect("database");
        let project =
            gnomcode_core::Project::from_origin(None, &dir.path().to_string_lossy(), None);
        let session = gnomcode_core::Session {
            id: "codex:img-test-raw".into(),
            agent: gnomcode_core::AgentId::Codex,
            project_id: project.id.clone(),
            surface: gnomcode_core::Surface::Cli,
            title: None,
            model: None,
            parent_session_id: None,
            state: gnomcode_core::SessionState::Idle,
            pid: None,
            started_at: chrono::Utc::now(),
            last_event_at: chrono::Utc::now(),
        };
        let event = gnomcode_core::ChatEvent {
            event_id: format!("codex:img-test-raw:{offset}"),
            session_id: "codex:img-test-raw".into(),
            turn_index: offset,
            role: gnomcode_core::ChatRole::User,
            content: "Inspect this\n\n[Image attachment: preview unavailable in imported history]"
                .into(),
            tool_name: None,
            status: None,
            created_at: chrono::Utc::now(),
        };
        let cp = gnomcode_store::SourceCheckpoint {
            file_path: rollout_path.to_string_lossy().into_owned(),
            last_read_offset: 0,
            file_size: 10,
            fingerprint: "fp".into(),
            last_ingested_at: chrono::Utc::now(),
        };
        db.commit_batch_with_chat(&[project], &[session], &[], &[], &[], &cp, &[event])
            .expect("commit");

        let app = build_router(AppState {
            db: Arc::new(Mutex::new(db)),
            bearer_token: "img-token".into(),
            orchestrator: None,
            remote: Arc::new(crate::remote::RemoteManager::new()),
        });
        let req = Request::builder()
            .uri(format!(
                "/api/v1/conversations/codex:img-test-raw/images/codex:img-test-raw:{offset}/0"
            ))
            .header("Host", "127.0.0.1")
            .header("Authorization", "Bearer img-token")
            .body(Body::empty())
            .expect("request");
        let res = app.oneshot(req).await.expect("response");
        assert_eq!(res.status(), StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 16384)
            .await
            .expect("body");
        let payload: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(payload["available"], true);
        assert_eq!(payload["kind"], "image");
        assert!(
            payload["data_url"]
                .as_str()
                .is_some_and(|u| u.starts_with("data:image/png;base64,"))
        );
        let _ = std::fs::remove_file(&img_path);
    }
}
