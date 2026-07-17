//! 分享链接端点（需认证）。

use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::models::share::{CreateShareDto, ShareResponse};
use crate::services::share_service;
use crate::state::AppState;

/// POST /shares
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<CreateShareDto>,
) -> AppResult<(StatusCode, JsonResp<Value>)> {
    let link = share_service::create(&state.db, &user.user_id, &dto).await?;
    let resp = ShareResponse::from_link(link, &state.config.public_base_url);
    Ok((StatusCode::CREATED, Json(json!({ "data": resp }))))
}

/// GET /shares
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<JsonResp<Value>> {
    let links = share_service::list(&state.db, &user.user_id).await?;
    let items: Vec<ShareResponse> = links
        .into_iter()
        .map(|l| ShareResponse::from_link(l, &state.config.public_base_url))
        .collect();
    Ok(Json(json!({ "data": { "items": items } })))
}

/// DELETE /shares/:id  撤销分享
pub async fn revoke(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    share_service::revoke(&state.db, &user.user_id, &id).await?;
    Ok(Json(json!({ "data": { "ok": true } })))
}
