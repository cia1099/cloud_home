//! 搜索端点：按名称模糊匹配当前用户的文件与资料夹。

use axum::extract::{Query, State};
use axum::{Json, response::Json as JsonResp};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::AuthUser;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::file::FileEntry;
use crate::services::file_service;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(rename = "type")]
    pub type_filter: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// GET /search
pub async fn search(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<SearchQuery>,
) -> AppResult<JsonResp<Value>> {
    let q = params.q.trim();
    if q.is_empty() || q.len() > 100 {
        return Err(AppError::BadRequest("q 长度需在 1..=100 之间".into()));
    }
    if let Some(t) = &params.type_filter
        && !file_service::is_valid_type_filter(t)
    {
        return Err(AppError::BadRequest("type 必须是 file 或 folder".into()));
    }

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let pattern = format!("%{q}%");

    // 统计总数。
    let total = count_matches(&state.db, &user.user_id, &pattern, params.type_filter.as_deref())
        .await?;

    // 取当前页。
    let rows = fetch_matches(
        &state.db,
        &user.user_id,
        &pattern,
        params.type_filter.as_deref(),
        per_page,
        offset,
    )
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for entry in rows {
        let parent_path = build_parent_path(&state.db, &user.user_id, entry.parent_id.as_deref())
            .await?;
        items.push(json!({
            "id": entry.id,
            "name": entry.name,
            "file_type": entry.file_type,
            "size_bytes": entry.size_bytes,
            "mime_type": entry.mime_type,
            "parent_id": entry.parent_id,
            "parent_path": parent_path,
            "created_at": entry.created_at,
            "updated_at": entry.updated_at,
        }));
    }

    Ok(Json(json!({
        "data": {
            "query": q,
            "items": items,
            "pagination": { "page": page, "per_page": per_page, "total": total }
        }
    })))
}

async fn count_matches(
    db: &Db,
    owner_id: &str,
    pattern: &str,
    type_filter: Option<&str>,
) -> AppResult<i64> {
    let (count,): (i64,) = match type_filter {
        Some(t) => sqlx::query_as(
            "SELECT COUNT(*) FROM files WHERE owner_id = ? AND is_deleted = 0 \
             AND name LIKE ? AND file_type = ?",
        )
        .bind(owner_id)
        .bind(pattern)
        .bind(t)
        .fetch_one(db)
        .await?,
        None => sqlx::query_as(
            "SELECT COUNT(*) FROM files WHERE owner_id = ? AND is_deleted = 0 AND name LIKE ?",
        )
        .bind(owner_id)
        .bind(pattern)
        .fetch_one(db)
        .await?,
    };
    Ok(count)
}

async fn fetch_matches(
    db: &Db,
    owner_id: &str,
    pattern: &str,
    type_filter: Option<&str>,
    per_page: i64,
    offset: i64,
) -> AppResult<Vec<FileEntry>> {
    let rows = match type_filter {
        Some(t) => sqlx::query_as::<_, FileEntry>(
            "SELECT * FROM files WHERE owner_id = ? AND is_deleted = 0 AND name LIKE ? \
             AND file_type = ? ORDER BY name ASC LIMIT ? OFFSET ?",
        )
        .bind(owner_id)
        .bind(pattern)
        .bind(t)
        .bind(per_page)
        .bind(offset)
        .fetch_all(db)
        .await?,
        None => sqlx::query_as::<_, FileEntry>(
            "SELECT * FROM files WHERE owner_id = ? AND is_deleted = 0 AND name LIKE ? \
             ORDER BY name ASC LIMIT ? OFFSET ?",
        )
        .bind(owner_id)
        .bind(pattern)
        .bind(per_page)
        .bind(offset)
        .fetch_all(db)
        .await?,
    };
    Ok(rows)
}

/// 由 parent_id 向上拼出可读路径，如 `Documents/2025`。根目录返回空字符串。
async fn build_parent_path(
    db: &Db,
    owner_id: &str,
    parent_id: Option<&str>,
) -> AppResult<String> {
    let mut segments: Vec<String> = Vec::new();
    let mut current = parent_id.map(|s| s.to_string());
    // 限制深度，防御异常数据形成的环。
    let mut guard = 0;
    while let Some(id) = current {
        guard += 1;
        if guard > 256 {
            break;
        }
        let row: Option<(String, Option<String>)> =
            sqlx::query_as("SELECT name, parent_id FROM files WHERE id = ? AND owner_id = ?")
                .bind(&id)
                .bind(owner_id)
                .fetch_optional(db)
                .await?;
        match row {
            Some((name, parent)) => {
                segments.push(name);
                current = parent;
            }
            None => break,
        }
    }
    segments.reverse();
    Ok(segments.join("/"))
}
