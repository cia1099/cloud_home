//! 第三方登录的账户逻辑：按第三方资料找到、关联或新建账户。
//!
//! 规则见 spec/backend.md「第三方登录」。要点：
//! - 账户只由 `(provider, subject)` 这条 `managed_by = provider` 的标识行认定；
//! - 邮箱必须是第三方声明「已验证」的，才用来关联或建号；
//! - 关联到既有账户时，该账户的本地邮箱也必须已验证，否则任何人都能先抢注
//!   他人邮箱、等本人用 Google 登录时被「并入」攻击者持有密码的账户；
//! - 所有查询都落在具体 `user_id` 或 `(provider, identifier)` 上，不会跨账户读写。

use crate::auth::oauth::ExternalProfile;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::user::{PROVIDER_EMAIL, User, UserIdentity};
use crate::services::user_service::{self, NewIdentity, insert_identity};
use crate::util::now_rfc3339;

/// 登录：已绑定则同步资料后返回该账户；否则按已验证邮箱关联既有账户或新建。
pub async fn login(db: &Db, provider: &str, profile: &ExternalProfile) -> AppResult<User> {
    let subject = subject_of(profile)?;

    if let Some(identity) = find_identity(db, provider, subject).await? {
        let user = user_service::get(db, &identity.user_id).await?;
        if user.is_active != 1 {
            crate::audit!(
                warn,
                "auth.oauth_login",
                "failure",
                reason = "user_inactive",
                provider = %provider,
                user_id = %user.id,
                "第三方登录失败：账户已停用"
            );
            return Err(AppError::Unauthorized);
        }
        sync_profile(db, provider, &user, profile).await?;
        return user_service::get(db, &user.id).await;
    }

    let email = verified_email(profile)?;
    let now = now_rfc3339();
    let mut tx = db.begin().await?;

    let existing = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE provider = ? AND identifier = ?",
    )
    .bind(PROVIDER_EMAIL)
    .bind(&email)
    .fetch_optional(&mut *tx)
    .await?;

    let user_id = match existing {
        Some(local) => {
            if local.verified_at.is_none() {
                return Err(AppError::Conflict(
                    "该邮箱已注册但尚未验证，无法并入；请先用原方式登录并验证邮箱，再绑定第三方账号"
                        .into(),
                ));
            }
            local.user_id
        }
        None => {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO users (id, username, created_at, updated_at, is_active) \
                 VALUES (?, ?, ?, ?, 1)",
            )
            .bind(&id)
            .bind(display_name(profile, &email))
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
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
    if user.is_active != 1 {
        return Err(AppError::Unauthorized);
    }
    Ok(user)
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
            Err(AppError::Conflict("该第三方账号已绑定到另一个账户".into()))
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
        return Err(AppError::Conflict(format!(
            "当前账户已绑定另一个 {provider} 账号"
        )));
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

async fn find_identity(
    db: &Db,
    provider: &str,
    subject: &str,
) -> AppResult<Option<UserIdentity>> {
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
    if let Some(name) = profile.name.as_deref().map(str::trim).filter(|n| !n.is_empty())
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
    use crate::db;

    async fn test_db() -> Db {
        let dir = std::env::temp_dir().join(format!("ch-oauth-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let pool = db::init_pool(&dir.join("t.db")).await.unwrap();
        db::run_migrations(&pool).await.unwrap();
        pool
    }

    fn profile(subject: &str, email: &str) -> ExternalProfile {
        ExternalProfile {
            subject: subject.into(),
            email: Some(email.into()),
            email_verified: true,
            name: Some("Dad".into()),
        }
    }

    #[tokio::test]
    async fn first_login_creates_account_and_identities() {
        let db = test_db().await;
        let u = login(&db, "google", &profile("g-1", "Dad@Gmail.com")).await.unwrap();
        assert_eq!(u.username, "Dad");
        let ids = user_service::identities(&db, &u.id).await.unwrap();
        let google = ids.iter().find(|i| i.provider == "google").unwrap();
        assert_eq!(google.identifier, "g-1");
        assert_eq!(google.managed_by.as_deref(), Some("google"));
        assert!(google.verified_at.is_some());
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert_eq!(email.identifier, "dad@gmail.com");
        assert_eq!(email.managed_by.as_deref(), Some("google"));
        // 纯第三方账户没有密码凭证。
        assert!(ids.iter().all(|i| i.provider != "password"));
    }

    #[tokio::test]
    async fn repeat_login_is_same_user_and_syncs_name() {
        let db = test_db().await;
        let a = login(&db, "google", &profile("g-1", "dad@gmail.com")).await.unwrap();
        let mut p = profile("g-1", "dad@gmail.com");
        p.name = Some("Father".into());
        let b = login(&db, "google", &p).await.unwrap();
        assert_eq!(a.id, b.id);
        assert_eq!(b.username, "Father");
        let n: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(n.0, 1);
    }

    #[tokio::test]
    async fn different_subjects_are_isolated_accounts() {
        let db = test_db().await;
        let a = login(&db, "google", &profile("g-1", "a@gmail.com")).await.unwrap();
        let b = login(&db, "google", &profile("g-2", "b@gmail.com")).await.unwrap();
        assert_ne!(a.id, b.id);
        // 同一 subject 在另一家 provider 下是另一个身份，不会落到别人账户。
        let c = login(&db, "apple", &profile("g-1", "c@icloud.com")).await.unwrap();
        assert_ne!(c.id, a.id);
    }

    #[tokio::test]
    async fn unverified_or_missing_email_is_rejected_with_reason() {
        let db = test_db().await;
        let mut p = profile("g-1", "a@gmail.com");
        p.email_verified = false;
        assert!(matches!(login(&db, "google", &p).await, Err(AppError::BadRequest(_))));
        p.email = None;
        p.email_verified = true;
        assert!(matches!(login(&db, "google", &p).await, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn links_onto_account_with_verified_local_email_only() {
        let db = test_db().await;
        let owner = user_service::create(&db, "owner", "password123", Some("dad@gmail.com"), None)
            .await
            .unwrap();

        // 本地邮箱未验证：拒绝并入。
        let r = login(&db, "google", &profile("g-1", "dad@gmail.com")).await;
        assert!(matches!(r, Err(AppError::Conflict(_))));

        user_service::mark_verified(&db, &owner.id, "email").await.unwrap();
        let u = login(&db, "google", &profile("g-1", "dad@gmail.com")).await.unwrap();
        assert_eq!(u.id, owner.id);
        // 本地自有的邮箱行不被第三方改写。
        let ids = user_service::identities(&db, &owner.id).await.unwrap();
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert!(email.managed_by.is_none());
    }

    #[tokio::test]
    async fn deactivated_account_cannot_login() {
        let db = test_db().await;
        let u = login(&db, "google", &profile("g-1", "a@gmail.com")).await.unwrap();
        sqlx::query("UPDATE users SET is_active = 0 WHERE id = ?")
            .bind(&u.id)
            .execute(&db)
            .await
            .unwrap();
        let r = login(&db, "google", &profile("g-1", "a@gmail.com")).await;
        assert!(matches!(r, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn sync_skips_email_owned_by_another_account() {
        let db = test_db().await;
        let a = login(&db, "google", &profile("g-1", "a@gmail.com")).await.unwrap();
        login(&db, "google", &profile("g-2", "b@gmail.com")).await.unwrap();
        // a 在 Google 改了邮箱，撞上 b：登录放行，邮箱保持原样。
        let again = login(&db, "google", &profile("g-1", "b@gmail.com")).await.unwrap();
        assert_eq!(again.id, a.id);
        let ids = user_service::identities(&db, &a.id).await.unwrap();
        let email = ids.iter().find(|i| i.provider == "email").unwrap();
        assert_eq!(email.identifier, "a@gmail.com");
    }

    #[tokio::test]
    async fn link_is_idempotent_and_never_steals() {
        let db = test_db().await;
        let a = user_service::create(&db, "alice", "password123", Some("a@x.com"), None)
            .await
            .unwrap();
        let b = user_service::create(&db, "bob", "password123", Some("b@x.com"), None)
            .await
            .unwrap();

        link(&db, &a.id, "google", &profile("g-1", "whatever@gmail.com")).await.unwrap();
        link(&db, &a.id, "google", &profile("g-1", "whatever@gmail.com")).await.unwrap();

        // bob 拿同一个第三方账号来绑：409，且归属不变。
        let r = link(&db, &b.id, "google", &profile("g-1", "whatever@gmail.com")).await;
        assert!(matches!(r, Err(AppError::Conflict(_))));
        // alice 再绑同家的另一个账号：409。
        let r = link(&db, &a.id, "google", &profile("g-2", "x@gmail.com")).await;
        assert!(matches!(r, Err(AppError::Conflict(_))));

        // 绑定后用 Google 登录进的就是 alice，且她原有的邮箱标识保持用户自有。
        let u = login(&db, "google", &profile("g-1", "whatever@gmail.com")).await.unwrap();
        assert_eq!(u.id, a.id);
        assert!(user_service::identities(&db, &b.id)
            .await
            .unwrap()
            .iter()
            .all(|i| i.provider != "google"));
    }
}
