//! 资料夹端点。

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::file::{CreateFolderDto, FILE_TYPE_FOLDER, FileResponse};
use crate::openapi::ApiResponse;
use crate::services::file_service;
use crate::state::AppState;

/// POST /folders
#[utoipa::path(
    post,
    path = "/folders",
    tag = "folders",
    request_body = CreateFolderDto,
    responses(
        (status = 201, description = "资料夹创建成功", body = ApiResponse<FileResponse>),
        (status = 400, description = "名称不合法", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "parent_id 不存在", body = ErrorResponse),
        (status = 409, description = "已存在同名项目", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<CreateFolderDto>,
) -> AppResult<(StatusCode, ApiResponse<FileResponse>)> {
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

    Ok((
        StatusCode::CREATED,
        ApiResponse::new(FileResponse::from(entry)),
    ))
}
