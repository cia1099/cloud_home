//! 分享链接业务逻辑。

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::file::FileEntry;
use crate::models::share::{CreateShareDto, PersonRef, ReceivedShare, ReceivedShareRow, ShareLink};
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

    let recipients = validate_recipients(db, owner_id, &dto.recipient_ids).await?;

    let id = uuid::Uuid::new_v4().to_string();
    let token = generate_share_token();
    let now = now_rfc3339();

    let mut tx = db.begin().await?;
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
    .bind(&now)
    .execute(&mut *tx)
    .await?;
    for user_id in &recipients {
        insert_recipient(&mut tx, &id, user_id, &now).await?;
    }
    tx.commit().await?;

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

/// 撤销分享（设 is_active = 0），返回需要被通知的收件人 id（公开链接为空）。
pub async fn revoke(db: &Db, owner_id: &str, id: &str) -> AppResult<Vec<String>> {
    let recipients = recipient_ids(db, owner_id, id).await?;
    let result = sqlx::query("UPDATE shares SET is_active = 0 WHERE id = ? AND owner_id = ?")
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(recipients)
}

/// 通过 token 解析有效分享，返回 (分享, 文件, 分享者用户名)。
///
/// - 不存在或已撤销：404
/// - 已过期：410 Gone
pub async fn resolve_public(db: &Db, token: &str) -> AppResult<(ShareLink, FileEntry, String)> {
    let share = sqlx::query_as::<_, ShareLink>(
        // 定向分享（有收件人行）不对外提供公开访问。
        "SELECT * FROM shares WHERE token = ? AND is_active = 1 \
         AND NOT EXISTS (SELECT 1 FROM share_recipients r WHERE r.share_id = shares.id)",
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

/// 校验收件人：必须是已批准、启用、未删除且不是分享者本人的账户；去重后返回。
/// 任一不合格就整体拒绝，不透露具体是哪一个及原因。
async fn validate_recipients(db: &Db, owner_id: &str, ids: &[String]) -> AppResult<Vec<String>> {
    let mut unique: Vec<String> = Vec::new();
    for id in ids {
        if !unique.contains(id) {
            unique.push(id.clone());
        }
    }
    for id in &unique {
        let ok: Option<(i64,)> = sqlx::query_as(
            "SELECT 1 FROM users WHERE id = ? AND id != ? AND is_active = 1 \
             AND deleted_at IS NULL AND approval_status = 'approved'",
        )
        .bind(id)
        .bind(owner_id)
        .fetch_optional(db)
        .await?;
        if ok.is_none() {
            return Err(AppError::BadRequest("recipient_ids 含无效的收件人".into()));
        }
    }
    Ok(unique)
}

async fn insert_recipient(
    tx: &mut sqlx::SqliteConnection,
    share_id: &str,
    user_id: &str,
    now: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO share_recipients (share_id, user_id, created_at) VALUES (?, ?, ?)")
        .bind(share_id)
        .bind(user_id)
        .bind(now)
        .execute(tx)
        .await?;
    Ok(())
}

/// 某条分享当前的收件人 id（强制按分享者过滤：不是自己的分享返回空）。
async fn recipient_ids(db: &Db, owner_id: &str, share_id: &str) -> AppResult<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT r.user_id FROM share_recipients r JOIN shares s ON s.id = r.share_id \
         WHERE r.share_id = ? AND s.owner_id = ? ORDER BY r.created_at, r.user_id",
    )
    .bind(share_id)
    .bind(owner_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// 分享者名下全部分享的收件人，按分享 id 归组。
pub async fn recipients_by_share(
    db: &Db,
    owner_id: &str,
) -> AppResult<HashMap<String, Vec<PersonRef>>> {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT r.share_id, u.id, u.username FROM share_recipients r \
         JOIN shares s ON s.id = r.share_id JOIN users u ON u.id = r.user_id \
         WHERE s.owner_id = ? ORDER BY r.created_at, u.username",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await?;
    let mut map: HashMap<String, Vec<PersonRef>> = HashMap::new();
    for (share_id, id, display_name) in rows {
        map.entry(share_id)
            .or_default()
            .push(PersonRef { id, display_name });
    }
    Ok(map)
}

/// 整组替换定向分享的收件人，返回 (新增, 移除) 的用户 id 供通知。
///
/// 只对本人名下、仍有效、且本来就是定向分享的记录生效；不能清空收件人，
/// 否则这条分享会悄悄变回没有收件人限制的状态——要停止分享请撤销。
pub async fn update_recipients(
    db: &Db,
    owner_id: &str,
    share_id: &str,
    ids: &[String],
) -> AppResult<(ShareLink, Vec<String>, Vec<String>)> {
    let share = fetch_by_id(db, owner_id, share_id).await?;
    if share.is_active == 0 {
        return Err(AppError::NotFound);
    }
    let current = recipient_ids(db, owner_id, share_id).await?;
    if current.is_empty() {
        return Err(AppError::BadRequest("公开链接没有收件人，不能修改".into()));
    }
    if ids.is_empty() {
        return Err(AppError::BadRequest("recipient_ids 不能为空，要停止分享请撤销".into()));
    }
    let wanted = validate_recipients(db, owner_id, ids).await?;
    let added: Vec<String> = wanted.iter().filter(|u| !current.contains(u)).cloned().collect();
    let removed: Vec<String> = current.iter().filter(|u| !wanted.contains(u)).cloned().collect();

    let now = now_rfc3339();
    let mut tx = db.begin().await?;
    for user_id in &removed {
        sqlx::query("DELETE FROM share_recipients WHERE share_id = ? AND user_id = ?")
            .bind(share_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await?;
    }
    for user_id in &added {
        insert_recipient(&mut tx, share_id, user_id, &now).await?;
    }
    tx.commit().await?;
    Ok((share, added, removed))
}

/// 人员目录：除自己外所有已批准、启用的账户，只暴露 id 与显示名（不含邮箱、手机、管理员标记）。
pub async fn directory(db: &Db, user_id: &str) -> AppResult<Vec<PersonRef>> {
    let rows = sqlx::query_as::<_, PersonRef>(
        "SELECT id, username AS display_name FROM users \
         WHERE id != ? AND is_active = 1 AND deleted_at IS NULL AND approval_status = 'approved' \
         ORDER BY username COLLATE NOCASE, id",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// 分享是否已过期（过期时间无法解析时按已过期处理，宁可拒绝）。
fn is_expired(expires_at: Option<&str>) -> bool {
    expires_at.is_some_and(|exp| parse_rfc3339(exp).map(|dt| dt <= Utc::now()).unwrap_or(true))
}

/// 列出分享给我的、仍有效的分享（最新在前）。
pub async fn list_received(db: &Db, user_id: &str) -> AppResult<Vec<ReceivedShare>> {
    let rows = sqlx::query_as::<_, ReceivedShareRow>(
        "SELECT s.id, s.file_id, f.name, f.file_type, f.size_bytes, f.mime_type, \
                (SELECT COUNT(*) FROM files c WHERE c.parent_id = f.id AND c.is_deleted = 0) AS child_count, \
                s.owner_id, o.username AS owner_name, s.created_at, s.expires_at, s.can_download, r.seen_at \
         FROM share_recipients r \
         JOIN shares s ON s.id = r.share_id AND s.is_active = 1 \
         JOIN files f ON f.id = s.file_id AND f.is_deleted = 0 \
         JOIN users o ON o.id = s.owner_id AND o.is_active = 1 \
         WHERE r.user_id = ? ORDER BY s.created_at DESC, s.id",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|r| !is_expired(r.expires_at.as_deref()))
        .map(ReceivedShare::from)
        .collect())
}

/// 收件人访问被分享的文件（`file_id` 为 None 即被分享项目本身）：校验他确实是这条分享的收件人、分享有效，
/// 且 `file_id` 就是被分享的项目或其未删除的后代。任何一项不满足都是 404
/// （不区分「不存在」与「无权」，避免探测）。返回的文件按分享者名下读取，
/// 调用方用 `share.owner_id` 定位物理文件。
pub async fn resolve_received(
    db: &Db,
    user_id: &str,
    share_id: &str,
    file_id: Option<&str>,
) -> AppResult<(ShareLink, FileEntry)> {
    let share = sqlx::query_as::<_, ShareLink>(
        "SELECT s.* FROM shares s JOIN share_recipients r ON r.share_id = s.id \
         WHERE s.id = ? AND r.user_id = ? AND s.is_active = 1",
    )
    .bind(share_id)
    .bind(user_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;
    if is_expired(share.expires_at.as_deref()) {
        return Err(AppError::NotFound);
    }

    // 省略 file_id 表示被分享的项目本身。
    let file_id = file_id.unwrap_or(&share.file_id);

    // 从目标文件沿父链向上（只看分享者名下、未删除的节点），必须经过被分享的项目。
    let (hit,): (i64,) = sqlx::query_as(
        "WITH RECURSIVE anc(id, parent_id) AS ( \
             SELECT id, parent_id FROM files WHERE id = ?1 AND owner_id = ?2 AND is_deleted = 0 \
             UNION ALL \
             SELECT f.id, f.parent_id FROM files f JOIN anc a ON f.id = a.parent_id \
             WHERE f.owner_id = ?2 AND f.is_deleted = 0 \
         ) SELECT COUNT(*) FROM anc WHERE id = ?3",
    )
    .bind(file_id)
    .bind(&share.owner_id)
    .bind(&share.file_id)
    .fetch_one(db)
    .await?;
    if hit == 0 {
        return Err(AppError::NotFound);
    }
    let file = file_service::get_owned(db, &share.owner_id, file_id).await?;
    Ok((share, file))
}

/// 收件人首次打开：把 `seen_at` 记为现在（已记过则不变）。不是收件人返回 404。
pub async fn mark_seen(db: &Db, user_id: &str, share_id: &str) -> AppResult<()> {
    let (exists,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM share_recipients WHERE share_id = ? AND user_id = ?")
            .bind(share_id)
            .bind(user_id)
            .fetch_one(db)
            .await?;
    if exists == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "UPDATE share_recipients SET seen_at = ? WHERE share_id = ? AND user_id = ? AND seen_at IS NULL",
    )
    .bind(now_rfc3339())
    .bind(share_id)
    .bind(user_id)
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
