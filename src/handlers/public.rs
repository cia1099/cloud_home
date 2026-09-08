//! 公开分享端点（无需认证）。

use axum::extract::{Path as AxumPath, State};
use axum::response::Response;

use crate::error::{AppError, AppResult, ErrorResponse};
use crate::handlers::files;
use crate::models::share::PublicShareResponse;
use crate::openapi::ApiResponse;
use crate::services::share_service;
use crate::state::AppState;

/// GET /public/shares/:token
#[utoipa::path(
    get,
    path = "/public/shares/{token}",
    tag = "public",
    params(("token" = String, Path, description = "分享令牌")),
    responses(
        (status = 200, description = "分享文件信息", body = ApiResponse<PublicShareResponse>),
        (status = 404, description = "分享不存在", body = ErrorResponse),
        (status = 410, description = "分享已过期", body = ErrorResponse),
    ),
    security()
)]
pub async fn info(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
) -> AppResult<ApiResponse<PublicShareResponse>> {
    let (share, file, shared_by) = share_service::resolve_public(&state.db, &token).await?;
    let resp = PublicShareResponse {
        name: file.name,
        file_type: file.file_type,
        size_bytes: file.size_bytes,
        mime_type: file.mime_type,
        can_download: share.can_download != 0,
        shared_by,
    };
    Ok(ApiResponse::new(resp))
}

/// GET /public/shares/:token/download
#[utoipa::path(
    get,
    path = "/public/shares/{token}/download",
    tag = "public",
    params(("token" = String, Path, description = "分享令牌")),
    responses(
        (status = 200, description = "文件内容", content_type = "application/octet-stream", body = [u8],
         headers(
             ("Content-Disposition" = String, description = "attachment; filename*=UTF-8''<name>"),
             ("Content-Length" = i64),
         )),
        (status = 403, description = "该分享禁止下载", body = ErrorResponse),
        (status = 404, description = "分享不存在", body = ErrorResponse),
        (status = 410, description = "分享已过期", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security()
)]
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
