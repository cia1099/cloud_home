//! 人员目录端点（需认证）：挑选分享收件人用。

use axum::extract::State;

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::share::DirectoryResponse;
use crate::openapi::ApiResponse;
use crate::services::share_service;
use crate::state::AppState;

/// GET /users/directory  其他已批准账户的 id 与显示名（不含任何联系方式）
#[utoipa::path(
    get,
    path = "/users/directory",
    tag = "users",
    responses(
        (status = 200, description = "可选择的收件人", body = ApiResponse<DirectoryResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn directory(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<DirectoryResponse>> {
    let items = share_service::directory(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(DirectoryResponse { items }))
}
