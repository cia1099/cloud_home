//! 分享链接端点（需认证）。

use axum::Json;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::common::OkResponse;
use crate::models::share::{CreateShareDto, ShareListResponse, ShareResponse};
use crate::openapi::ApiResponse;
use crate::services::share_service;
use crate::state::AppState;

/// POST /shares
#[utoipa::path(
    post,
    path = "/shares",
    tag = "shares",
    request_body = CreateShareDto,
    responses(
        (status = 201, description = "分享创建成功", body = ApiResponse<ShareResponse>),
        (status = 400, description = "expires_at 格式不正确", body = ErrorResponse),
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
    let resp = ShareResponse::from_link(link, &state.config.public_base_url);
    Ok((StatusCode::CREATED, ApiResponse::new(resp)))
}

/// GET /shares
#[utoipa::path(
    get,
    path = "/shares",
    tag = "shares",
    responses(
        (status = 200, description = "分享链接列表", body = ApiResponse<ShareListResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<ShareListResponse>> {
    let links = share_service::list(&state.db, &user.user_id).await?;
    let items: Vec<ShareResponse> = links
        .into_iter()
        .map(|l| ShareResponse::from_link(l, &state.config.public_base_url))
        .collect();
    Ok(ApiResponse::new(ShareListResponse { items }))
}

/// DELETE /shares/:id  撤销分享
#[utoipa::path(
    delete,
    path = "/shares/{id}",
    tag = "shares",
    params(("id" = String, Path, description = "分享链接 id")),
    responses(
        (status = 200, description = "已撤销", body = ApiResponse<OkResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "分享链接不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn revoke(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<OkResponse>> {
    share_service::revoke(&state.db, &user.user_id, &id).await?;
    Ok(ApiResponse::new(OkResponse { ok: true }))
}
