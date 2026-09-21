//! 运维端点：列出账户、软删除账户、还原账户。
//!
//! 这不是权限系统——没有管理员角色，只有一个共享口令（`ADMIN_PASSWORD`）
//! 外加「调用方必须来自本机 / 内网」的限制。两道关卡都由 `AdminGuard` 把守，
//! 且在读取请求体之前就完成。
//!
//! 注意反向代理场景下的局限：网关在前时对端 IP 就是网关自己的回环地址，
//! 内网判定会形同虚设，除非开启 `TRUST_PROXY_HEADERS` 并信任 `X-Forwarded-For`。
//! 它是给局域网机器加的一层防护，不能替代口令本身。

use std::net::IpAddr;

use axum::Json;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;

use crate::error::{AppError, AppResult, ErrorResponse};
use crate::models::event::{ChangeKind, FileChangeEvent};
use crate::models::user::{
    AdminActionResponse, AdminPurgeResponse, AdminTargetDto, AdminUserItem, AdminUserListResponse,
    IdentityResponse, PROVIDER_PASSWORD,
};
use crate::openapi::ApiResponse;
use crate::services::{trash_service, user_service};
use crate::state::AppState;

/// 管理端点守卫：功能开关 → 来源网段 → 共享口令，任一不过即拒。
pub struct AdminGuard;

impl FromRequestParts<AppState> for AdminGuard {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // 未配置口令即视为功能未启用，返回 404 而非 403：没开就当这条路由不存在。
        let Some(expected) = state
            .config
            .admin_password
            .as_deref()
            .filter(|s| !s.is_empty())
        else {
            return Err(AppError::NotFound);
        };

        let ip = crate::logging::client_ip(
            &parts.headers,
            &parts.extensions,
            state.config.trust_proxy_headers,
        );
        if !ip.map(is_local).unwrap_or(false) {
            crate::audit!(
                warn,
                "admin.auth",
                "failure",
                reason = "remote_caller",
                client_ip = %ip.map(|i| i.to_string()).unwrap_or_else(|| "unknown".into()),
                "管理端点拒绝非本地调用"
            );
            return Err(AppError::ForbiddenReason(
                "管理端点仅允许本机或内网调用".into(),
            ));
        }

        let supplied = parts
            .headers
            .get("x-admin-password")
            .and_then(|v| v.to_str().ok());
        let Some(supplied) = supplied else {
            crate::audit!(
                warn,
                "admin.auth",
                "failure",
                reason = "missing_admin_password",
                "管理端点缺少口令"
            );
            return Err(AppError::Unauthorized);
        };
        if !constant_time_eq(supplied.as_bytes(), expected.as_bytes()) {
            crate::audit!(
                warn,
                "admin.auth",
                "failure",
                reason = "bad_admin_password",
                "管理端点口令错误"
            );
            return Err(AppError::Unauthorized);
        }

        Ok(AdminGuard)
    }
}

/// 回环或 RFC1918 / ULA 私有地址。
fn is_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            v6.is_loopback()
                // fc00::/7 唯一本地地址；std 的 is_unique_local 尚未稳定，自行判定。
                || (v6.segments()[0] & 0xfe00) == 0xfc00
                // fe80::/10 链路本地。
                || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

/// 定长比较，避免按字节提前返回而泄露口令前缀。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y;
    }
    diff == 0
}

/// GET /admin/users — 列出所有账户（含已停用），用于定位要删除 / 还原的对象。
#[utoipa::path(
    get,
    path = "/admin/users",
    tag = "admin",
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "账户列表", body = ApiResponse<AdminUserListResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
    )
)]
pub async fn list_users(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> AppResult<ApiResponse<AdminUserListResponse>> {
    let rows = user_service::list_all(&state.db).await?;
    let items = rows
        .into_iter()
        .map(|(u, ids)| AdminUserItem {
            id: u.id,
            username: u.username,
            is_active: u.is_active != 0,
            deleted_at: u.deleted_at,
            identities: ids
                .iter()
                .filter(|i| i.provider != PROVIDER_PASSWORD)
                .map(IdentityResponse::from)
                .collect(),
        })
        .collect();
    Ok(ApiResponse::new(AdminUserListResponse { items }))
}

/// POST /admin/users/delete — 软删除账户：文件全部进回收站，账户停用。
///
/// 用户行不会被物理删除，回收站清理任务到期后只清文件。
#[utoipa::path(
    post,
    path = "/admin/users/delete",
    tag = "admin",
    request_body = AdminTargetDto,
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "已删除", body = ApiResponse<AdminActionResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "账户不存在，或未配置 ADMIN_PASSWORD", body = ErrorResponse),
        (status = 409, description = "标识命中多个账户，请用 provider 指定；或账户已停用", body = ErrorResponse),
    )
)]
pub async fn delete_user(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(dto): Json<AdminTargetDto>,
) -> AppResult<ApiResponse<AdminActionResponse>> {
    let target = user_service::find_any(&state.db, dto.identifier.trim(), dto.provider.as_deref())
        .await?;
    let (user, affected) =
        user_service::soft_remove(&state.db, state.config.trash_retention_days, &target.id).await?;

    crate::audit!(
        info,
        "admin.user.delete",
        "success",
        target_user_id = %user.id,
        target_username = %user.username,
        affected_items = affected,
        "运维删除账户"
    );

    Ok(ApiResponse::new(AdminActionResponse {
        user_id: user.id,
        username: user.username,
        affected_items: affected,
    }))
}

