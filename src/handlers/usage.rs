//! 空间占用查询端点。
//!
//! 与文件类端点不同，硬盘不可用时这里返回 `200`（`drive_available: false`）而非
//! `503 DRIVE_UNAVAILABLE`——前端需要能区分「硬盘拔出」与「服务不可用」，这正是本端点的用途。

use axum::extract::State;

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::usage::{UsageBreakdownResponse, UsageResponse};
use crate::openapi::ApiResponse;
use crate::services::usage_service;
use crate::state::AppState;

/// GET /usage  当前账户的空间占用与整盘容量。
#[utoipa::path(
    get,
    path = "/usage",
    tag = "usage",
    responses(
        (status = 200, description = "空间占用快照", body = ApiResponse<UsageResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn current(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<UsageResponse>> {
    let snapshot = usage_service::snapshot(&state.db, &state.drive_manager, &user.user_id).await?;
    Ok(ApiResponse::new(snapshot))
}

/// GET /usage/breakdown  全部账户在整盘中的占比（任意已登录用户可见）。
#[utoipa::path(
    get,
    path = "/usage/breakdown",
    tag = "usage",
    responses(
        (status = 200, description = "各账户占比", body = ApiResponse<UsageBreakdownResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn breakdown(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<ApiResponse<UsageBreakdownResponse>> {
    let result = usage_service::breakdown(&state.db, &state.drive_manager).await?;
    Ok(ApiResponse::new(result))
}
