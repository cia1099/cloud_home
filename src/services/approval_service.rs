//! 账户审批：批准 / 拒绝 / 过期自动拒绝。
//!
//! 管理端点与（之后的）聊天机器人调用同一组函数，权限边界与审计完全一致。
//! 审批状态与软删除（`deleted_at`）互不相干；`pending` / `rejected` 恒为 `is_active = 0`，
//! 只有批准才会置 1。

use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::user::User;
use crate::services::user_service;
use crate::util::now_rfc3339;

/// 审批决定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Approve,
    Reject,
}

impl Decision {
    fn target_status(self) -> &'static str {
        match self {
            Self::Approve => "approved",
            Self::Reject => "rejected",
        }
    }
}

/// 一次审批的结果。
pub struct Decided {
    pub user: User,
    /// false 表示账户本来就是该状态（幂等重复调用），未做任何改动。
    pub changed: bool,
}

/// 对账户做出审批决定。
///
/// - pending → 批准 / 拒绝；rejected → 批准（允许改判）；
/// - 与当前状态相同 → 幂等成功（`changed = false`）；
/// - 其余（拒绝一个已批准的账户）→ 409，并写明先前的决定者——
///   已批准账户可能已有数据，要停用应走删除账户。
///
/// 状态迁移是一条带条件的 UPDATE，两个管理员同时点击时恰好一个生效。
/// `decided_by` 形如 `admin-endpoint`、`chat:<provider>:<sender-id>`。
pub async fn decide(
    db: &Db,
    user_id: &str,
    decision: Decision,
    decided_by: &str,
    note: Option<&str>,
) -> AppResult<Decided> {
    let now = now_rfc3339();
    let note = note.map(str::trim).filter(|n| !n.is_empty());

    let updated = match decision {
        Decision::Approve => {
            sqlx::query(
                "UPDATE users SET approval_status = 'approved', is_active = 1, \
                 approval_decided_at = ?, approval_decided_by = ?, approval_note = ?, updated_at = ? \
                 WHERE id = ? AND approval_status IN ('pending', 'rejected') AND deleted_at IS NULL",
            )
            .bind(&now)
            .bind(decided_by)
            .bind(note)
            .bind(&now)
            .bind(user_id)
            .execute(db)
            .await?
        }
        Decision::Reject => {
            sqlx::query(
                "UPDATE users SET approval_status = 'rejected', is_active = 0, \
                 approval_decided_at = ?, approval_decided_by = ?, approval_note = ?, updated_at = ? \
                 WHERE id = ? AND approval_status = 'pending'",
            )
            .bind(&now)
            .bind(decided_by)
            .bind(note)
            .bind(&now)
            .bind(user_id)
            .execute(db)
            .await?
        }
    };

    let user = user_service::get(db, user_id).await?;
    if updated.rows_affected() == 1 {
        let event = match decision {
            Decision::Approve => "user.approve",
            Decision::Reject => "user.reject",
        };
        crate::audit!(
            info,
            event,
            "success",
            target_user_id = %user.id,
            target_username = %user.username,
            decided_by = %decided_by,
            "账户审批：{}",
            decision.target_status()
        );
        return Ok(Decided { user, changed: true });
    }

    if user.approval_status == decision.target_status() {
        return Ok(Decided {
            user,
            changed: false,
        });
    }
    Err(AppError::Conflict(format!(
        "该账户已于 {} 由 {} 决定为 {}，不能改为 {}",
        user.approval_decided_at.as_deref().unwrap_or("未知时间"),
        user.approval_decided_by.as_deref().unwrap_or("未知"),
        user.approval_status,
        decision.target_status()
    )))
}

/// 把超过 `expire_days` 天仍 pending 的申请自动拒绝，返回被拒的 (id, username)。
pub async fn expire_pending(db: &Db, expire_days: i64) -> AppResult<Vec<(String, String)>> {
    let now = now_rfc3339();
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(expire_days)).to_rfc3339();
    let expired: Vec<(String, String)> = sqlx::query_as(
        "UPDATE users SET approval_status = 'rejected', is_active = 0, \
         approval_decided_at = ?, approval_decided_by = 'expiry', \
         approval_note = 'pending expired', updated_at = ? \
         WHERE approval_status = 'pending' AND created_at < ? \
         RETURNING id, username",
    )
    .bind(&now)
    .bind(&now)
    .bind(&cutoff)
    .fetch_all(db)
    .await?;

    for (id, username) in &expired {
        crate::audit!(
            info,
            "user.expire",
            "success",
            target_user_id = %id,
            target_username = %username,
            expire_days = expire_days,
            "待批准申请超期，自动拒绝"
        );
    }
    Ok(expired)
}
