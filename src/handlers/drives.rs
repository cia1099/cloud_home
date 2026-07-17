//! 硬盘状态端点。

use axum::extract::State;
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::db::Db;
use crate::error::AppResult;
use crate::state::AppState;

/// GET /drives
pub async fn list(
    State(state): State<AppState>,
    _user: AuthUser,
) -> AppResult<JsonResp<Value>> {
    let drives = fetch_drives(&state.db).await?;
    let active_mount = state.drive_manager.active_mount().await;

    Ok(Json(json!({
        "data": {
            "active_drive": active_mount.map(|p| p.display().to_string()),
            "is_available": active_mount_is_some(&state).await,
            "drives": drives,
        }
    })))
}

async fn active_mount_is_some(state: &AppState) -> bool {
    state.drive_manager.is_available().await
}

async fn fetch_drives(db: &Db) -> AppResult<Vec<Value>> {
    let rows: Vec<(String, String, Option<String>, i64, String, String)> = sqlx::query_as(
        "SELECT id, mount_path, label, is_active, detected_at, last_seen_at FROM drives \
         ORDER BY last_seen_at DESC",
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(id, mount_path, label, is_active, detected_at, last_seen_at)| {
            json!({
                "id": id,
                "mount_path": mount_path,
                "label": label,
                "is_active": is_active != 0,
                "detected_at": detected_at,
                "last_seen_at": last_seen_at,
            })
        })
        .collect())
}
