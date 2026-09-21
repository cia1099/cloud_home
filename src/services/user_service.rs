//! 用户与身份（凭证 / 标识）业务逻辑。
//!
//! 设计要点：`users` 只存个人资料，所有凭证与标识都在 `user_identities`，
//! 每条标识用 `managed_by` 记录归属——NULL 为用户自有（可编辑），
//! `google`/`apple` 为第三方托管（本地只读）。同一账户可两者兼有：
//! 先用邮箱注册、之后绑定 Google 的用户，其邮箱仍归自己编辑。

use crate::auth::password;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::user::{
    PROVIDER_EMAIL, PROVIDER_PASSWORD, PROVIDER_PHONE, USER_OWNED_PROVIDERS, User, UserIdentity,
};
use crate::services::trash_service;
use crate::util::now_rfc3339;

// ---- 查询 ----

pub async fn get(db: &Db, user_id: &str) -> AppResult<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(user_id)
        .fetch_optional(db)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn identities(db: &Db, user_id: &str) -> AppResult<Vec<UserIdentity>> {
    let rows = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE user_id = ? ORDER BY provider",
    )
    .bind(user_id)
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// 登录用查找：只认 email / phone，且账户必须启用。
/// 刻意不含第三方 subject——没人会手动输入一串不透明 id 来登录。
pub async fn find_for_login(db: &Db, identifier: &str) -> AppResult<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u \
         JOIN user_identities i ON i.user_id = u.id \
         WHERE i.identifier = ? AND i.provider IN (?, ?) AND u.is_active = 1",
    )
    .bind(identifier)
    .bind(PROVIDER_EMAIL)
    .bind(PROVIDER_PHONE)
    .fetch_optional(db)
    .await?;
    Ok(user)
}

/// 管理端查找：匹配任意 provider（含 google/apple subject），且包含已停用账户。
/// 同一字符串可能在不同 provider 下属于不同账户，命中多个时要求调用方用
/// `provider` 消歧，绝不猜测删哪一个。
pub async fn find_any(
    db: &Db,
    identifier: &str,
    provider: Option<&str>,
) -> AppResult<User> {
    let ids: Vec<(String,)> = match provider {
        Some(p) => sqlx::query_as(
            "SELECT DISTINCT user_id FROM user_identities WHERE identifier = ? AND provider = ?",
        )
        .bind(identifier)
        .bind(p)
        .fetch_all(db)
        .await?,
        None => {
            sqlx::query_as("SELECT DISTINCT user_id FROM user_identities WHERE identifier = ?")
                .bind(identifier)
                .fetch_all(db)
                .await?
        }
    };

    match ids.len() {
        0 => Err(AppError::NotFound),
        1 => get(db, &ids[0].0).await,
        n => Err(AppError::Conflict(format!(
            "标识 {identifier} 命中 {n} 个账户，请用 provider 指定"
        ))),
    }
}

