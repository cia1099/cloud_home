//! 文件操作端点。

use axum::body::Body;
use axum::extract::{Multipart, Path as AxumPath, Query, State};
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE,
    RANGE,
};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

use crate::auth::AuthUser;
use crate::drive::path_resolver;
use crate::error::{AppError, AppResult, ErrorResponse};
use crate::models::common::OkResponse;
use crate::models::event::{ChangeKind, FileChangeEvent};
use crate::models::file::{
    FILE_TYPE_FILE, FileEntry, FileListResponse, FileResponse, ListQuery, MoveDto, RenameDto,
    ThumbnailQuery,
};
use crate::openapi::ApiResponse;
use crate::services::{file_service, thumbnail_service, trash_service};
use crate::state::AppState;

/// `Content-Disposition` 类型：`download` 强制另存，`raw` 内联预览。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Inline,
    Attachment,
}

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
        Ok(entry) => {
            let response = FileResponse::from(entry);
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

    let mime = file_service::guess_mime(&name);
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

/// GET /files/:id/download  强制另存（`Content-Disposition: attachment`）。
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
             ("Accept-Ranges" = String, description = "bytes"),
         )),
        (status = 206, description = "部分内容（响应 Range 请求）", content_type = "application/octet-stream", body = [u8],
         headers(("Content-Range" = String, description = "bytes {start}-{end}/{total}"))),
        (status = 400, description = "资料夹不能下载", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
        (status = 416, description = "Range 不满足"),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn download(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    let range = range_header(&headers);
    stream_file(
        &state,
        &user.user_id,
        &entry,
        Disposition::Attachment,
        range,
    )
    .await
}

/// GET /files/:id/raw  内联预览（`Content-Disposition: inline`），支持 HTTP Range，
/// 用于图片/音频/视频在前端直接渲染（`<img>` / `<audio>` / `<video>`）而非触发下载。
///
/// 音视频播放正是依赖此处已有的两个特性：
/// - 单段 Range（`bytes=start-end` / `start-` / `-suffix`），支持拖动进度条与
///   Safari 探测式的 `bytes=0-1` 首次请求；
/// - 流式响应体，播放大文件不会把整个文件读入内存。
///
/// 两端调用方式：
/// - **Web**：`<video controls src="/api/v1/files/{id}/raw">` /
///   `<audio controls src="...">`，同源请求会自动携带 `auth_token` Cookie；
/// - **Flutter**：`video_player` 的 `VideoPlayerController.networkUrl(uri,
///   httpHeaders: {'Authorization': 'Bearer <jwt>'})`，或 `just_audio` 的
///   `AudioSource.uri(uri, headers: {...})`。
///
/// 鉴权与其他端点一致（Cookie 优先，回退 Bearer）——刻意不做成公开端点：
/// 文件 id 一旦出现在 `<img src>` 中就可能通过浏览器历史/Referer/日志泄露，
/// 公开等同于签发一个永不过期、不可撤销的访问凭证，会破坏多账户隔离。
/// 真正需要「任何人可查看」的场景请使用 `/public/shares/{token}`
/// （该端点当前仅支持下载，暂不支持内联播放）。
#[utoipa::path(
    get,
    path = "/files/{id}/raw",
    tag = "files",
    params(("id" = String, Path, description = "文件 id")),
    responses(
        (status = 200, description = "文件内容（内联）", content_type = "application/octet-stream", body = [u8],
         headers(("Accept-Ranges" = String, description = "bytes"))),
        (status = 206, description = "部分内容（响应 Range 请求，用于视频拖动/大图渐进加载）",
         content_type = "application/octet-stream", body = [u8],
         headers(("Content-Range" = String, description = "bytes {start}-{end}/{total}"))),
        (status = 400, description = "资料夹不能预览", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
        (status = 416, description = "Range 不满足"),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn raw(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    let range = range_header(&headers);
    stream_file(&state, &user.user_id, &entry, Disposition::Inline, range).await
}

/// GET /files/:id/thumbnail?size=256  图片缩略图（JPEG，磁盘缓存），用于画廊网格。
#[utoipa::path(
    get,
    path = "/files/{id}/thumbnail",
    tag = "files",
    params(("id" = String, Path, description = "文件 id"), ThumbnailQuery),
    responses(
        (status = 200, description = "缩略图", content_type = "image/jpeg", body = [u8]),
        (status = 400, description = "非图片文件或 size 不在允许范围内", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "文件不存在", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn thumbnail(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(id): AxumPath<String>,
    Query(q): Query<ThumbnailQuery>,
) -> AppResult<Response> {
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;
    let data_root = state.drive_manager.require_data_root().await?;
    let bytes = thumbnail_service::get_or_create(&data_root, &user.user_id, &entry, q.size).await?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "image/jpeg")
        .header(CONTENT_LENGTH, bytes.len())
        .header(CONTENT_DISPOSITION, "inline")
        .header(CACHE_CONTROL, "private, max-age=31536000, immutable")
        .body(Body::from(bytes))
        .map_err(|e| AppError::Other(anyhow::anyhow!("构造响应失败: {e}")))?;
    Ok(response.into_response())
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
    let response = FileResponse::from(entry);
    state.events.notify(
        &user.user_id,
        FileChangeEvent {
            kind: ChangeKind::Updated,
            parent_ids: vec![response.parent_id.clone()],
            file: Some(response.clone()),
        },
    );
    Ok(ApiResponse::new(response))
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
    let old_parent_id = file_service::get_owned(&state.db, &user.user_id, &id)
        .await?
        .parent_id;

    let entry = file_service::move_entry(
        &state.db,
        &user.user_id,
        &id,
        dto.target_parent_id.as_deref(),
    )
    .await?;
    let response = FileResponse::from(entry);

    let mut parent_ids = vec![old_parent_id, response.parent_id.clone()];
    parent_ids.dedup();
    state.events.notify(
        &user.user_id,
        FileChangeEvent {
            kind: ChangeKind::Updated,
            parent_ids,
            file: Some(response.clone()),
        },
    );
    Ok(ApiResponse::new(response))
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
    let entry = file_service::get_owned(&state.db, &user.user_id, &id).await?;

    trash_service::soft_delete(
        &state.db,
        state.config.trash_retention_days,
        &user.user_id,
        &id,
    )
    .await?;

    let response = FileResponse::from(entry);
    state.events.notify(
        &user.user_id,
        FileChangeEvent {
            kind: ChangeKind::Deleted,
            parent_ids: vec![response.parent_id.clone()],
            file: Some(response),
        },
    );
    Ok(ApiResponse::new(OkResponse { ok: true }))
}

/// POST /files/download  批量打包下载（流式归档，ZIP 或 tar.gz，前端通过 `format` 字段选择）。
///
/// `ids` 可混合文件与资料夹 id，资料夹会递归展开为其全部子孙文件；
/// 每个 id 均按当前用户强制账户隔离校验，任一不存在或非本人所有则整体 404。
/// 响应体为流式生成（边打包边发送），不预先缓冲整个归档，内存占用恒定。
#[utoipa::path(
    post,
    path = "/files/download",
    tag = "files",
    request_body = crate::models::file::DownloadRequest,
    responses(
        (status = 200, description = "归档文件（流式；ZIP 或 tar.gz，取决于请求体 format 字段，默认 ZIP）",
         content_type = "application/octet-stream", body = [u8],
         headers(("Content-Disposition" = String, description = "attachment; filename=\"cloud_home_download.zip|tar.gz\""))),
        (status = 400, description = "ids 为空或未选中任何可下载文件", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "存在不属于本人或不存在的 id", body = ErrorResponse),
        (status = 503, description = "外接硬盘不可用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn download_archive(
    State(state): State<AppState>,
    user: AuthUser,
    axum::Json(req): axum::Json<crate::models::file::DownloadRequest>,
) -> AppResult<Response> {
    if req.ids.is_empty() {
        return Err(AppError::BadRequest("ids 不能为空".into()));
    }

    let data_root = state.drive_manager.require_data_root().await?;
    let entries =
        file_service::collect_download_entries(&state.db, &data_root, &user.user_id, &req.ids)
            .await?;
    if entries.is_empty() {
        return Err(AppError::BadRequest("未选中任何可下载文件".into()));
    }

    // 校验/展开全部在流开始前完成——响应一旦开始流式输出就无法再改变 HTTP 状态码。
    let format = req.format;
    let (reader, writer) = tokio::io::duplex(64 * 1024);
    tokio::spawn(crate::services::archive_service::write_archive(
        format, writer, entries,
    ));

    let stream = tokio_util::io::ReaderStream::new(reader);
    let body = Body::from_stream(stream);

    let filename = format!("cloud_home_download{}", format.extension());
    let response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, format.content_type())
        .header(
            CONTENT_DISPOSITION,
            format!("attachment; filename=\"{filename}\""),
        )
        .body(body)
        .map_err(|e| AppError::Other(anyhow::anyhow!("构造响应失败: {e}")))?;

    Ok(response.into_response())
}

/// 从请求头中取出 `Range` 值（若存在）。
fn range_header(headers: &HeaderMap) -> Option<&str> {
    headers.get(RANGE).and_then(|v| v.to_str().ok())
}

/// 单段 `Range` 请求头的解析结果。
#[derive(Debug, PartialEq)]
enum RangeResult {
    /// 无 Range 或无法解析（含多段 Range，本实现不支持）——返回完整内容。
    Full,
    /// 合法单段范围（含头尾字节，闭区间）。
    Partial(u64, u64),
    /// Range 请求的起始位置超出文件长度，无法满足。
    Unsatisfiable,
}

/// 解析形如 `bytes=200-1000` / `bytes=200-` / `bytes=-500` 的单段 Range 请求头。
/// 仅支持单段——含逗号的多段请求视为不满足解析条件，退回完整内容（对图片/视频场景足够）。
fn resolve_range(header: Option<&str>, file_len: u64) -> RangeResult {
    let Some(h) = header else {
        return RangeResult::Full;
    };
    let Some(spec) = h.strip_prefix("bytes=") else {
        return RangeResult::Full;
    };
    if spec.contains(',') {
        return RangeResult::Full;
    }
    let Some((start_s, end_s)) = spec.split_once('-') else {
        return RangeResult::Full;
    };

    if start_s.is_empty() {
        // 后缀范围：bytes=-500 表示最后 500 字节。
        let Ok(suffix) = end_s.parse::<u64>() else {
            return RangeResult::Full;
        };
        if suffix == 0 {
            return RangeResult::Full;
        }
        if file_len == 0 {
            return RangeResult::Unsatisfiable;
        }
        let start = file_len.saturating_sub(suffix);
        return RangeResult::Partial(start, file_len - 1);
    }

    let Ok(start) = start_s.parse::<u64>() else {
        return RangeResult::Full;
    };
    if start >= file_len {
        return RangeResult::Unsatisfiable;
    }
    let end = if end_s.is_empty() {
        file_len - 1
    } else {
        match end_s.parse::<u64>() {
            Ok(e) => e.min(file_len - 1),
            Err(_) => return RangeResult::Full,
        }
    };
    if end < start {
        return RangeResult::Full;
    }
    RangeResult::Partial(start, end)
}

/// 以流式方式返回文件内容，支持内联/附件两种 disposition 与 HTTP Range 部分请求。
/// 供本人下载（`download`/`raw`）与公开下载共用。
pub async fn stream_file(
    state: &AppState,
    owner_id: &str,
    entry: &FileEntry,
    disposition: Disposition,
    range: Option<&str>,
) -> AppResult<Response> {
    if entry.is_folder() {
        return Err(AppError::BadRequest("资料夹不能下载".into()));
    }

    let data_root = state.drive_manager.require_data_root().await?;
    let path = path_resolver::file_path(&data_root, owner_id, &entry.id);

    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| AppError::NotFound)?;
    let metadata = file.metadata().await?;
    let file_len = metadata.len();

    let mime = entry
        .mime_type
        .clone()
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let disposition_kind = match disposition {
        Disposition::Inline => "inline",
        Disposition::Attachment => "attachment",
    };
    let disposition_value = format!(
        "{disposition_kind}; filename*=UTF-8''{}",
        urlencode(&entry.name)
    );

    match resolve_range(range, file_len) {
        RangeResult::Unsatisfiable => {
            let response = Response::builder()
                .status(StatusCode::RANGE_NOT_SATISFIABLE)
                .header(CONTENT_RANGE, format!("bytes */{file_len}"))
                .body(Body::empty())
                .map_err(|e| AppError::Other(anyhow::anyhow!("构造响应失败: {e}")))?;
            Ok(response.into_response())
        }
        RangeResult::Full => {
            let stream = tokio_util::io::ReaderStream::new(file);
            let body = Body::from_stream(stream);
            let response = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_LENGTH, file_len)
                .header(CONTENT_DISPOSITION, disposition_value)
                .header(ACCEPT_RANGES, "bytes")
                .body(body)
                .map_err(|e| AppError::Other(anyhow::anyhow!("构造响应失败: {e}")))?;
            Ok(response.into_response())
        }
        RangeResult::Partial(start, end) => {
            file.seek(std::io::SeekFrom::Start(start)).await?;
            let len = end - start + 1;
            let limited = file.take(len);
            let stream = tokio_util::io::ReaderStream::new(limited);
            let body = Body::from_stream(stream);
            let response = Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header(CONTENT_TYPE, mime)
                .header(CONTENT_LENGTH, len)
                .header(CONTENT_DISPOSITION, disposition_value)
                .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_len}"))
                .header(ACCEPT_RANGES, "bytes")
                .body(body)
                .map_err(|e| AppError::Other(anyhow::anyhow!("构造响应失败: {e}")))?;
            Ok(response.into_response())
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::file_service::guess_mime;

    const LEN: u64 = 1000;

    #[test]
    fn no_range_header_is_full() {
        assert_eq!(resolve_range(None, LEN), RangeResult::Full);
    }

    #[test]
    fn non_bytes_unit_is_full() {
        assert_eq!(resolve_range(Some("items=0-1"), LEN), RangeResult::Full);
    }

    #[test]
    fn multi_range_is_full() {
        assert_eq!(
            resolve_range(Some("bytes=0-1,5-6"), LEN),
            RangeResult::Full
        );
    }

    #[test]
    fn safari_probe_bytes_0_1() {
        assert_eq!(
            resolve_range(Some("bytes=0-1"), LEN),
            RangeResult::Partial(0, 1)
        );
    }

    #[test]
    fn chrome_initial_bytes_0_dash() {
        assert_eq!(
            resolve_range(Some("bytes=0-"), LEN),
            RangeResult::Partial(0, LEN - 1)
        );
    }

    #[test]
    fn seek_to_offset() {
        assert_eq!(
            resolve_range(Some("bytes=500-"), LEN),
            RangeResult::Partial(500, LEN - 1)
        );
    }

    #[test]
    fn end_past_eof_is_clamped() {
        assert_eq!(
            resolve_range(Some("bytes=0-999999999"), LEN),
            RangeResult::Partial(0, LEN - 1)
        );
    }

    #[test]
    fn suffix_range() {
        assert_eq!(
            resolve_range(Some("bytes=-500"), LEN),
            RangeResult::Partial(LEN - 500, LEN - 1)
        );
    }

    #[test]
    fn suffix_larger_than_file_is_whole_file() {
        assert_eq!(
            resolve_range(Some("bytes=-5000"), LEN),
            RangeResult::Partial(0, LEN - 1)
        );
    }

    #[test]
    fn start_at_eof_is_unsatisfiable() {
        assert_eq!(
            resolve_range(Some("bytes=1000-"), LEN),
            RangeResult::Unsatisfiable
        );
    }

    #[test]
    fn start_past_eof_is_unsatisfiable() {
        assert_eq!(
            resolve_range(Some("bytes=5000-"), LEN),
            RangeResult::Unsatisfiable
        );
    }

    #[test]
    fn any_range_on_empty_file_is_unsatisfiable() {
        assert_eq!(resolve_range(Some("bytes=0-"), 0), RangeResult::Unsatisfiable);
    }

    #[test]
    fn guess_mime_normalizes_apple_audio() {
        assert_eq!(guess_mime("voice.m4a").as_deref(), Some("audio/mp4"));
        assert_eq!(guess_mime("VOICE.M4A").as_deref(), Some("audio/mp4"));
    }

    #[test]
    fn guess_mime_normalizes_apple_video() {
        assert_eq!(guess_mime("clip.m4v").as_deref(), Some("video/mp4"));
    }

    #[test]
    fn guess_mime_passes_through_standard_types() {
        assert_eq!(guess_mime("movie.mp4").as_deref(), Some("video/mp4"));
        assert_eq!(guess_mime("song.mp3").as_deref(), Some("audio/mpeg"));
    }

    #[test]
    fn guess_mime_none_without_extension() {
        assert_eq!(guess_mime("README"), None);
    }
}