/// POST /admin/users/restore — 还原账户。
///
/// 只还原当初随删除一起进回收站的项目；用户自己先前删掉的东西保持原样。
/// 若保留期已过、文件被清理任务清走，则账户恢复为可用但内容为空。
#[utoipa::path(
    post,
    path = "/admin/users/restore",
    tag = "admin",
    request_body = AdminTargetDto,
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "已还原", body = ApiResponse<AdminActionResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "账户不存在，或未配置 ADMIN_PASSWORD", body = ErrorResponse),
        (status = 409, description = "标识命中多个账户，请用 provider 指定；或账户未被停用", body = ErrorResponse),
    )
)]
pub async fn restore_user(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(dto): Json<AdminTargetDto>,
) -> AppResult<ApiResponse<AdminActionResponse>> {
    let target = user_service::find_any(&state.db, dto.identifier.trim(), dto.provider.as_deref())
        .await?;
    let (user, affected) = user_service::restore(&state.db, &target.id).await?;

    crate::audit!(
        info,
        "admin.user.restore",
        "success",
        target_user_id = %user.id,
        target_username = %user.username,
        affected_items = affected,
        "运维还原账户"
    );

    Ok(ApiResponse::new(AdminActionResponse {
        user_id: user.id,
        username: user.username,
        affected_items: affected,
    }))
}

/// POST /admin/trash/purge — 立即清空某账户的回收站（物理删除，不等保留期）。
///
/// 为什么必须有这个端点：账户被 `/admin/users/delete` 停用后，它的令牌会被拒绝，
/// 于是它自己的 `DELETE /trash` 再也调不动——那批文件只能在盘上躺满保留期，
/// 等清理任务来收。确认不再需要该账户时，这里可以立刻把空间要回来。
///
/// 对未停用的账户同样可用，纯粹当作「腾空间」使用；只动回收站，不碰在用文件。
#[utoipa::path(
    post,
    path = "/admin/trash/purge",
    tag = "admin",
    request_body = AdminTargetDto,
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "已清空；返回删除项目数与释放字节数", body = ApiResponse<AdminPurgeResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "账户不存在，或未配置 ADMIN_PASSWORD", body = ErrorResponse),
        (status = 409, description = "标识命中多个账户，请用 provider 指定", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用，无法物理删除", body = ErrorResponse),
    )
)]
pub async fn purge_trash(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(dto): Json<AdminTargetDto>,
) -> AppResult<ApiResponse<AdminPurgeResponse>> {
    let target =
        user_service::find_any(&state.db, dto.identifier.trim(), dto.provider.as_deref()).await?;

    // 先确认盘在：物理删除做不了就不该动数据库，否则会留下指向不存在文件的空记录。
    let data_root = state.drive_manager.require_data_root().await?;
    let (count, bytes) = trash_service::empty(&state.db, &data_root, &target.id).await?;

    crate::audit!(
        info,
        "admin.trash.purge",
        "success",
        target_user_id = %target.id,
        target_username = %target.username,
        deleted_count = count,
        freed_bytes = bytes,
        "运维清空账户回收站"
    );

    if count > 0 {
        state.events.notify(
            &target.id,
            FileChangeEvent {
                kind: ChangeKind::Purged,
                file: None,
                parent_ids: Vec::new(),
            },
        );
    }

    Ok(ApiResponse::new(AdminPurgeResponse {
        user_id: target.id,
        username: target.username,
        deleted_count: count as i64,
        freed_bytes: bytes as i64,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_and_private_are_local() {
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.5",
            "172.16.0.1",
            "172.31.255.254",
            "::1",
            "fd00::1",
        ] {
            assert!(is_local(ip.parse().unwrap()), "{ip} should be local");
        }
    }

    #[test]
    fn public_addresses_are_not_local() {
        for ip in ["8.8.8.8", "1.1.1.1", "172.32.0.1", "2001:4860:4860::8888"] {
            assert!(!is_local(ip.parse().unwrap()), "{ip} should not be local");
        }
    }

    #[test]
    fn constant_time_eq_matches_only_identical() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(!constant_time_eq(b"secret", b"secreT"));
        assert!(!constant_time_eq(b"secret", b"secre"));
        assert!(!constant_time_eq(b"", b"x"));
    }
}
