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
/// - pending → 批准 / 拒绝；rejected → 批准仅当 `overturn_rejection`（管理端点：允许改判；
///   聊天按钮：不允许，否则并发的 Reject / Approve 会两个都「成功」）；
/// - 与当前状态相同 → 幂等成功（`changed = false`）；
/// - 其余（拒绝一个已批准的账户等）→ 409，并写明先前的决定者——
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
    overturn_rejection: bool,
) -> AppResult<Decided> {
    let now = now_rfc3339();
    let note = note.map(str::trim).filter(|n| !n.is_empty());

    let updated = match decision {
        Decision::Approve => {
            sqlx::query(
                "UPDATE users SET approval_status = 'approved', is_active = 1, \
                 approval_decided_at = ?, approval_decided_by = ?, approval_note = ?, updated_at = ? \
                 WHERE id = ? AND (approval_status = 'pending' \
                 OR (? AND approval_status = 'rejected')) AND deleted_at IS NULL",
            )
            .bind(&now)
            .bind(decided_by)
            .bind(note)
            .bind(&now)
            .bind(user_id)
            .bind(overturn_rejection)
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
        return Ok(Decided {
            user,
            changed: true,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RegistrationMode, RegistrationPolicy};
    use crate::db::test_pool;
    use crate::services::user_service::{NewAccount, RegisterSource, register};

    async fn pending(db: &Db, email: &str) -> User {
        let policy = RegistrationPolicy {
            mode: RegistrationMode::Approval,
            rate_per_ip_per_hour: 100,
            pending_max: 100,
            pending_expire_days: 14,
        };
        register(
            db,
            &policy,
            NewAccount {
                username: "n",
                password: "password123",
                email: Some(email),
                phone: None,
            },
            RegisterSource {
                ip: Some("8.8.8.8".into()),
                is_local: false,
            },
        )
        .await
        .unwrap()
        .0
    }

    #[tokio::test]
    async fn approve_activates_and_is_idempotent() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;

        let d = decide(
            &db,
            &u.id,
            Decision::Approve,
            "admin-endpoint",
            Some(" ok "),
            true,
        )
        .await
        .unwrap();
        assert!(d.changed);
        assert_eq!(
            (d.user.is_active, d.user.approval_status.as_str()),
            (1, "approved")
        );
        assert_eq!(
            d.user.approval_decided_by.as_deref(),
            Some("admin-endpoint")
        );
        assert_eq!(d.user.approval_note.as_deref(), Some("ok"));

        // 重复批准：幂等，不改动，且不覆盖先前的决定者。
        let again = decide(&db, &u.id, Decision::Approve, "chat:telegram:1", None, false)
            .await
            .unwrap();
        assert!(!again.changed);
        assert_eq!(
            again.user.approval_decided_by.as_deref(),
            Some("admin-endpoint")
        );
    }

    #[tokio::test]
    async fn reject_keeps_inactive_and_reject_is_idempotent() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        let d = decide(&db, &u.id, Decision::Reject, "admin-endpoint", Some("spam"), true)
            .await
            .unwrap();
        assert!(d.changed);
        assert_eq!(
            (d.user.is_active, d.user.approval_status.as_str()),
            (0, "rejected")
        );
        assert!(
            !decide(&db, &u.id, Decision::Reject, "admin-endpoint", None, true)
                .await
                .unwrap()
                .changed
        );
    }

    #[tokio::test]
    async fn reject_then_approve_is_allowed() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        decide(&db, &u.id, Decision::Reject, "admin-endpoint", None, true)
            .await
            .unwrap();
        let d = decide(&db, &u.id, Decision::Approve, "admin-endpoint", None, true)
            .await
            .unwrap();
        assert!(d.changed);
        assert_eq!(
            (d.user.is_active, d.user.approval_status.as_str()),
            (1, "approved")
        );
    }

    #[tokio::test]
    async fn chat_style_approve_cannot_overturn_a_rejection() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        decide(&db, &u.id, Decision::Reject, "chat:telegram:2", None, false)
            .await
            .unwrap();
        assert!(matches!(
            decide(&db, &u.id, Decision::Approve, "chat:telegram:1", None, false).await,
            Err(AppError::Conflict(_))
        ));
        let after = user_service::get(&db, &u.id).await.unwrap();
        assert_eq!((after.is_active, after.approval_status.as_str()), (0, "rejected"));
    }

    #[tokio::test]
    async fn rejecting_an_approved_account_conflicts_and_names_the_decider() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        decide(&db, &u.id, Decision::Approve, "chat:telegram:42", None, false)
            .await
            .unwrap();
        let Err(AppError::Conflict(msg)) =
            decide(&db, &u.id, Decision::Reject, "admin-endpoint", None, true).await
        else {
            panic!("expected conflict");
        };
        assert!(msg.contains("chat:telegram:42"), "{msg}");
        assert_eq!(user_service::get(&db, &u.id).await.unwrap().is_active, 1);
    }

    #[tokio::test]
    async fn approve_never_reactivates_a_soft_deleted_account() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        sqlx::query("UPDATE users SET deleted_at = ? WHERE id = ?")
            .bind(now_rfc3339())
            .bind(&u.id)
            .execute(&db)
            .await
            .unwrap();
        assert!(
            decide(&db, &u.id, Decision::Approve, "admin-endpoint", None, true)
                .await
                .is_err()
        );
        assert_eq!(user_service::get(&db, &u.id).await.unwrap().is_active, 0);
    }

    #[tokio::test]
    async fn unknown_account_is_not_found() {
        let db = test_pool().await;
        assert!(matches!(
            decide(&db, "nope", Decision::Approve, "admin-endpoint", None, true).await,
            Err(AppError::NotFound)
        ));
    }

    #[tokio::test]
    async fn simultaneous_decisions_have_exactly_one_winner() {
        let db = test_pool().await;
        let u = pending(&db, "a@x.com").await;
        let (a, b) = tokio::join!(
            decide(&db, &u.id, Decision::Approve, "chat:telegram:1", None, false),
            decide(&db, &u.id, Decision::Approve, "chat:telegram:2", None, false),
        );
        let changed = [a.unwrap(), b.unwrap()]
            .iter()
            .filter(|d| d.changed)
            .count();
        assert_eq!(changed, 1);
    }

    #[tokio::test]
    async fn expiry_rejects_only_old_pending_accounts() {
        let db = test_pool().await;
        let old = pending(&db, "old@x.com").await;
        let fresh = pending(&db, "new@x.com").await;
        let approved = pending(&db, "ok@x.com").await;
        decide(&db, &approved.id, Decision::Approve, "admin-endpoint", None, true)
            .await
            .unwrap();

        let backdated = (chrono::Utc::now() - chrono::Duration::days(15)).to_rfc3339();
        for id in [&old.id, &approved.id] {
            sqlx::query("UPDATE users SET created_at = ? WHERE id = ?")
                .bind(&backdated)
                .bind(id)
                .execute(&db)
                .await
                .unwrap();
        }

        let expired = expire_pending(&db, 14).await.unwrap();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].0, old.id);

        let o = user_service::get(&db, &old.id).await.unwrap();
        assert_eq!((o.is_active, o.approval_status.as_str()), (0, "rejected"));
        assert_eq!(o.approval_decided_by.as_deref(), Some("expiry"));
        assert_eq!(
            user_service::get(&db, &fresh.id)
                .await
                .unwrap()
                .approval_status,
            "pending"
        );
        assert_eq!(
            user_service::get(&db, &approved.id)
                .await
                .unwrap()
                .approval_status,
            "approved"
        );
    }
}
