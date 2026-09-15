//! 公开分享端点（无需认证）。

use axum::extract::{Path as AxumPath, State};
use axum::http::HeaderMap;
use axum::http::header::RANGE;
use axum::response::Response;

use crate::error::{AppError, AppResult, ErrorResponse};
use crate::handlers::files::{self, Disposition};
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
    crate::audit!(
        info,
        "public.share_view",
        "success",
        share_token_prefix = %token_prefix(&token),
        owner_id = %share.owner_id,
        file_id = %file.id,
        "查看分享信息"
    );
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

/// 仅取 token 前 8 字符用于日志——完整 token 是访问凭证，绝不可写入日志文件。
fn token_prefix(token: &str) -> String {
    token.chars().take(8).collect()
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
             ("Accept-Ranges" = String, description = "bytes"),
         )),
        (status = 206, description = "部分内容（响应 Range 请求）", content_type = "application/octet-stream", body = [u8],
         headers(("Content-Range" = String, description = "bytes {start}-{end}/{total}"))),
        (status = 403, description = "该分享禁止下载", body = ErrorResponse),
        (status = 404, description = "分享不存在", body = ErrorResponse),
        (status = 410, description = "分享已过期", body = ErrorResponse),
        (status = 416, description = "Range 不满足"),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security()
)]
pub async fn download(
    State(state): State<AppState>,
    AxumPath(token): AxumPath<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let (share, file, _shared_by) = share_service::resolve_public(&state.db, &token).await?;

    if share.can_download == 0 {
        crate::audit!(
            warn,
            "public.share_download",
            "failure",
            reason = "download_disabled",
            share_token_prefix = %token_prefix(&token),
            owner_id = %share.owner_id,
            file_id = %file.id,
            "分享禁止下载"
        );
        return Err(AppError::Forbidden);
    }

    let range = headers.get(RANGE).and_then(|v| v.to_str().ok());
    let response = files::stream_file(
        &state,
        &share.owner_id,
        &file,
        Disposition::Attachment,
        range,
    )
    .await?;
    crate::audit!(
        info,
        "public.share_download",
        "success",
        share_token_prefix = %token_prefix(&token),
        owner_id = %share.owner_id,
        file_id = %file.id,
        size_bytes = file.size_bytes,
        "下载分享文件"
    );
    share_service::record_access(&state.db, &share.id).await?;
    Ok(response)
}
