//! 第三方登录的账户逻辑：按第三方资料找到、关联或新建账户。
//!
//! 规则见 spec/backend.md「第三方登录」。要点：
//! - 账户只由 `(provider, subject)` 这条 `managed_by = provider` 的标识行认定；
//! - 邮箱必须是第三方声明「已验证」的，才用来关联或建号；
//! - 关联到既有账户时，该账户的本地邮箱也必须已验证，否则任何人都能先抢注
//!   他人邮箱、等本人用 Google 登录时被「并入」攻击者持有密码的账户；
//! - 所有查询都落在具体 `user_id` 或 `(provider, identifier)` 上，不会跨账户读写。

use crate::auth::oauth::ExternalProfile;
use crate::config::RegistrationPolicy;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::user::{
    APPROVAL_PENDING, APPROVAL_REJECTED, PROVIDER_EMAIL, User, UserIdentity,
};
use crate::services::user_service::{self, NewIdentity, RegisterSource, insert_identity};
use crate::util::now_rfc3339;

/// 409：邮箱已属于一个「本地邮箱尚未验证」的账户，不能并入。
pub const CODE_EMAIL_UNVERIFIED_LINK: &str = "OAUTH_EMAIL_UNVERIFIED_LINK";
/// 409：该第三方账号已绑定到另一个账户。
pub const CODE_ACCOUNT_ALREADY_LINKED: &str = "OAUTH_ACCOUNT_ALREADY_LINKED";
/// 409：当前账户已绑定同一 provider 的另一个第三方账号。
pub const CODE_PROVIDER_ALREADY_LINKED: &str = "OAUTH_PROVIDER_ALREADY_LINKED";

/// 登录结果。
pub struct OAuthLogin {
    pub user: User,
    /// 本次是否新建了账户；`user.is_active == 0` 且为新建 ⇒ 等待管理员批准（handler 回 202）。
    pub created: bool,
}

/// 登录：已绑定则同步资料后返回该账户；否则按已验证邮箱关联既有账户或新建。
///
/// 新建账户与 `/auth/register` 走同一条注册闸门与审批规则
/// （`user_service::admission_gate` / `insert_user`）：closed 拒绝、approval 待批准、
/// open 直接启用、引导规则同样适用。已存在但未获批准的账户登录一律 403。
pub async fn login(
    db: &Db,
    policy: &RegistrationPolicy,
    provider: &str,
    profile: &ExternalProfile,
    source: RegisterSource,
) -> AppResult<OAuthLogin> {
    let subject = subject_of(profile)?;

    if let Some(identity) = find_identity(db, provider, subject).await? {
        let user = user_service::get(db, &identity.user_id).await?;
        ensure_usable(provider, &user)?;
        sync_profile(db, provider, &user, profile).await?;
        let user = user_service::get(db, &user.id).await?;
        return Ok(OAuthLogin {
            user,
            created: false,
        });
    }

    let email = verified_email(profile)?;
    // 先过注册闸门再进事务：已有账户并入不算注册，但无法事先区分，所以只对「会新建」的分支把关。
    let existing = find_identity(db, PROVIDER_EMAIL, &email).await?;
    let mut outcome = None;
    if existing.is_none() {
        user_service::admission_gate(db, policy, &source).await?;
    }

    let now = now_rfc3339();
    let mut tx = db.begin().await?;
    let user_id = match existing {
        Some(local) => {
            if local.verified_at.is_none() {
                return Err(AppError::CodedConflict(
                    CODE_EMAIL_UNVERIFIED_LINK,
                    "该邮箱已注册但尚未验证，无法并入；请先用原方式登录并验证邮箱，再绑定第三方账号"
                        .into(),
                ));
            }
            local.user_id
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            let name = display_name(profile, &email);
            outcome =
                Some(user_service::insert_user(&mut tx, policy, &id, &name, &source, &now).await?);
            let managed_email = NewIdentity {
                verified_at: Some(&now),
                managed_by: Some(provider),
                ..NewIdentity::new(&id, PROVIDER_EMAIL, &email)
            };
            insert_identity(&mut tx, &managed_email, &now).await?;
            id
        }
    };

    let identity = NewIdentity {
        verified_at: Some(&now),
        managed_by: Some(provider),
        ..NewIdentity::new(&user_id, provider, subject)
    };
    insert_identity(&mut tx, &identity, &now).await?;
    tx.commit().await?;

    let user = user_service::get(db, &user_id).await?;
    let created = outcome.is_some();
    if !created {
        ensure_usable(provider, &user)?;
    }
    Ok(OAuthLogin { user, created })
}