pub async fn password_credential(db: &Db, user_id: &str) -> AppResult<Option<UserIdentity>> {
    let row = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(PROVIDER_PASSWORD)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

/// 取出一条用户自有的标识；若被第三方托管则拒绝——那属于 Google/Apple，
/// 只能去对方那里改。所有增删改入口都走这里，规则不会在各处走样。
pub async fn user_owned_identity(
    db: &Db,
    user_id: &str,
    provider: &str,
) -> AppResult<UserIdentity> {
    let identity = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(db)
    .await?
    .ok_or(AppError::NotFound)?;

    if let Some(owner) = &identity.managed_by {
        return Err(AppError::ForbiddenReason(format!(
            "该 {provider} 由 {owner} 托管，请到 {owner} 帐户中修改"
        )));
    }
    Ok(identity)
}

/// 账户当前可用于接收「旧标识」验证码的任意一条自有标识。
/// 新增某个 provider 时没有同类旧值可challenge，就退回到已有的另一条。
pub async fn any_user_owned_identity(db: &Db, user_id: &str) -> AppResult<UserIdentity> {
    let all = identities(db, user_id).await?;
    all.into_iter()
        .find(|i| USER_OWNED_PROVIDERS.contains(&i.provider.as_str()) && i.is_user_owned())
        .ok_or_else(|| {
            AppError::ForbiddenReason("该账户没有可编辑的标识（由第三方登录托管）".into())
        })
}

// ---- 写入 ----

/// 建立账户：users 行 + password 凭证 + email/phone 标识，全在一个事务里。
pub async fn create(
    db: &Db,
    username: &str,
    password_plain: &str,
    email: Option<&str>,
    phone: Option<&str>,
) -> AppResult<User> {
    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let hash = password::hash(password_plain)?;

    let mut tx = db.begin().await?;

    sqlx::query(
        "INSERT INTO users (id, username, created_at, updated_at, is_active) VALUES (?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(username)
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // password 凭证的 identifier 仅作留痕，查找一律走 email/phone 标识行。
    insert_identity(
        &mut tx,
        &id,
        PROVIDER_PASSWORD,
        email.or(phone).unwrap_or(&id),
        Some(&hash),
        None,
        &now,
    )
    .await?;

    if let Some(e) = email {
        insert_identity(&mut tx, &id, PROVIDER_EMAIL, e, None, None, &now).await?;
    }
    if let Some(p) = phone {
        insert_identity(&mut tx, &id, PROVIDER_PHONE, p, None, None, &now).await?;
    }

    tx.commit().await?;
    get(db, &id).await
}

async fn insert_identity(
    tx: &mut sqlx::SqliteConnection,
    user_id: &str,
    provider: &str,
    identifier: &str,
    secret: Option<&str>,
    verified_at: Option<&str>,
    now: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO user_identities \
         (id, user_id, provider, identifier, secret, verified_at, managed_by, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(provider)
    .bind(identifier)
    .bind(secret)
    .bind(verified_at)
    .bind(now)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(duplicate_identifier)?;
    Ok(())
}

/// 把 UNIQUE(provider, identifier) 冲突翻译成 409，而不是 500。
fn duplicate_identifier(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &e
        && db_err.is_unique_violation()
    {
        return AppError::Conflict("该邮箱或手机号已被使用".into());
    }
    AppError::Database(e)
}

pub async fn update_username(db: &Db, user_id: &str, username: &str) -> AppResult<User> {
    sqlx::query("UPDATE users SET username = ?, updated_at = ? WHERE id = ?")
        .bind(username)
        .bind(now_rfc3339())
        .bind(user_id)
        .execute(db)
        .await?;
    get(db, user_id).await
}

pub async fn change_password(db: &Db, user_id: &str, new_password: &str) -> AppResult<()> {
    let hash = password::hash(new_password)?;
    sqlx::query(
        "UPDATE user_identities SET secret = ?, updated_at = ? WHERE user_id = ? AND provider = ?",
    )
    .bind(&hash)
    .bind(now_rfc3339())
    .bind(user_id)
    .bind(PROVIDER_PASSWORD)
    .execute(db)
    .await?;
    Ok(())
}

/// 新增或改写一条用户自有标识；始终以未验证状态落库，验证是独立的后续动作。
pub async fn set_identifier(
    db: &Db,
    user_id: &str,
    provider: &str,
    value: &str,
) -> AppResult<()> {
    let now = now_rfc3339();
    let existing = sqlx::query_as::<_, UserIdentity>(
        "SELECT * FROM user_identities WHERE user_id = ? AND provider = ?",
    )
    .bind(user_id)
    .bind(provider)
    .fetch_optional(db)
    .await?;

    match existing {
        Some(_) => {
            sqlx::query(
                "UPDATE user_identities SET identifier = ?, verified_at = NULL, updated_at = ? \
                 WHERE user_id = ? AND provider = ?",
            )
            .bind(value)
            .bind(&now)
            .bind(user_id)
            .bind(provider)
            .execute(db)
            .await
            .map_err(duplicate_identifier)?;
        }
        None => {
            let mut conn = db.acquire().await?;
            insert_identity(&mut conn, user_id, provider, value, None, None, &now).await?;
        }
    }
    Ok(())
}

pub async fn mark_verified(db: &Db, user_id: &str, provider: &str) -> AppResult<()> {
    let now = now_rfc3339();
    sqlx::query(
        "UPDATE user_identities SET verified_at = ?, updated_at = ? WHERE user_id = ? AND provider = ?",
    )
    .bind(&now)
    .bind(&now)
    .bind(user_id)
    .bind(provider)
    .execute(db)
    .await?;
    Ok(())
}

/// 删除一条自有标识；账户必须至少保留一条，否则将无人能再登录。
pub async fn remove_identifier(db: &Db, user_id: &str, provider: &str) -> AppResult<()> {
    let all = identities(db, user_id).await?;
    let remaining = all
        .iter()
        .filter(|i| USER_OWNED_PROVIDERS.contains(&i.provider.as_str()) && i.provider != provider)
        .count();
    if remaining == 0 {
        return Err(AppError::BadRequest(
            "至少需保留一个邮箱或手机号，否则将无法登录".into(),
        ));
    }

    sqlx::query("DELETE FROM user_identities WHERE user_id = ? AND provider = ?")
        .bind(user_id)
        .bind(provider)
        .execute(db)
        .await?;
    Ok(())
}

// ---- 管理操作 ----

pub async fn list_all(db: &Db) -> AppResult<Vec<(User, Vec<UserIdentity>)>> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at")
        .fetch_all(db)
        .await?;

    let mut out = Vec::with_capacity(users.len());
    for u in users {
        let ids = identities(db, &u.id).await?;
        out.push((u, ids));
    }
    Ok(out)
}

/// 软删除账户：全部文件进回收站（共用同一 deleted_at 作为批次标记），
/// 账户停用。用户行永不物理删除——回收站清理任务只清文件。
pub async fn soft_remove(
    db: &Db,
    retention_days: i64,
    user_id: &str,
) -> AppResult<(User, u64)> {
    let user = get(db, user_id).await?;
    if user.is_active == 0 {
        return Err(AppError::Conflict("该账户已被停用".into()));
    }

    let at = now_rfc3339();
    let count = trash_service::soft_delete_all_for_user(db, retention_days, user_id, &at).await?;

    sqlx::query("UPDATE users SET is_active = 0, deleted_at = ?, updated_at = ? WHERE id = ?")
        .bind(&at)
        .bind(&at)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok((get(db, user_id).await?, count))
}

/// 还原账户：只还原当初随删除一起进回收站的项目（按 deleted_at 批次匹配），
/// 用户自己先前删掉的东西保持在回收站里。
pub async fn restore(db: &Db, user_id: &str) -> AppResult<(User, u64)> {
    let user = get(db, user_id).await?;
    let Some(deleted_at) = user.deleted_at.clone() else {
        return Err(AppError::Conflict("该账户未被停用，无需还原".into()));
    };

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM trash WHERE owner_id = ? AND deleted_at = ?")
            .bind(user_id)
            .bind(&deleted_at)
            .fetch_all(db)
            .await?;

    let mut restored = 0u64;
    for (trash_id,) in &rows {
        match trash_service::restore(db, user_id, trash_id).await {
            Ok(_) => restored += 1,
            Err(e) => {
                tracing::warn!(trash_id = %trash_id, error = ?e, "还原账户项目失败，跳过");
            }
        }
    }

    let now = now_rfc3339();
    sqlx::query("UPDATE users SET is_active = 1, deleted_at = NULL, updated_at = ? WHERE id = ?")
        .bind(&now)
        .bind(user_id)
        .execute(db)
        .await?;

    Ok((get(db, user_id).await?, restored))
}

// ---- 标识规范化 ----

/// 邮箱：去空白 + 转小写，必须含 `@` 且两侧非空。
pub fn normalize_email(raw: &str) -> AppResult<String> {
    let e = raw.trim().to_lowercase();
    let mut parts = e.splitn(2, '@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(AppError::BadRequest("邮箱格式不正确".into()));
    }
    Ok(e)
}

/// 手机号：规范到 E.164（`+` 开头，8–15 位数字）。
pub fn normalize_phone(raw: &str) -> AppResult<String> {
    let p: String = raw.chars().filter(|c| !c.is_whitespace() && *c != '-').collect();
    let Some(digits) = p.strip_prefix('+') else {
        return Err(AppError::BadRequest(
            "手机号需为 E.164 格式，以 + 和国码开头".into(),
        ));
    };
    if !(8..=15).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(AppError::BadRequest(
            "手机号需为 + 开头加 8–15 位数字".into(),
        ));
    }
    Ok(p)
}

/// 按 provider 规范化标识值。
pub fn normalize_for(provider: &str, raw: &str) -> AppResult<String> {
    match provider {
        PROVIDER_EMAIL => normalize_email(raw),
        PROVIDER_PHONE => normalize_phone(raw),
        _ => Err(AppError::BadRequest(
            "provider 只能是 email 或 phone".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_is_lowercased_and_trimmed() {
        assert_eq!(normalize_email("  Dad@GMail.com ").unwrap(), "dad@gmail.com");
    }

    #[test]
    fn email_requires_dotted_domain() {
        assert!(normalize_email("dad@localhost").is_err());
        assert!(normalize_email("@gmail.com").is_err());
        assert!(normalize_email("dad@").is_err());
    }

    #[test]
    fn phone_keeps_plus_and_strips_separators() {
        assert_eq!(normalize_phone("+886 912-345-678").unwrap(), "+886912345678");
    }

    #[test]
    fn phone_requires_e164() {
        assert!(normalize_phone("0912345678").is_err());
        assert!(normalize_phone("+123").is_err());
        assert!(normalize_phone("+12345678901234567").is_err());
        assert!(normalize_phone("+886abc12345").is_err());
    }

    #[test]
    fn normalize_for_rejects_unknown_provider() {
        assert!(normalize_for("google", "x").is_err());
    }
}
