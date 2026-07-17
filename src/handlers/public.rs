//! 公开分享端点（无需认证）。

use axum::extract::{Path as AxumPath, State};
use axum::response::Response;
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};

use crate::error::{AppError, AppResult};
use crate::handlers::files;
use crate::models::share::PublicShareResponse;
use crate::services::share_service;
use crate::state::AppState;

/// GET /public/shares/:token
pub async fn info(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    let (share, file, shared_by) = share_service::resolve_public(&state.db, &token).await?;
    let resp = PublicShareResponse {
        name: file.name,
        file_type: file.file_type,
        size_bytes: file.size_bytes,
        mime_type: file.mime_type,
        can_download: share.can_download != 0,
        shared_by,
    };
    Ok(Json(json!({ "data": resp })))
}

/// GET /public/shares/:token/download
pub async fn download(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
) -> AppResult<Response> {
    let (share, file, _shared_by) = share_service::resolve_public(&state.db, &token).await?;

    if share.can_download == 0 {
        return Err(AppError::Forbidden);
    }

    let response = files::stream_file(&state, &share.owner_id, &file).await?;
    share_service::record_access(&state.db, &share.id).await?;
    Ok(response)
}