/// 账户必须已获批准且启用：pending / rejected 给出与密码登录一致的 403，其余（被运维删除）401。
fn ensure_usable(provider: &str, user: &User) -> AppResult<()> {
    if user.is_active == 1 {
        return Ok(());
    }
    let (reason, err) = match user.approval_status.as_str() {
        APPROVAL_PENDING => ("pending_approval", AppError::AccountPending),
        APPROVAL_REJECTED => ("rejected", AppError::AccountRejected),
        _ => ("user_inactive", AppError::Unauthorized),
    };
    crate::audit!(
        warn,
        "auth.oauth_login",
        "failure",
        reason = reason,
        provider = %provider,
        user_id = %user.id,
        "第三方登录失败：账户未启用"
    );
    Err(err)
}

/// 为已登录账户绑定第三方账号。幂等；该第三方账号属于别人、
/// 或本账户已绑了同一家的另一个账号，都是 409，绝不转移归属。
pub async fn link(
    db: &Db,
    user_id: &str,
    provider: &str,
    profile: &ExternalProfile,
) -> AppResult<()> {
    let subject = subject_of(profile)?;

    if let Some(found) = find_identity(db, provider, subject).await? {
        return if found.user_id == user_id {
            Ok(())
        } else {
            Err(AppError::CodedConflict(
                CODE_ACCOUNT_ALREADY_LINKED,
                "该第三方账号已绑定到另一个账户".into(),
            ))
        };
    }

    let mine = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(db)
    .await?;
    if mine.is_some() {
        return Err(AppError::CodedConflict(
            CODE_PROVIDER_ALREADY_LINKED,
            format!("当前账户已绑定另一个 {provider} 账号"),
        ));
    }

    let now = now_rfc3339();
    let identity = NewIdentity {
        verified_at: Some(&now),
        managed_by: Some(provider),
        ..NewIdentity::new(user_id, provider, subject)
    };
    let mut conn = db.acquire().await?;
    insert_identity(&mut conn, &identity, &now).await
}

async fn find_identity(db: &Db, provider: &str, subject: &str) -> AppResult<Option<UserIdentity>> {
    let row = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE provider = ? AND identifier = ?",
    )
    .bind(provider)
    .bind(subject)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

fn subject_of(profile: &ExternalProfile) -> AppResult<&str> {
    let s = profile.subject.trim();
    if s.is_empty() {
        return Err(AppError::Unauthorized);
    }
    Ok(s)
}

/// 第三方必须声明邮箱已验证；否则明确拒绝并说明原因，不静默降级。
fn verified_email(profile: &ExternalProfile) -> AppResult<String> {
    match profile.email.as_deref() {
        Some(raw) if profile.email_verified => user_service::normalize_email(raw),
        _ => Err(AppError::BadRequest(
            "第三方未提供已验证的邮箱（或使用了私密转发地址），无法登录".into(),
        )),
    }
}

fn display_name(profile: &ExternalProfile, email: &str) -> String {
    profile
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| email.split('@').next().unwrap_or(email).to_string())
}

