//! 资料夹端点。

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::models::event::{ChangeKind, FileChangeEvent};
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
        (status = 201, description = "资料夹创建成功；若同名已存在会自动追加 (1)/(2)…… 后缀", body = ApiResponse<FileResponse>),
        (status = 400, description = "名称不合法", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "parent_id 不存在", body = ErrorResponse),
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
    // 创建时若同名已存在，仿照系统复制文件的做法自动追加 (1)/(2)……而非拒绝。
    let name = file_service::dedupe_name(
        &state.db,
        &user.user_id,
        dto.parent_id.as_deref(),
        &dto.name,
        true,
    )
    .await?;

    let id = uuid::Uuid::new_v4().to_string();
    let entry = file_service::insert_entry(
        &state.db,
        &id,
        &user.user_id,
        dto.parent_id.as_deref(),
        &state.drive_id,
        &name,
        FILE_TYPE_FOLDER,
        0,
        None,
        None,
    )
    .await?;

    let response = FileResponse::from(entry);
    let file_path = file_service::full_path(
        &state.db,
        &user.user_id,
        response.parent_id.as_deref(),
        &response.name,
    )
    .await
    .unwrap_or_else(|_| "?".into());
    crate::audit!(
        info,
        "folder.create",
        "success",
        file_id = %response.id,
        file_name = %response.name,
        file_path = %file_path,
        "创建资料夹"
    );
    state.events.notify(
        &user.user_id,
        FileChangeEvent {
            kind: ChangeKind::Created,
            parent_ids: vec![response.parent_id.clone()],
            file: Some(response.clone()),
        },
    );

    Ok((StatusCode::CREATED, ApiResponse::new(response)))
}
