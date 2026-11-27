//! 分享端点（需认证）：公开链接、站内定向分享（收件人）及收件人的只读访问。

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;

use crate::auth::AuthUser;
use crate::error::{AppError, AppResult, ErrorResponse};
use crate::events::ShareNoticeKind;
use crate::handlers::files::{self, Disposition};
use crate::models::common::OkResponse;
use crate::models::file::{FileListResponse, FileResponse, ThumbnailQuery};
use crate::models::share::{
    CreateShareDto, ReceivedListQuery, ReceivedShareListResponse, ShareLink, ShareListResponse,
    ShareResponse, UpdateShareDto,
};
use crate::openapi::ApiResponse;
use crate::services::{file_service, share_service};
use crate::state::AppState;

/// 组装单条分享响应（附带收件人）。
async fn respond(state: &AppState, owner_id: &str, link: ShareLink) -> AppResult<ShareResponse> {
    let mut recipients = share_service::recipients_by_share(&state.db, owner_id).await?;
    let people = recipients.remove(&link.id).unwrap_or_default();
    Ok(ShareResponse::from_link(
        link,
        people,
        &state.config.public_base_url,
    ))
}

/// POST /shares
#[utoipa::path(
    post,
    path = "/shares",
    tag = "shares",
    request_body = CreateShareDto,
    responses(
        (status = 201, description = "分享创建成功；带 recipient_ids 时是站内定向分享（无 token/url）", body = ApiResponse<ShareResponse>),
        (status = 400, description = "expires_at 格式不正确，或 recipient_ids 含无效收件人", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<CreateShareDto>,
) -> AppResult<(StatusCode, ApiResponse<ShareResponse>)> {
    let link = share_service::create(&state.db, &user.user_id, &dto).await?;
    let resp = respond(&state, &user.user_id, link).await?;
    crate::audit!(
        info,
        "share.create",
        "success",
        share_id = %resp.id,
        file_id = %resp.file_id,
        share_token_prefix = %resp.token.as_deref().map(token_prefix).unwrap_or_default(),
        recipient_count = resp.recipients.len(),
        expires_at = ?resp.expires_at,
        can_download = resp.can_download,
        "创建分享"
    );
    for p in &resp.recipients {
        state
            .events
            .notify_share(&p.id, ShareNoticeKind::Received, &resp.id);
    }
    Ok((StatusCode::CREATED, ApiResponse::new(resp)))
}

/// 仅取 token 前 8 字符用于日志——完整 token 是访问凭证，绝不可写入日志文件。
fn token_prefix(token: &str) -> String {
    token.chars().take(8).collect()
}

/// GET /shares
#[utoipa::path(
    get,
    path = "/shares",
    tag = "shares",
    responses(
        (status = 200, description = "自己创建的分享列表（含收件人）", body = ApiResponse<ShareListResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<ShareListResponse>> {
    let links = share_service::list(&state.db, &user.user_id).await?;
    let mut recipients = share_service::recipients_by_share(&state.db, &user.user_id).await?;
    let items: Vec<ShareResponse> = links
        .into_iter()
        .map(|l| {
            let people = recipients.remove(&l.id).unwrap_or_default();
            ShareResponse::from_link(l, people, &state.config.public_base_url)
        })
        .collect();
    Ok(ApiResponse::new(ShareListResponse { items }))
}

/// PATCH /shares/:id  整组替换定向分享的收件人
#[utoipa::path(
    patch,
    path = "/shares/{id}",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id")),
    request_body = UpdateShareDto,
    responses(
        (status = 200, description = "已更新", body = ApiResponse<ShareResponse>),
        (status = 400, description = "收件人无效、为空，或这是公开链接", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在（或不是自己的、已撤销）", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    Json(dto): Json<UpdateShareDto>,
) -> AppResult<ApiResponse<ShareResponse>> {
    let (link, added, removed) =
        share_service::update_recipients(&state.db, &user.user_id, &id, &dto.recipient_ids)
            .await?;
    crate::audit!(
        info,
        "share.update",
        "success",
        share_id = %id,
        added_count = added.len(),
        removed_count = removed.len(),
        "修改分享收件人"
    );
    for u in &added {
        state.events.notify_share(u, ShareNoticeKind::Received, &id);
    }
    for u in &removed {
        state.events.notify_share(u, ShareNoticeKind::Revoked, &id);
    }
    Ok(ApiResponse::new(respond(&state, &user.user_id, link).await?))
}

/// DELETE /shares/:id  撤销分享（收件人的列表立即不再包含它）
#[utoipa::path(
    delete,
    path = "/shares/{id}",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id")),
    responses(
        (status = 200, description = "已撤销", body = ApiResponse<OkResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn revoke(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<OkResponse>> {
    let recipients = share_service::revoke(&state.db, &user.user_id, &id).await?;
    crate::audit!(
        info,
        "share.revoke",
        "success",
        share_id = %id,
        recipient_count = recipients.len(),
        "撤销分享"
    );
    for u in &recipients {
        state.events.notify_share(u, ShareNoticeKind::Revoked, &id);
    }
    Ok(ApiResponse::new(OkResponse { ok: true }))
}

/// GET /shares/received  分享给我的项目（最新在前；已撤销、已过期的不返回）
#[utoipa::path(
    get,
    path = "/shares/received",
    tag = "shares",
    responses(
        (status = 200, description = "收到的分享", body = ApiResponse<ReceivedShareListResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<ReceivedShareListResponse>> {
    let items = share_service::list_received(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(ReceivedShareListResponse { items }))
}

/// POST /shares/:id/seen  收件人标记已读（`seen_at` 只记第一次）
#[utoipa::path(
    post,
    path = "/shares/{id}/seen",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id")),
    responses(
        (status = 200, description = "已标记", body = ApiResponse<OkResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "你不是这条分享的收件人", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn seen(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<OkResponse>> {
    share_service::mark_seen(&state.db, &user.user_id, &id).await?;
    Ok(ApiResponse::new(OkResponse { ok: true }))
}

/// 收件人查看类访问的审计行（收件人的 user_id 由请求 span 自动带上）。
fn audit_view(share: &ShareLink, file_id: &str, what: &'static str) {
    crate::audit!(
        info,
        "share.received_view",
        "success",
        share_id = %share.id,
        owner_id = %share.owner_id,
        file_id = %file_id,
        what,
        "收件人查看分享内容"
    );
}

/// GET /shares/received/:id/files?parent_id=  列出被分享资料夹的子项（只读，限定在被分享的子树内）
#[utoipa::path(
    get,
    path = "/shares/received/{id}/files",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id"), ReceivedListQuery),
    responses(
        (status = 200, description = "子项", body = ApiResponse<FileListResponse>),
        (status = 400, description = "不是资料夹", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在、无权访问或已失效", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_children(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    Query(q): Query<ReceivedListQuery>,
) -> AppResult<ApiResponse<FileListResponse>> {
    let (share, parent) =
        share_service::resolve_received(&state.db, &user.user_id, &id, q.parent_id.as_deref())
            .await?;
    let items = file_service::list_dir(&state.db, &share.owner_id, Some(&parent.id)).await?;
    audit_view(&share, &parent.id, "list");
    let items = items.into_iter().map(FileResponse::from).collect();
    Ok(ApiResponse::new(FileListResponse { items }))
}

/// GET /shares/received/:id/files/:file_id  被分享项目（或其后代）的元数据
#[utoipa::path(
    get,
    path = "/shares/received/{id}/files/{file_id}",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id"), ("file_id" = String, Path, description = "被分享项目或其后代的 id")),
    responses(
        (status = 200, description = "元数据", body = ApiResponse<FileResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在、无权访问或已失效", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_metadata(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath((id, file_id)): AxumPath<(String, String)>,
) -> AppResult<ApiResponse<FileResponse>> {
    let (share, file) =
        share_service::resolve_received(&state.db, &user.user_id, &id, Some(&file_id)).await?;
    audit_view(&share, &file.id, "metadata");
    Ok(ApiResponse::new(FileResponse::from(file)))
}

/// GET /shares/received/:id/files/:file_id/raw  内联预览/播放，支持 Range
#[utoipa::path(
    get,
    path = "/shares/received/{id}/files/{file_id}/raw",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id"), ("file_id" = String, Path, description = "文件 id")),
    responses(
        (status = 200, description = "文件内容（内联）", content_type = "application/octet-stream", body = [u8]),
        (status = 206, description = "部分内容（响应 Range 请求）", content_type = "application/octet-stream", body = [u8]),
        (status = 400, description = "资料夹不能预览", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在、无权访问或已失效", body = ErrorResponse),
        (status = 416, description = "Range 不满足"),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_raw(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath((id, file_id)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let (share, file) =
        share_service::resolve_received(&state.db, &user.user_id, &id, Some(&file_id)).await?;
    let response = files::stream_file(
        &state,
        &share.owner_id,
        &file,
        Disposition::Inline,
        files::range_header(&headers),
    )
    .await?;
    audit_view(&share, &file.id, "raw");
    Ok(response)
}

/// GET /shares/received/:id/files/:file_id/thumbnail  图片缩略图
#[utoipa::path(
    get,
    path = "/shares/received/{id}/files/{file_id}/thumbnail",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id"), ("file_id" = String, Path, description = "文件 id"), ThumbnailQuery),
    responses(
        (status = 200, description = "缩略图", content_type = "image/jpeg", body = [u8]),
        (status = 400, description = "非图片文件或 size 不在允许范围内", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享不存在、无权访问或已失效", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_thumbnail(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath((id, file_id)): AxumPath<(String, String)>,
    Query(q): Query<ThumbnailQuery>,
) -> AppResult<Response> {
    let (share, file) =
        share_service::resolve_received(&state.db, &user.user_id, &id, Some(&file_id)).await?;
    let response = files::thumbnail_response(&state, &share.owner_id, &file, q.size).await?;
    audit_view(&share, &file.id, "thumbnail");
    Ok(response)
}

/// GET /shares/received/:id/files/:file_id/download  强制另存；分享禁止下载时 403
#[utoipa::path(
    get,
    path = "/shares/received/{id}/files/{file_id}/download",
    tag = "shares",
    params(("id" = String, Path, description = "分享 id"), ("file_id" = String, Path, description = "文件 id")),
    responses(
        (status = 200, description = "文件内容", content_type = "application/octet-stream", body = [u8]),
        (status = 206, description = "部分内容（响应 Range 请求）", content_type = "application/octet-stream", body = [u8]),
        (status = 400, description = "资料夹不能下载", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 403, description = "该分享禁止下载", body = ErrorResponse),
        (status = 404, description = "分享不存在、无权访问或已失效", body = ErrorResponse),
        (status = 416, description = "Range 不满足"),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn received_download(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath((id, file_id)): AxumPath<(String, String)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let (share, file) =
        share_service::resolve_received(&state.db, &user.user_id, &id, Some(&file_id)).await?;
    if share.can_download == 0 {
        crate::audit!(
            warn,
            "share.received_download",
            "failure",
            reason = "download_disabled",
            share_id = %share.id,
            owner_id = %share.owner_id,
            file_id = %file.id,
            "分享禁止下载"
        );
        return Err(AppError::Forbidden);
    }
    let response = files::stream_file(
        &state,
        &share.owner_id,
        &file,
        Disposition::Attachment,
        files::range_header(&headers),
    )
    .await?;
    crate::audit!(
        info,
        "share.received_download",
        "success",
        share_id = %share.id,
        owner_id = %share.owner_id,
        file_id = %file.id,
        size_bytes = file.size_bytes,
        "收件人下载分享文件"
    );
    Ok(response)
}
