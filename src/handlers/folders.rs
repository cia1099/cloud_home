//! 资料夹端点。

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::error::AppResult;
use crate::models::file::{CreateFolderDto, FILE_TYPE_FOLDER, FileResponse};
use crate::services::file_service;
use crate::state::AppState;

/// POST /folders
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<CreateFolderDto>,
) -> AppResult<(StatusCode, JsonResp<Value>)> {
    file_service::validate_name(&dto.name)?;
    file_service::validate_parent(&state.db, &user.user_id, dto.parent_id.as_deref()).await?;
    file_service::ensure_name_available(
        &state.db,
        &user.user_id,
        dto.parent_id.as_deref(),
        &dto.name,
    )
    .await?;

    let id = uuid::Uuid::new_v4().to_string();
    let entry = file_service::insert_entry(
        &state.db,
        &id,
        &user.user_id,
        dto.parent_id.as_deref(),
        &state.drive_id,
        &dto.name,
        FILE_TYPE_FOLDER,
        0,
        None,
        None,
    )
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "data": FileResponse::from(entry) }))))
}
