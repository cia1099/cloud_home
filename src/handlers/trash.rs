//! 回收站端点。

use axum::extract::{Path as AxumPath, State};
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::models::file::FileResponse;
use crate::services::trash_service;
use crate::state::AppState;

/// GET /trash
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<JsonResp<Value>> {
    let items = trash_service::list(&state.db, &user.user_id).await?;
    Ok(Json(json!({ "data": { "items": items } })))
}

/// POST /trash/:id/restore
pub async fn restore(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    let entry = trash_service::restore(&state.db, &user.user_id, &id).await?;
    Ok(Json(json!({ "data": FileResponse::from(entry) })))
}

/// DELETE /trash/:id  立即永久删除单个文件
pub async fn delete_one(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    let data_root = state.drive_manager.require_data_root().await?;
    let bytes = trash_service::delete_permanent(&state.db, &data_root, &user.user_id, &id).await?;
    Ok(Json(json!({ "data": { "freed_bytes": bytes } })))
}

/// DELETE /trash  清空回收站
pub async fn empty(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<JsonResp<Value>> {
    let data_root = state.drive_manager.require_data_root().await?;
    let (count, bytes) = trash_service::empty(&state.db, &data_root, &user.user_id).await?;
    Ok(Json(json!({ "data": { "deleted_count": count, "freed_bytes": bytes } })))
}
