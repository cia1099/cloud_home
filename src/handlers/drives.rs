//! 硬盘状态端点。

use axum::extract::State;

use crate::auth::AuthUser;
use crate::db::Db;
use crate::error::{AppResult, ErrorResponse};
use crate::models::drive::{DriveInfo, DrivesListResponse};
use crate::openapi::ApiResponse;
use crate::state::AppState;

/// GET /drives
#[utoipa::path(
    get,
    path = "/drives",
    tag = "drives",
    responses(
        (status = 200, description = "已侦测到的外接硬盘列表", body = ApiResponse<DrivesListResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<ApiResponse<DrivesListResponse>> {
    let drives = fetch_drives(&state.db).await?;
    let active_mount = state.drive_manager.active_mount().await;

    Ok(ApiResponse::new(DrivesListResponse {
        active_drive: active_mount.map(|p| p.display().to_string()),
        is_available: active_mount_is_some(&state).await,
        drives,
    }))
}

async fn active_mount_is_some(state: &AppState) -> bool {
    state.drive_manager.is_available().await
}

async fn fetch_drives(db: &Db) -> AppResult<Vec<DriveInfo>> {
    let rows: Vec<(String, String, Option<String>, i64, String, String)> = sqlx::query_as(
        "SELECT id, mount_path, label, is_active, detected_at, last_seen_at FROM drives \
         ORDER BY last_seen_at DESC",
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(id, mount_path, label, is_active, detected_at, last_seen_at)| DriveInfo {
                id,
                mount_path,
                label,
                is_active: is_active != 0,
                detected_at,
                last_seen_at,
            },
        )
        .collect())
}
