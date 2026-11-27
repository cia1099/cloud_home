//! 运维操作（删除 / 还原账户、清空回收站）：`/admin/*` 端点与聊天指令共用同一份实现，
//! 权限边界、副作用与审计完全一致。调用方负责鉴权（`AdminGuard` 或已配对的聊天管理员），
//! `actor` 只用于审计，标明是谁发起的。

use crate::db::Db;
use crate::drive::DriveManager;
use crate::error::AppResult;
use crate::events::ChangeNotifier;
use crate::models::event::{ChangeKind, FileChangeEvent};
use crate::models::user::User;
use crate::services::{trash_service, user_service};

/// 软删除账户：文件全部进回收站，账户停用。返回 (账户, 受影响项目数)。
pub async fn delete_account(
    db: &Db,
    retention_days: i64,
    target: &User,
    actor: &str,
) -> AppResult<(User, u64)> {
    let (user, affected) = user_service::soft_remove(db, retention_days, &target.id).await?;
    crate::audit!(
        info,
        "admin.user.delete",
        "success",
        actor = actor,
        target_user_id = %user.id,
        target_username = %user.username,
        affected_items = affected,
        "运维删除账户"
    );
    Ok((user, affected))
}

/// 还原账户。返回 (账户, 还原的项目数)。
pub async fn restore_account(db: &Db, target: &User, actor: &str) -> AppResult<(User, u64)> {
    let (user, affected) = user_service::restore(db, &target.id).await?;
    crate::audit!(
        info,
        "admin.user.restore",
        "success",
        actor = actor,
        target_user_id = %user.id,
        target_username = %user.username,
        affected_items = affected,
        "运维还原账户"
    );
    Ok((user, affected))
}

/// 立即清空某账户的回收站（物理删除）。返回 (删除项目数, 释放字节数)。只动该账户的回收站。
pub async fn purge_trash(
    db: &Db,
    drive: &DriveManager,
    events: &ChangeNotifier,
    target: &User,
    actor: &str,
) -> AppResult<(u64, u64)> {
    // 先确认盘在：物理删除做不了就不该动数据库，否则会留下指向不存在文件的空记录。
    let data_root = drive.require_data_root().await?;
    let (count, bytes) = trash_service::empty(db, &data_root, &target.id).await?;

    crate::audit!(
        info,
        "admin.trash.purge",
        "success",
        actor = actor,
        target_user_id = %target.id,
        target_username = %target.username,
        deleted_count = count,
        freed_bytes = bytes,
        "运维清空账户回收站"
    );

    if count > 0 {
        events.notify(
            &target.id,
            FileChangeEvent {
                kind: ChangeKind::Purged,
                file: None,
                parent_ids: Vec::new(),
            },
        );
    }
    Ok((count, bytes))
}
