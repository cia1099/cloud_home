//! 回收站端点。

use axum::extract::{Path as AxumPath, State};

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::file::FileResponse;
use crate::models::trash::{FreedBytesResponse, TrashEmptyResponse, TrashListResponse};
use crate::openapi::ApiResponse;
use crate::services::trash_service;
use crate::state::AppState;

/// GET /trash
#[utoipa::path(
    get,
    path = "/trash",
    tag = "trash",
    responses(
        (status = 200, description = "回收站内容", body = ApiResponse<TrashListResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<TrashListResponse>> {
    let items = trash_service::list(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(TrashListResponse { items }))
}

/// POST /trash/:id/restore
#[utoipa::path(
    post,
    path = "/trash/{id}/restore",
    tag = "trash",
    params(("id" = String, Path, description = "回收站记录 id")),
    responses(
        (status = 200, description = "恢复成功", body = ApiResponse<FileResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "回收站记录不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn restore(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<FileResponse>> {
    let entry = trash_service::restore(&state.db, &user.user_id, &id).await?;
    Ok(ApiResponse::new(FileResponse::from(entry)))
}

/// DELETE /trash/:id  立即永久删除单个文件
#[utoipa::path(
    delete,
    path = "/trash/{id}",
    tag = "trash",
    params(("id" = String, Path, description = "回收站记录 id")),
    responses(
        (status = 200, description = "已永久删除", body = ApiResponse<FreedBytesResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "回收站记录不存在", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn delete_one(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<FreedBytesResponse>> {
    let data_root = state.drive_manager.require_data_root().await?;
    let bytes = trash_service::delete_permanent(&state.db, &data_root, &user.user_id, &id).await?;
    Ok(ApiResponse::new(FreedBytesResponse {
        freed_bytes: bytes as i64,
    }))
}

/// DELETE /trash  清空回收站
#[utoipa::path(
    delete,
    path = "/trash",
    tag = "trash",
    responses(
        (status = 200, description = "回收站已清空", body = ApiResponse<TrashEmptyResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn empty(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<TrashEmptyResponse>> {
    let data_root = state.drive_manager.require_data_root().await?;
    let (count, bytes) = trash_service::empty(&state.db, &data_root, &user.user_id).await?;
    Ok(ApiResponse::new(TrashEmptyResponse {
        deleted_count: count as i64,
        freed_bytes: bytes as i64,
    }))
}
