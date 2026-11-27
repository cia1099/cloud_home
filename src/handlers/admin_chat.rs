//! 管理员聊天集成的私有端点：签发配对码、查看状态、取消配对。
//! 与其余 `/admin/*` 端点同一道 `AdminGuard`（口令 + 内网来源）。
//!
//! 不涉及任何用户的文件数据，只读写管理员配对身份，故无账户隔离问题。

use axum::extract::{Path, State};

use super::admin::AdminGuard;
use crate::error::{AppResult, ErrorResponse};
use crate::models::chat::{ChatStatusResponse, ChatUnpairResponse, PairingResponse};
use crate::openapi::ApiResponse;
use crate::state::AppState;

/// POST /admin/chat/pairing — 签发一次性配对码（8 位，10 分钟有效，单次使用）。
#[utoipa::path(
    post,
    path = "/admin/chat/pairing",
    tag = "admin",
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "配对码；管理员在聊天里发 `/pair <code>`", body = ApiResponse<PairingResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
        (status = 409, description = "CHAT_PROVIDER=none，聊天集成未启用", body = ErrorResponse),
    )
)]
pub async fn create_pairing(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> AppResult<ApiResponse<PairingResponse>> {
    Ok(ApiResponse::new(state.chat.create_pairing_code().await?))
}

/// GET /admin/chat — 提供方、连接状态与已配对的管理员。
#[utoipa::path(
    get,
    path = "/admin/chat",
    tag = "admin",
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "聊天集成状态", body = ApiResponse<ChatStatusResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
    )
)]
pub async fn status(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> AppResult<ApiResponse<ChatStatusResponse>> {
    Ok(ApiResponse::new(state.chat.status().await?))
}

/// DELETE /admin/chat/admins/{id} — 取消配对，立即生效。
#[utoipa::path(
    delete,
    path = "/admin/chat/admins/{id}",
    tag = "admin",
    params(
        ("id" = String, Path, description = "管理员配对 id（来自 GET /admin/chat）"),
        ("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）"),
    ),
    responses(
        (status = 200, description = "已取消配对", body = ApiResponse<ChatUnpairResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "配对不存在，或未配置 ADMIN_PASSWORD", body = ErrorResponse),
    )
)]
pub async fn unpair(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Path(id): Path<String>,
) -> AppResult<ApiResponse<ChatUnpairResponse>> {
    Ok(ApiResponse::new(state.chat.unpair(&id).await?))
}