/// 每次第三方登录按其响应同步资料：覆盖 `username`，并更新本 provider 托管的邮箱行。
/// `managed_by IS NULL` 的行绝不碰；邮箱撞上别的账户则跳过并告警，不挡登录。
async fn sync_profile(
    db: &Db,
    provider: &str,
    user: &User,
    profile: &ExternalProfile,
) -> AppResult<()> {
    if let Some(name) = profile
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        && name != user.username
    {
        user_service::update_username(db, &user.id, name).await?;
    }

    let Ok(email) = verified_email(profile) else {
        return Ok(());
    };
    let res = sqlx::query(
        "UPDATE user_identities SET identifier = ?, updated_at = ? \
         WHERE user_id = ? AND provider = ? AND managed_by = ? AND identifier <> ?",
    )
    .bind(&email)
    .bind(now_rfc3339())
    .bind(&user.id)
    .bind(PROVIDER_EMAIL)
    .bind(provider)
    .bind(&email)
    .execute(db)
    .await;
    if let Err(sqlx::Error::Database(e)) = &res
        && e.is_unique_violation()
    {
        crate::audit!(
            warn,
            "auth.oauth_sync",
            "skipped",
            reason = "email_taken",
            provider = %provider,
            user_id = %user.id,
            "第三方邮箱已属于另一个账户，跳过邮箱同步"
        );
        return Ok(());
    }
    res?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegistrationMode;
    use crate::db::test_pool;
    use crate::services::user_service::NewAccount;

    fn policy(mode: RegistrationMode) -> RegistrationPolicy {
        RegistrationPolicy {
            mode,
            rate_per_ip_per_hour: 100,
            pending_max: 100,
            pending_expire_days: 14,
        }
    }

    fn open() -> RegistrationPolicy {
        policy(RegistrationMode::Open)
    }

    fn src(is_local: bool) -> RegisterSource {
        RegisterSource {
            ip: Some("8.8.8.8".into()),
            is_local,
        }
    }

    fn profile(subject: &str, email: &str) -> ExternalProfile {
        ExternalProfile {
            subject: subject.into(),
            email: Some(email.into()),
            email_verified: true,
            name: Some("Dad".into()),
        }
    }

    async fn sign_in(
        db: &Db,
        p: &RegistrationPolicy,
        subject: &str,
        email: &str,
    ) -> AppResult<OAuthLogin> {
        login(db, p, "google", &profile(subject, email), src(false)).await
    }

    /// 以 open 模式注册一个密码账户（已启用）。
    async fn password_user(db: &Db, name: &str, email: &str) -> User {
        user_service::register(
            db,
            &open(),
            NewAccount {
                username: name,
                password: "password123",
                email: Some(email),
                phone: None,
            },
            src(false),
        )
        .await
        .unwrap()
        .0
    }

    async fn user_count(db: &Db) -> i64 {
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(db)
            .await
            .unwrap();
        n.0
    }

    #[tokio::test]
    async fn first_login_creates_account_and_identities() {
        let db = test_pool().await;
        let r = sign_in(&db, &open(), "g-1", "Dad@Gmail.com").await.unwrap();
        assert!(r.created);
        assert_eq!(r.user.is_active, 1);
        assert_eq!(r.user.username, "Dad");
        let ids = user_service::identities(&db, &r.user.id).await.unwrap();
        let google = ids.iter().find(|i| i.provider == "google").unwrap();
        assert_eq!(google.identifier, "g-1");
        assert_eq!(google.managed_by.as_deref(), Some("google"));
        assert!(google.verified_at.is_some());
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert_eq!(email.identifier, "dad@gmail.com");
        assert_eq!(email.managed_by.as_deref(), Some("google"));
        assert!(ids.iter().all(|i| i.provider != "password"));
    }

    #[tokio::test]
    async fn repeat_login_is_same_user_and_syncs_name() {
        let db = test_pool().await;
        let a = sign_in(&db, &open(), "g-1", "dad@gmail.com").await.unwrap();
        let mut p = profile("g-1", "dad@gmail.com");
        p.name = Some("Father".into());
        let b = login(&db, &open(), "google", &p, src(false)).await.unwrap();
        assert!(!b.created);
        assert_eq!(a.user.id, b.user.id);
        assert_eq!(b.user.username, "Father");
        assert_eq!(user_count(&db).await, 1);
    }

    #[tokio::test]
    async fn different_subjects_are_isolated_accounts() {
        let db = test_pool().await;
        let a = sign_in(&db, &open(), "g-1", "a@gmail.com").await.unwrap();
        let b = sign_in(&db, &open(), "g-2", "b@gmail.com").await.unwrap();
        assert_ne!(a.user.id, b.user.id);
        // 同一 subject 在另一家 provider 下是另一个身份，不会落到别人账户。
        let c = login(
            &db,
            &open(),
            "apple",
            &profile("g-1", "c@icloud.com"),
            src(false),
        )
        .await
        .unwrap();
        assert_ne!(c.user.id, a.user.id);
    }

    #[tokio::test]
    async fn unverified_or_missing_email_is_rejected_with_reason() {
        let db = test_pool().await;
        let mut p = profile("g-1", "a@gmail.com");
        p.email_verified = false;
        let r = login(&db, &open(), "google", &p, src(false)).await;
        assert!(matches!(r, Err(AppError::BadRequest(_))));
        p.email = None;
        p.email_verified = true;
        let r = login(&db, &open(), "google", &p, src(false)).await;
        assert!(matches!(r, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn approval_mode_creates_pending_account_without_session() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let r = sign_in(&db, &p, "g-1", "a@gmail.com").await.unwrap();
        assert!(r.created);
        assert_eq!(r.user.is_active, 0);
        assert_eq!(r.user.approval_status, "pending");
        // 再次登录：仍待批准，403，不会再建号。
        let again = sign_in(&db, &p, "g-1", "a@gmail.com").await;
        assert!(matches!(again, Err(AppError::AccountPending)));
        assert_eq!(user_count(&db).await, 1);
    }

    #[tokio::test]
    async fn approval_mode_bootstraps_first_local_account_only() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let first = login(&db, &p, "google", &profile("g-1", "a@gmail.com"), src(true))
            .await
            .unwrap();
        assert_eq!(first.user.is_active, 1);
        assert_eq!(first.user.approval_decided_by.as_deref(), Some("bootstrap"));
        let second = login(&db, &p, "google", &profile("g-2", "b@gmail.com"), src(true))
            .await
            .unwrap();
        assert_eq!(second.user.is_active, 0);
    }

    #[tokio::test]
    async fn approved_pending_account_can_then_login() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let r = sign_in(&db, &p, "g-1", "a@gmail.com").await.unwrap();
        sqlx::query("UPDATE users SET approval_status = 'approved', is_active = 1 WHERE id = ?")
            .bind(&r.user.id)
            .execute(&db)
            .await
            .unwrap();
        let again = sign_in(&db, &p, "g-1", "a@gmail.com").await.unwrap();
        assert!(!again.created);
        assert_eq!(again.user.id, r.user.id);
    }

    #[tokio::test]
    async fn rejected_account_gets_account_rejected() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let r = sign_in(&db, &p, "g-1", "a@gmail.com").await.unwrap();
        sqlx::query("UPDATE users SET approval_status = 'rejected' WHERE id = ?")
            .bind(&r.user.id)
            .execute(&db)
            .await
            .unwrap();
        let again = sign_in(&db, &p, "g-1", "a@gmail.com").await;
        assert!(matches!(again, Err(AppError::AccountRejected)));
    }

    #[tokio::test]
    async fn closed_mode_refuses_new_accounts_but_not_existing_ones() {
        let db = test_pool().await;
        let existing = sign_in(&db, &open(), "g-1", "a@gmail.com").await.unwrap();
        let closed = policy(RegistrationMode::Closed);
        let new = sign_in(&db, &closed, "g-2", "b@gmail.com").await;
        assert!(matches!(new, Err(AppError::RegistrationClosed)));
        let back = sign_in(&db, &closed, "g-1", "a@gmail.com").await.unwrap();
        assert_eq!(back.user.id, existing.user.id);
    }

    #[tokio::test]
    async fn pending_queue_limit_applies_to_oauth_signups() {
        let db = test_pool().await;
        let p = RegistrationPolicy {
            pending_max: 1,
            ..policy(RegistrationMode::Approval)
        };
        sign_in(&db, &p, "g-1", "a@gmail.com").await.unwrap();
        let r = sign_in(&db, &p, "g-2", "b@gmail.com").await;
        assert!(matches!(r, Err(AppError::RegistrationBusy(_))));
    }

    #[tokio::test]
    async fn links_onto_account_with_verified_local_email_only() {
        let db = test_pool().await;
        let owner = password_user(&db, "owner", "dad@gmail.com").await;

        // 本地邮箱未验证：拒绝并入，带专用错误码。
        let r = sign_in(&db, &open(), "g-1", "dad@gmail.com").await;
        assert!(matches!(
            r,
            Err(AppError::CodedConflict(CODE_EMAIL_UNVERIFIED_LINK, _))
        ));

        user_service::mark_verified(&db, &owner.id, "email")
            .await
            .unwrap();
        let u = sign_in(&db, &open(), "g-1", "dad@gmail.com").await.unwrap();
        assert!(!u.created);
        assert_eq!(u.user.id, owner.id);
        // 本地自有的邮箱行不被第三方改写。
        let ids = user_service::identities(&db, &owner.id).await.unwrap();
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert!(email.managed_by.is_none());
    }

    #[tokio::test]
    async fn deactivated_account_cannot_login() {
        let db = test_pool().await;
        let u = sign_in(&db, &open(), "g-1", "a@gmail.com").await.unwrap();
        sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
            .bind(&u.user.id)
            .execute(&db)
            .await
            .unwrap();
        let r = sign_in(&db, &open(), "g-1", "a@gmail.com").await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn sync_skips_email_owned_by_another_account() {
        let db = test_pool().await;
        let a = sign_in(&db, &open(), "g-1", "a@gmail.com").await.unwrap();
        sign_in(&db, &open(), "g-2", "b@gmail.com").await.unwrap();
        // a 在 Google 改了邮箱，撞上 b：登录放行，邮箱保持原样。
        let again = sign_in(&db, &open(), "g-1", "b@gmail.com").await.unwrap();
        assert_eq!(again.user.id, a.user.id);
        let ids = user_service::identities(&db, &a.user.id).await.unwrap();
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert_eq!(email.identifier, "a@gmail.com");
    }

    #[tokio::test]
    async fn link_is_idempotent_and_never_steals() {
        let db = test_pool().await;
        let a = password_user(&db, "alice", "a@x.com").await;
        let b = password_user(&db, "bob", "b@x.com").await;

        link(&db, &a.id, "google", &profile("g-1", "w@gmail.com"))
            .await
            .unwrap();
        link(&db, &a.id, "google", &profile("g-1", "w@gmail.com"))
            .await
            .unwrap();

        // bob 拿同一个第三方账号来绑：专用 409，且归属不变。
        let r = link(&db, &b.id, "google", &profile("g-1", "w@gmail.com")).await;
        assert!(matches!(
            r,
            Err(AppError::CodedConflict(CODE_ACCOUNT_ALREADY_LINKED, _))
        ));
        // alice 再绑同家的另一个账号：另一个专用 409。
        let r = link(&db, &a.id, "google", &profile("g-2", "x@gmail.com")).await;
        assert!(matches!(
            r,
            Err(AppError::CodedConflict(CODE_PROVIDER_ALREADY_LINKED, _))
        ));

        // 绑定后用 Google 登录进的就是 alice，bob 身上没有 google 标识。
        let u = sign_in(&db, &open(), "g-1", "w@gmail.com").await.unwrap();
        assert_eq!(u.user.id, a.id);
        let bob_ids = user_service::identities(&db, &b.id).await.unwrap();
        assert!(bob_ids.iter().all(|i| i.provider != "google"));
    }
}
