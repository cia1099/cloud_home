//! 文件操作端点。

use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use tokio::io::AsyncWriteExt;

use crate::auth::AuthUser;
use crate::drive::path_resolver;
use crate::error::{AppError, AppResult, ErrorResponse};
use crate::models::common::OkResponse;
use crate::models::file::{
    FILE_TYPE_FILE, FileEntry, FileListResponse, FileResponse, ListQuery, MoveDto, RenameDto,
};
use crate::openapi::ApiResponse;
use crate::services::{file_service, trash_service};
use crate::state::AppState;

/// GET /files?parent_id=
#[utoipa::path(
    get,
    path = "/files",
    tag = "files",
    params(ListQuery),
    responses(
        (status = 200, description = "目录内容", body = ApiResponse<FileListResponse>),
        (status = 400, description = "parent_id 不是资料夹", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "parent_id 不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<ApiResponse<FileListResponse>> {
    let items = file_service::list_dir(&state.db, &user.user_id, q.parent_id.as_deref()).await?;
    let items: Vec<FileResponse> = items.into_iter().map(FileResponse::from).collect();
    Ok(ApiResponse::new(FileListResponse { items }))
}

/// GET /files/:id
#[utoipa::path(
    get,
    path = "/files/{id}",
    tag = "files",
    params(("id" = String, Path, description = "文件/资料夹 id")),
    responses(
        (status = 200, description = "文件元数据", body = ApiResponse<FileResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn get_metadata(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<FileResponse>> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    Ok(ApiResponse::new(FileResponse::from(entry)))
}

/// POST /files/upload （multipart/form-data）
#[utoipa::path(
    post,
    path = "/files/upload",
    tag = "files",
    request_body(content = crate::models::file::UploadForm, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "上传成功", body = ApiResponse<FileResponse>),
        (status = 400, description = "参数校验失败", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 409, description = "已存在同名项目", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, ApiResponse<FileResponse>)> {
    let data_root = state.drive_manager.require_data_root().await?;

    let file_id = uuid::Uuid::new_v4().to_string();
    let physical_path = path_resolver::file_path(&data_root, &user.user_id, &file_id);

    let mut parent_id: Option<String> = None;
    let mut name: Option<String> = None;
    let mut wrote_file = false;
    let mut size_bytes: i64 = 0;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::BadRequest("multipart 解析失败".into()))?
    {
        match field.name() {
            Some("parent_id") => {
                let v = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("parent_id 读取失败".into()))?;
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    parent_id = Some(trimmed.to_string());
                }
            }
            Some("name") => {
                let v = field
                    .text()
                    .await
                    .map_err(|_| AppError::BadRequest("name 读取失败".into()))?;
                let trimmed = v.trim();
                if !trimmed.is_empty() {
                    name = Some(trimmed.to_string());
                }
            }
            Some("file") => {
                if name.is_none() {
                    name = field.file_name().map(|s| s.to_string());
                }
                let dir = path_resolver::file_dir(&data_root, &user.user_id, &file_id);
                tokio::fs::create_dir_all(&dir).await?;
                let mut out = tokio::fs::File::create(&physical_path).await?;
                while let Some(chunk) = field
                    .chunk()
                    .await
                    .map_err(|_| AppError::BadRequest("文件流读取失败".into()))?
                {
                    out.write_all(&chunk).await?;
                    size_bytes += chunk.len() as i64;
                }
                out.flush().await?;
                wrote_file = true;
            }
            _ => {}
        }
    }

    // 校验与落库；失败时清理已写入的物理文件，避免孤儿。
    let result = finalize_upload(
        &state,
        &user.user_id,
        &file_id,
        parent_id.as_deref(),
        name,
        wrote_file,
        size_bytes,
        &physical_path,
    )
    .await;

    match result {
        Ok(entry) => Ok((
            StatusCode::CREATED,
            ApiResponse::new(FileResponse::from(entry)),
        )),
        Err(e) => {
            let _ = tokio::fs::remove_file(&physical_path).await;
            Err(e)
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn finalize_upload(
    state: &AppState,
    owner_id: &str,
    file_id: &str,
    parent_id: Option<&str>,
    name: Option<String>,
    wrote_file: bool,
    size_bytes: i64,
    physical_path: &std::path::Path,
) -> AppResult<FileEntry> {
    if !wrote_file {
        return Err(AppError::BadRequest("缺少 file 字段".into()));
    }
    let name = name.ok_or_else(|| AppError::BadRequest("缺少文件名".into()))?;
    file_service::validate_name(&name)?;
    file_service::validate_parent(&state.db, owner_id, parent_id).await?;
    file_service::ensure_name_available(&state.db, owner_id, parent_id, &name).await?;

    let mime = mime_guess::from_path(&name)
        .first_raw()
        .map(|s| s.to_string());
    let path_str = physical_path.to_string_lossy().to_string();

    file_service::insert_entry(
        &state.db,
        file_id,
        owner_id,
        parent_id,
        &state.drive_id,
        &name,
        FILE_TYPE_FILE,
        size_bytes,
        mime.as_deref(),
        Some(&path_str),
    )
    .await
}

/// GET /files/:id/download
#[utoipa::path(
    get,
    path = "/files/{id}/download",
    tag = "files",
    params(("id" = String, Path, description = "文件 id")),
    responses(
        (status = 200, description = "文件内容", content_type = "application/octet-stream", body = [u8],
         headers(
             ("Content-Disposition" = String, description = "attachment; filename*=UTF-8''<name>"),
             ("Content-Length" = i64),
         )),
        (status = 400, description = "资料夹不能下载", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn download(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<Response> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    stream_file(&state, &user.user_id, &entry).await
}

/// PATCH /files/:id  重命名
#[utoipa::path(
    patch,
    path = "/files/{id}",
    tag = "files",
    params(("id" = String, Path, description = "文件/资料夹 id")),
    request_body = RenameDto,
    responses(
        (status = 200, description = "重命名成功", body = ApiResponse<FileResponse>),
        (status = 400, description = "名称不合法", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
        (status = 409, description = "已存在同名项目", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn rename(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    axum::Json(dto): axum::Json<RenameDto>,
) -> AppResult<ApiResponse<FileResponse>> {
    let entry = file_service::rename(&state.db, &user.user_id, &id, &dto.name).await?;
    Ok(ApiResponse::new(FileResponse::from(entry)))
}

/// POST /files/:id/move
#[utoipa::path(
    post,
    path = "/files/{id}/move",
    tag = "files",
    params(("id" = String, Path, description = "文件/资料夹 id")),
    request_body = MoveDto,
    responses(
        (status = 200, description = "移动成功", body = ApiResponse<FileResponse>),
        (status = 400, description = "目标非法或形成环", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件或目标目录不存在", body = ErrorResponse),
        (status = 409, description = "目标目录下已存在同名项目", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn move_file(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    axum::Json(dto): axum::Json<MoveDto>,
) -> AppResult<ApiResponse<FileResponse>> {
    let entry = file_service::move_entry(
        &state.db,
        &user.user_id,
        &id,
        dto.target_parent_id.as_deref(),
    )
    .await?;
    Ok(ApiResponse::new(FileResponse::from(entry)))
}

/// DELETE /files/:id  软删除（移入回收站）
#[utoipa::path(
    delete,
    path = "/files/{id}",
    tag = "files",
    params(("id" = String, Path, description = "文件/资料夹 id")),
    responses(
        (status = 200, description = "已移入回收站", body = ApiResponse<OkResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<ApiResponse<OkResponse>> {
    trash_service::soft_delete(
        &state.db,
        state.config.trash_retention_days,
        &user.user_id,
        &id,
    )
    .await?;
    Ok(ApiResponse::new(OkResponse { ok: true }))
}

/// 以流式方式返回文件内容。供本人下载与公开下载共用。
pub async fn stream_file(
    state: &AppState,
    owner_id: &str,
    entry: &FileEntry,
) -> AppResult<Response> {
    if entry.is_folder() {
        return Err(AppError::BadRequest("资料夹不能下载".into()));
    }

    let data_root = state.drive_manager.require_data_root().await?;
    let path = path_resolver::file_path(&data_root, owner_id, &entry.id);

    let file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| AppError::NotFound)?;
    let metadata = file.metadata().await?;

    let stream = tokio_util::io::ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let mime = entry
        .mime_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let disposition = format!("attachment; filename*=UTF-8''{}", urlencode(&entry.name));

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, mime)
        .header(CONTENT_LENGTH, metadata.len())
        .header(CONTENT_DISPOSITION, disposition)
        .body(body)
        .map_err(|e| AppError::Other(anyhow::anyhow!("构造下载响应失败: {e}")))?;

    Ok(response.into_response())
}

/// 最小 URL 编码，用于 Content-Disposition 的 filename*。
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
