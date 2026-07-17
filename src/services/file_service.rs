//! 文件与资料夹业务逻辑。

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::file::{FILE_TYPE_FOLDER, FileEntry};
use crate::util::now_rfc3339;

/// 获取单个文件（强制账户隔离），不存在或非本人所有返回 404。
pub async fn get_owned(db: &Db, owner_id: &str, id: &str) -> AppResult<FileEntry> {
    let entry = sqlx::query_as::<_, FileEntry>(
        "SELECT * FROM files WHERE id = ? AND owner_id = ? AND is_deleted = 0",
    )
    .bind(id)
    .bind(owner_id)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(entry)
}

/// 列出某目录（`parent_id` 为 None 表示根目录）下未删除的内容。
pub async fn list_dir(
    db: &Db,
    owner_id: &str,
    parent_id: Option<&str>,
) -> AppResult<Vec<FileEntry>> {
    let rows = match parent_id {
        Some(pid) => {
            // 校验父目录存在且属于本人。
            let parent = get_owned(db, owner_id, pid).await?;
            if !parent.is_folder() {
                return Err(AppError::BadRequest("parent_id 不是资料夹".into()));
            }
            sqlx::query_as::<_, FileEntry>(
                "SELECT * FROM files WHERE owner_id = ? AND parent_id = ? AND is_deleted = 0 \
                 ORDER BY file_type DESC, name ASC",
            )
            .bind(owner_id)
            .bind(pid)
            .fetch_all(db)
            .await?
        }
        None => {
            sqlx::query_as::<_, FileEntry>(
                "SELECT * FROM files WHERE owner_id = ? AND parent_id IS NULL AND is_deleted = 0 \
                 ORDER BY file_type DESC, name ASC",
            )
            .bind(owner_id)
            .fetch_all(db)
            .await?
        }
    };
    Ok(rows)
}

/// 校验父目录合法（存在、属于本人、且为资料夹）。`None` 表示根目录，直接放行。
pub async fn validate_parent(db: &Db, owner_id: &str, parent_id: Option<&str>) -> AppResult<()> {
    if let Some(pid) = parent_id {
        let parent = get_owned(db, owner_id, pid).await?;
        if !parent.is_folder() {
            return Err(AppError::BadRequest("target_parent_id 不是资料夹".into()));
        }
    }
    Ok(())
}

/// 确认同目录下没有重名（未删除）项目。
pub async fn ensure_name_available(
    db: &Db,
    owner_id: &str,
    parent_id: Option<&str>,
    name: &str,
) -> AppResult<()> {
    let exists: Option<(String,)> = match parent_id {
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
    if exists.is_some() {
        return Err(AppError::Conflict(format!("已存在同名项目: {name}")));
    }
    Ok(())
}

/// 插入一条文件/资料夹行。
#[allow(clippy::too_many_arguments)]
pub async fn insert_entry(
    db: &Db,
    id: &str,
    owner_id: &str,
    parent_id: Option<&str>,
    drive_id: &str,
    name: &str,
    file_type: &str,
    size_bytes: i64,
    mime_type: Option<&str>,
    physical_path: Option<&str>,
) -> AppResult<FileEntry> {
    let now = now_rfc3339();
    sqlx::query(
        "INSERT INTO files \
         (id, owner_id, parent_id, drive_id, name, file_type, size_bytes, mime_type, physical_path, created_at, updated_at, is_deleted) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0)",
    )
    .bind(id)
    .bind(owner_id)
    .bind(parent_id)
    .bind(drive_id)
    .bind(name)
    .bind(file_type)
    .bind(size_bytes)
    .bind(mime_type)
    .bind(physical_path)
    .bind(&now)
    .bind(&now)
    .execute(db)
    .await?;

    get_owned(db, owner_id, id).await
}

/// 重命名文件/资料夹。
pub async fn rename(db: &Db, owner_id: &str, id: &str, new_name: &str) -> AppResult<FileEntry> {
    let entry = get_owned(db, owner_id, id).await?;
    validate_name(new_name)?;
    ensure_name_available(db, owner_id, entry.parent_id.as_deref(), new_name).await?;

    sqlx::query("UPDATE files SET name = ?, updated_at = ? WHERE id = ? AND owner_id = ?")
        .bind(new_name)
        .bind(now_rfc3339())
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;

    get_owned(db, owner_id, id).await
}

/// 移动文件/资料夹到新的父目录。
pub async fn move_entry(
    db: &Db,
    owner_id: &str,
    id: &str,
    target_parent_id: Option<&str>,
) -> AppResult<FileEntry> {
    let entry = get_owned(db, owner_id, id).await?;
    validate_parent(db, owner_id, target_parent_id).await?;

    // 禁止移动到自身或自身子孙下，避免形成环。
    if let Some(target) = target_parent_id
        && (target == id || is_descendant(db, owner_id, id, target).await?)
    {
        return Err(AppError::BadRequest("不能将资料夹移动到自身或其子目录".into()));
    }

    ensure_name_available(db, owner_id, target_parent_id, &entry.name).await?;

    sqlx::query("UPDATE files SET parent_id = ?, updated_at = ? WHERE id = ? AND owner_id = ?")
        .bind(target_parent_id)
        .bind(now_rfc3339())
        .bind(id)
        .bind(owner_id)
        .execute(db)
        .await?;

    get_owned(db, owner_id, id).await
}

/// `candidate` 是否为 `ancestor` 的子孙。
async fn is_descendant(
    db: &Db,
    owner_id: &str,
    ancestor: &str,
    candidate: &str,
) -> AppResult<bool> {
    let mut current = Some(candidate.to_string());
    while let Some(id) = current {
        let row: Option<(Option<String>,)> =
            sqlx::query_as("SELECT parent_id FROM files WHERE id = ? AND owner_id = ?")
                .bind(&id)
                .bind(owner_id)
                .fetch_optional(db)
                .await?;
        match row {
            Some((parent,)) => {
                if parent.as_deref() == Some(ancestor) {
                    return Ok(true);
                }
                current = parent;
            }
            None => break,
        }
    }
    Ok(false)
}

/// 收集某个节点的所有子孙 id（含自身），用于递归软删除/恢复。
pub async fn collect_subtree_ids(db: &Db, owner_id: &str, root_id: &str) -> AppResult<Vec<String>> {
    let mut result = vec![root_id.to_string()];
    let mut queue = vec![root_id.to_string()];

    while let Some(current) = queue.pop() {
        let children: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM files WHERE parent_id = ? AND owner_id = ?")
                .bind(&current)
                .bind(owner_id)
                .fetch_all(db)
                .await?;
        for (child_id,) in children {
            result.push(child_id.clone());
            queue.push(child_id);
        }
    }
    Ok(result)
}

/// 名称合法性校验（非空、无路径分隔符）。
pub fn validate_name(name: &str) -> AppResult<()> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("名称不能为空".into()));
    }
    if trimmed.len() > 255 {
        return Err(AppError::BadRequest("名称过长".into()));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('\0') {
        return Err(AppError::BadRequest("名称含有非法字符".into()));
    }
    Ok(())
}

/// 供搜索使用：判断类型字符串是否合法。
pub fn is_valid_type_filter(t: &str) -> bool {
    t == crate::models::file::FILE_TYPE_FILE || t == FILE_TYPE_FOLDER
}
