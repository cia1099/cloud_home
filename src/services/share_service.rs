//! 分享链接业务逻辑。

use chrono::{DateTime, Utc};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::file::FileEntry;
use crate::models::share::{CreateShareDto, ShareLink};
use crate::services::file_service;
use crate::util::{generate_share_token, now_rfc3339};

/// 创建分享链接（校验文件属于本人）。
pub async fn create(db: &Db, owner_id: &str, dto: &CreateShareDto) -> AppResult<ShareLink> {
    // 确认文件存在且属于本人。
    file_service::get_owned(db, owner_id, &dto.file_id).await?;

    // 校验 expires_at 格式（若提供）。
    if let Some(exp) = &dto.expires_at {
        parse_rfc3339(exp)?;
    }

    let id = uuid::Uuid::new_v4().to_string();
    let token = generate_share_token();

    sqlx::query(
        "INSERT INTO shares (id, file_id, owner_id, token, can_download, expires_at, access_count, created_at, is_active) \
         VALUES (?, ?, ?, ?, ?, ?, 0, ?, 1)",
    )
    .bind(&id)
    .bind(&dto.file_id)
    .bind(owner_id)
    .bind(&token)
    .bind(dto.can_download as i64)
    .bind(dto.expires_at.as_deref())
    .bind(now_rfc3339())
    .execute(db)
    .await?;

    fetch_by_id(db, owner_id, &id).await
}

/// 列出用户创建的所有分享链接。
pub async fn list(db: &Db, owner_id: &str) -> AppResult<Vec<ShareLink>> {
    let rows = sqlx::query_as::<_, ShareLink>(
        "SELECT * FROM shares WHERE owner_id = ? ORDER BY created_at DESC",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// 撤销分享链接（设 is_active = 0）。
pub async fn revoke(db: &Db, owner_id: &str, id: &str) -> AppResult<()> {
    let result = sqlx::query("UPDATE shares SET is_active = 0 WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

/// 通过 token 解析有效分享，返回 (分享, 文件, 分享者用户名)。
///
/// - 不存在或已撤销：404
/// - 已过期：410 Gone
pub async fn resolve_public(db: &Db, token: &str) -> AppResult<(ShareLink, FileEntry, String)> {
    let share = sqlx::query_as::<_, ShareLink>(
        "SELECT * FROM shares WHERE token = ? AND is_active = 1",
    )
    .bind(token)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    if let Some(exp) = &share.expires_at {
        let exp_dt = parse_rfc3339(exp)?;
        if exp_dt <= Utc::now() {
            return Err(AppError::Gone);
        }
    }

    // 文件可能已被删除。
    let file = sqlx::query_as::<_, FileEntry>(
        "SELECT * FROM files WHERE id = ? AND is_deleted = 0",
    )
    .bind(&share.file_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    let (username,): (String,) = sqlx::query_as("SELECT username FROM users WHERE id = ?")
        .bind(&share.owner_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)?;

    Ok((share, file, username))
}

/// 记录一次公开访问，access_count + 1。
pub async fn record_access(db: &Db, share_id: &str) -> AppResult<()> {
    sqlx::query("UPDATE shares SET access_count = access_count + 1 WHERE id = ?")
        .bind(share_id)
        .execute(db)
        .await?;
    Ok(())
}

async fn fetch_by_id(db: &Db, owner_id: &str, id: &str) -> AppResult<ShareLink> {
    sqlx::query_as::<_, ShareLink>("SELECT * FROM shares WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(owner_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
}

fn parse_rfc3339(s: &str) -> AppResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::BadRequest("expires_at 必须是 RFC3339 时间格式".into()))
}
