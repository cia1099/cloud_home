//! 文件操作端点。

use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use axum::{Json, response::Json as JsonResp};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use crate::auth::AuthUser;
use crate::drive::path_resolver;
use crate::error::{AppError, AppResult};
use crate::models::file::{
    FILE_TYPE_FILE, FileEntry, FileResponse, ListQuery, MoveDto, RenameDto,
};
use crate::services::{file_service, trash_service};
use crate::state::AppState;

/// GET /files?parent_id=
pub async fn list(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListQuery>,
) -> AppResult<JsonResp<Value>> {
    let items = file_service::list_dir(&state.db, &user.user_id, q.parent_id.as_deref()).await?;
    let data: Vec<FileResponse> = items.into_iter().map(FileResponse::from).collect();
    Ok(Json(json!({ "data": { "items": data } })))
}

/// GET /files/:id
pub async fn get_metadata(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    Ok(Json(json!({ "data": FileResponse::from(entry) })))
}

/// POST /files/upload （multipart/form-data）
pub async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, JsonResp<Value>)> {
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
        Ok(entry) => Ok((StatusCode::CREATED, Json(json!({ "data": FileResponse::from(entry) })))),
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
pub async fn download(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<Response> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    stream_file(&state, &user.user_id, &entry).await
}

/// PATCH /files/:id  重命名
pub async fn rename(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    Json(dto): Json<RenameDto>,
) -> AppResult<JsonResp<Value>> {
    let entry = file_service::rename(&state.db, &user.user_id, &id, &dto.name).await?;
    Ok(Json(json!({ "data": FileResponse::from(entry) })))
}

/// POST /files/:id/move
pub async fn move_file(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    Json(dto): Json<MoveDto>,
) -> AppResult<JsonResp<Value>> {
    let entry =
        file_service::move_entry(&state.db, &user.user_id, &id, dto.target_parent_id.as_deref())
            .await?;
    Ok(Json(json!({ "data": FileResponse::from(entry) })))
}

/// DELETE /files/:id  软删除（移入回收站）
pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
) -> AppResult<JsonResp<Value>> {
    trash_service::soft_delete(
        &state.db,
        state.config.trash_retention_days,
        &user.user_id,
        &id,
    )
    .await?;
    Ok(Json(json!({ "data": { "ok": true } })))
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
    let disposition = format!(
        "attachment; filename*=UTF-8''{}",
        urlencode(&entry.name)
    );

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
