//! 回收站业务逻辑：软删除、恢复、永久删除、清理。

use std::path::Path;

use chrono::{Duration, Utc};

use crate::db::Db;
use crate::drive::path_resolver;
use crate::error::{AppError, AppResult};
use crate::models::file::FileEntry;
use crate::models::trash::TrashItem;
use crate::services::file_service;
use crate::util::now_rfc3339;

/// 软删除：将目标及其所有子孙标记为已删除，并为顶层项目建立一条回收站记录。
pub async fn soft_delete(
    db: &Db,
    retention_days: i64,
    owner_id: &str,
    id: &str,
) -> AppResult<()> {
    let entry = file_service::get_owned(db, owner_id, id).await?;
    let subtree = file_service::collect_subtree_ids(db, owner_id, id).await?;

    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let expires = (now + Duration::days(retention_days)).to_rfc3339();

    let mut tx = db.begin().await?;

    for fid in &subtree {
        sqlx::query("UPDATE files SET is_deleted = 1, updated_at = ? WHERE id = ? AND owner_id = ?")
            .bind(&now_str)
            .bind(fid)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
    }

    sqlx::query(
        "INSERT INTO trash (id, file_id, owner_id, original_parent_id, original_name, deleted_at, expires_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&entry.id)
    .bind(owner_id)
    .bind(entry.parent_id.as_deref())
    .bind(&entry.name)
    .bind(&now_str)
    .bind(&expires)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// 列出用户回收站内容。
pub async fn list(db: &Db, owner_id: &str) -> AppResult<Vec<TrashItem>> {
    let items = sqlx::query_as::<_, TrashItem>(
        "SELECT t.id AS id, t.file_id AS file_id, f.name AS name, f.file_type AS file_type, \
                f.size_bytes AS size_bytes, t.deleted_at AS deleted_at, t.expires_at AS expires_at \
         FROM trash t JOIN files f ON f.id = t.file_id \
         WHERE t.owner_id = ? ORDER BY t.deleted_at DESC",
    )
    .bind(owner_id)
    .fetch_all(db)
    .await?;
    Ok(items)
}

/// 恢复回收站中的项目。原目录已删则恢复到根目录。
pub async fn restore(db: &Db, owner_id: &str, trash_id: &str) -> AppResult<FileEntry> {
    let trash = fetch_trash(db, owner_id, trash_id).await?;

    // 决定恢复目标父目录：原父目录仍存在且未删除才使用，否则回到根目录。
    let restore_parent: Option<String> = match &trash.original_parent_id {
        Some(pid) => {
            let alive: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM files WHERE id = ? AND owner_id = ? AND is_deleted = 0",
            )
            .bind(pid)
            .bind(owner_id)
            .fetch_optional(db)
            .await?;
            alive.map(|(id,)| id)
        }
        None => None,
    };

    // 处理目标目录下的重名冲突。
    let mut name = trash.original_name.clone();
    if name_taken(db, owner_id, restore_parent.as_deref(), &name).await? {
        name = format!("{name} (已还原 {})", Utc::now().format("%Y%m%d%H%M%S"));
    }

    let subtree = file_service::collect_subtree_ids(db, owner_id, &trash.file_id).await?;
    let now = now_rfc3339();

    let mut tx = db.begin().await?;
    for fid in &subtree {
        sqlx::query("UPDATE files SET is_deleted = 0, updated_at = ? WHERE id = ? AND owner_id = ?")
            .bind(&now)
            .bind(fid)
            .bind(owner_id)
            .execute(&mut *tx)
            .await?;
    }
    // 顶层项目更新父目录与（可能调整后的）名称。
    sqlx::query("UPDATE files SET parent_id = ?, name = ?, updated_at = ? WHERE id = ? AND owner_id = ?")
        .bind(restore_parent.as_deref())
        .bind(&name)
        .bind(&now)
        .bind(&trash.file_id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM trash WHERE id = ? AND owner_id = ?")
        .bind(trash_id)
        .bind(owner_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    file_service::get_owned(db, owner_id, &trash.file_id).await
}

/// 立即永久删除单个回收站项目，返回释放的字节数。
pub async fn delete_permanent(
    db: &Db,
    data_root: &Path,
    owner_id: &str,
    trash_id: &str,
) -> AppResult<u64> {
    let trash = fetch_trash(db, owner_id, trash_id).await?;
    let (_, bytes) = purge_subtree(db, data_root, owner_id, &trash.file_id).await?;
    Ok(bytes)
}

/// 清空回收站，返回 (删除项目数, 释放字节数)。
pub async fn empty(db: &Db, data_root: &Path, owner_id: &str) -> AppResult<(u64, u64)> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT file_id FROM trash WHERE owner_id = ?")
        .bind(owner_id)
        .fetch_all(db)
        .await?;

    let mut total_bytes = 0u64;
    let count = rows.len() as u64;
    for (file_id,) in rows {
        let (_, bytes) = purge_subtree(db, data_root, owner_id, &file_id).await?;
        total_bytes += bytes;
    }
    Ok((count, total_bytes))
}

/// 全局清理所有已过期的回收站项目（供定时任务调用）。返回清理的项目数。
pub async fn cleanup_expired(db: &Db, data_root: &Path) -> AppResult<u64> {
    let now = now_rfc3339();
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT file_id, owner_id FROM trash WHERE expires_at <= ?")
            .bind(&now)
            .fetch_all(db)
            .await?;

    let count = rows.len() as u64;
    for (file_id, owner_id) in rows {
        if let Err(e) = purge_subtree(db, data_root, &owner_id, &file_id).await {
            tracing::warn!(file_id = %file_id, error = %e, "清理过期文件失败，跳过");
        }
    }
    Ok(count)
}

/// 物理删除某项目子树的所有文件并从 DB 移除（CASCADE 删除 trash 行）。
/// 返回 (删除文件数, 释放字节数)。物理删除失败仅告警，不中断。
async fn purge_subtree(
    db: &Db,
    data_root: &Path,
    owner_id: &str,
    root_id: &str,
) -> AppResult<(u64, u64)> {
    let subtree = file_service::collect_subtree_ids(db, owner_id, root_id).await?;

    let mut deleted_files = 0u64;
    let mut released = 0u64;

    for fid in &subtree {
        let row: Option<(String, i64)> = sqlx::query_as(
            "SELECT file_type, size_bytes FROM files WHERE id = ? AND owner_id = ?",
        )
        .bind(fid)
        .bind(owner_id)
        .fetch_optional(db)
        .await?;

        if let Some((file_type, size)) = row
            && file_type == crate::models::file::FILE_TYPE_FILE
        {
            let path = path_resolver::file_path(data_root, owner_id, fid);
            match tokio::fs::remove_file(&path).await {
                Ok(_) => {
                    deleted_files += 1;
                    released += size.max(0) as u64;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    tracing::warn!(path = %path.display(), error = %e, "删除物理文件失败");
                }
            }
        }
    }

    // 删除顶层行，子孙依赖 ON DELETE CASCADE (parent_id) 之外需显式删除；逐个删除更稳妥。
    for fid in &subtree {
        sqlx::query("DELETE FROM files WHERE id = ? AND owner_id = ?")
            .bind(fid)
            .bind(owner_id)
            .execute(db)
            .await?;
    }

    Ok((deleted_files, released))
}

async fn fetch_trash(
    db: &Db,
    owner_id: &str,
    trash_id: &str,
) -> AppResult<crate::models::trash::TrashEntry> {
    sqlx::query_as::<_, crate::models::trash::TrashEntry>(
        "SELECT * FROM trash WHERE id = ? AND owner_id = ?",
    )
    .bind(trash_id)
    .bind(owner_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)
}

async fn name_taken(
    db: &Db,
    owner_id: &str,
    parent_id: Option<&str>,
    name: &str,
) -> AppResult<bool> {
    let row: Option<(String,)> = match parent_id {
        Some(pid) => sqlx::query_as(
            "SELECT id FROM files WHERE owner_id = ? AND parent_id = ? AND name = ? AND is_deleted = 0",
        )
        .bind(owner_id)
        .bind(pid)
        .bind(name)
        .fetch_optional(db)
        .await?,
        None => sqlx::query_as(
            "SELECT id FROM files WHERE owner_id = ? AND parent_id IS NULL AND name = ? AND is_deleted = 0",
        )
        .bind(owner_id)
        .bind(name)
        .fetch_optional(db)
        .await?,
    };
    Ok(row.is_some())
}
