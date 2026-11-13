//! Local React console bundle handler (`SPEC §3.2`, E1).

use axum::{http::header, response::IntoResponse};

/// Serves the compiled local console React bundle from `web/`.
pub(crate) async fn handler() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        include_str!("../console-react.js"),
    )
}
