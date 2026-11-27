//! 用户与身份（凭证 / 标识）业务逻辑。
//!
//! 设计要点：`users` 只存个人资料，所有凭证与标识都在 `user_identities`，
//! 每条标识用 `managed_by` 记录归属——NULL 为用户自有（可编辑），
//! `google`/`apple` 为第三方托管（本地只读）。同一账户可两者兼有：
//! 先用邮箱注册、之后绑定 Google 的用户，其邮箱仍归自己编辑。

use crate::auth::password;
use crate::config::{RegistrationMode, RegistrationPolicy};
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::models::user::{
    APPROVAL_APPROVED, APPROVAL_PENDING, PROVIDER_EMAIL, PROVIDER_PASSWORD, PROVIDER_PHONE,
    USER_OWNED_PROVIDERS, User, UserIdentity,
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

/// 登录用查找：只认 email / phone。账户须启用，或处于 pending / rejected 且未被软删除——
/// 后两者要找出来，是为了在密码正确时给出 `ACCOUNT_PENDING` / `ACCOUNT_REJECTED`
/// 而不是笼统的 401；它们仍是 `is_active = 0`，拿不到令牌。
/// 刻意不含第三方 subject——没人会手动输入一串不透明 id 来登录。
pub async fn find_for_login(db: &Db, identifier: &str) -> AppResult<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT u.* FROM users u \
         JOIN user_identities i ON i.user_id = u.id \
         WHERE i.identifier = ? AND i.provider IN (?, ?) \
           AND (u.is_active = 1 \
                OR (u.approval_status IN ('pending', 'rejected') AND u.deleted_at IS NULL))",
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

/// 注册请求的账户资料（已校验、已规范化）。
pub struct NewAccount<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub email: Option<&'a str>,
    pub phone: Option<&'a str>,
}

/// 注册请求的来源信息。
pub struct RegisterSource {
    /// 客户端 IP（`registered_ip` 与按 IP 限流用）；取不到则为 None。
    pub ip: Option<String>,
    /// 来源是否回环 / 内网（引导规则要求）。
    pub is_local: bool,
}

/// 注册结果：决定 handler 是签发令牌还是只回 202。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterOutcome {
    /// `open` 模式：直接启用。
    Open,
    /// 引导：users 表原本完全为空且来源为本机 / 内网，自动批准。
    Bootstrap,
    /// 等待管理员批准。
    Pending,
}

impl RegisterOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Bootstrap => "bootstrap",
            Self::Pending => "pending",
        }
    }
}

/// 建立账户：users 行 + password 凭证 + email/phone 标识，全在一个事务里。
///
/// 流程：模式检查 → 限流 / 队列上限（仅 approval）→ 被拒绝的旧申请改回 pending（无冷却、不限次数）→ 事务。
/// 事务的第一条语句就是 INSERT（拿到 SQLite 写锁），所以「users 表是否原本为空」
/// 的判定与插入串行化——两个并发的首次注册不可能都被引导批准。
pub async fn register(
    db: &Db,
    policy: &RegistrationPolicy,
    acct: NewAccount<'_>,
    source: RegisterSource,
) -> AppResult<(User, RegisterOutcome)> {
    if policy.mode == RegistrationMode::Closed {
        return Err(AppError::RegistrationClosed);
    }
    if policy.mode == RegistrationMode::Approval {
        check_registration_load(db, policy, source.ip.as_deref()).await?;
    }
    if let Some(old_id) = rejected_match(db, acct.email, acct.phone).await? {
        let user = reapply_rejected(db, &old_id, &acct, &source).await?;
        return Ok((user, RegisterOutcome::Pending));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let hash = password::hash(acct.password)?;
    let open = policy.mode == RegistrationMode::Open;
    let (status, active, decided_by) = if open {
        (APPROVAL_APPROVED, 1, Some("open"))
    } else {
        (APPROVAL_PENDING, 0, None)
    };

    let mut tx = db.begin().await?;

    sqlx::query(
        "INSERT INTO users (id, username, created_at, updated_at, is_active, \
         approval_status, approval_decided_at, approval_decided_by, registered_ip) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(acct.username)
    .bind(&now)
    .bind(&now)
    .bind(active)
    .bind(status)
    .bind(decided_by.map(|_| now.as_str()))
    .bind(decided_by)
    .bind(source.ip.as_deref())
    .execute(&mut *tx)
    .await?;

    let mut outcome = if open {
        RegisterOutcome::Open
    } else {
        RegisterOutcome::Pending
    };
    if !open && source.is_local {
        // 刚插入的这一行是表里唯一一行 ⇒ 之前（含软删除行）完全为空。
        let promoted = sqlx::query(
            "UPDATE users SET approval_status = 'approved', is_active = 1, \
             approval_decided_at = ?, approval_decided_by = 'bootstrap' \
             WHERE id = ? AND (SELECT COUNT(*) FROM users) = 1",
        )
        .bind(&now)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
        if promoted.rows_affected() == 1 {
            outcome = RegisterOutcome::Bootstrap;
        }
    }

    // password 凭证的 identifier 仅作留痕，查找一律走 email/phone 标识行。
    insert_identity(
        &mut tx,
        &id,
        PROVIDER_PASSWORD,
        acct.email.or(acct.phone).unwrap_or(&id),
        Some(&hash),
        None,
        &now,
    )
    .await?;

    if let Some(e) = acct.email {
        insert_identity(&mut tx, &id, PROVIDER_EMAIL, e, None, None, &now).await?;
    }
    if let Some(p) = acct.phone {
        insert_identity(&mut tx, &id, PROVIDER_PHONE, p, None, None, &now).await?;
    }

    tx.commit().await?;
    Ok((get(db, &id).await?, outcome))
}

/// approval 模式的滥用限制：待批准队列上限与按来源 IP 的小时限流，超限返回 429。
async fn check_registration_load(
    db: &Db,
    policy: &RegistrationPolicy,
    ip: Option<&str>,
) -> AppResult<()> {
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM users WHERE approval_status = 'pending'")
            .fetch_one(db)
            .await?;
    if pending >= policy.pending_max {
        crate::audit!(
            warn,
            "user.register",
            "failure",
            reason = "pending_max",
            pending = pending,
            "注册被拒：待批准账户已达上限"
        );
        return Err(AppError::RegistrationBusy(
            "待批准的申请过多，请稍后再试".into(),
        ));
    }

    if let Some(ip) = ip {
        let since = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let (recent,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM users WHERE registered_ip = ? AND created_at > ?")
                .bind(ip)
                .bind(&since)
                .fetch_one(db)
                .await?;
        if recent >= policy.rate_per_ip_per_hour {
            crate::audit!(
                warn,
                "user.register",
                "failure",
                reason = "rate_limited",
                client_ip = %ip,
                "注册被拒：同一来源注册过于频繁"
            );
            return Err(AppError::RegistrationBusy(
                "该来源注册过于频繁，请稍后再试".into(),
            ));
        }
    }
    Ok(())
}

/// 占着这些标识的 rejected 旧账户（若有）：重新注册会把它改回 pending，而不是新建一行。
async fn rejected_match(
    db: &Db,
    email: Option<&str>,
    phone: Option<&str>,
) -> AppResult<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT u.id FROM users u \
         JOIN user_identities i ON i.user_id = u.id \
         WHERE u.approval_status = 'rejected' AND u.deleted_at IS NULL \
           AND i.provider IN (?, ?) AND i.identifier IN (?, ?) \
         ORDER BY u.created_at LIMIT 1",
    )
    .bind(PROVIDER_EMAIL)
    .bind(PROVIDER_PHONE)
    .bind(email.unwrap_or(""))
    .bind(phone.unwrap_or(""))
    .fetch_optional(db)
    .await?;

    Ok(row.map(|(id,)| id))
}

/// 被拒绝的申请人再次注册：同一行改回 pending，按全新申请处理（改写名字、密码、来源 IP，
/// `created_at` 重置以便计入限流与过期），仍保持 `is_active = 0`，不签发令牌。
/// 重试次数与间隔都不设限；限流、待批准上限已在调用前检查过。
async fn reapply_rejected(
    db: &Db,
    old_id: &str,
    acct: &NewAccount<'_>,
    source: &RegisterSource,
) -> AppResult<User> {
    let hash = password::hash(acct.password)?;
    let now = now_rfc3339();
    let mut tx = db.begin().await?;

    // 仅当仍是 rejected 时才改写（与并发的审批决定互斥）。
    let updated = sqlx::query(
        "UPDATE users SET username = ?, is_active = 0, approval_status = 'pending', \
         approval_decided_at = NULL, approval_decided_by = NULL, approval_note = NULL, \
         registered_ip = ?, created_at = ?, updated_at = ? \
         WHERE id = ? AND approval_status = 'rejected' AND deleted_at IS NULL",
    )
    .bind(acct.username)
    .bind(source.ip.as_deref())
    .bind(&now)
    .bind(&now)
    .bind(old_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(AppError::Conflict("该申请状态已变化，请重试".into()));
    }

    sqlx::query(
        "UPDATE user_identities SET secret = ?, updated_at = ? WHERE user_id = ? AND provider = ?",
    )
    .bind(&hash)
    .bind(&now)
    .bind(old_id)
    .bind(PROVIDER_PASSWORD)
    .execute(&mut *tx)
    .await?;

    // 新提交的邮箱 / 手机号：已有同类标识则改写，没有则新增。
    for (provider, value) in [(PROVIDER_EMAIL, acct.email), (PROVIDER_PHONE, acct.phone)] {
        let Some(value) = value else { continue };
        let updated = sqlx::query(
            "UPDATE user_identities SET identifier = ?, updated_at = ? \
             WHERE user_id = ? AND provider = ?",
        )
        .bind(value)
        .bind(&now)
        .bind(old_id)
        .bind(provider)
        .execute(&mut *tx)
        .await
        .map_err(duplicate_identifier)?;
        if updated.rows_affected() == 0 {
            insert_identity(&mut tx, old_id, provider, value, None, None, &now).await?;
        }
    }

    tx.commit().await?;
    crate::audit!(
        info,
        "user.register",
        "reapply",
        user_id = %old_id,
        "被拒绝的申请人重新提交申请，账户改回待批准"
    );
    get(db, old_id).await
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

/// 列出账户；`status` 为 None 表示不按审批状态过滤。
pub async fn list_all(
    db: &Db,
    status: Option<&str>,
) -> AppResult<Vec<(User, Vec<UserIdentity>)>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE (? IS NULL OR approval_status = ?) ORDER BY created_at",
    )
    .bind(status)
    .bind(status)
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
    // 还原只撤销软删除，绝不能顺带放行没通过审批的账户。
    if user.approval_status != APPROVAL_APPROVED {
        return Err(AppError::Conflict("该账户未通过审批，不能还原".into()));
    }

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
    sqlx::query(
        "UPDATE users SET is_active = 1, deleted_at = NULL, updated_at = ? \
         WHERE id = ? AND approval_status = 'approved'",
    )
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

    // ---- 注册 / 审批 ----

    use crate::db::test_pool;

    fn policy(mode: RegistrationMode) -> RegistrationPolicy {
        RegistrationPolicy {
            mode,
            rate_per_ip_per_hour: 100,
            pending_max: 100,
            pending_expire_days: 14,
        }
    }

    async fn reg(
        db: &Db,
        p: &RegistrationPolicy,
        email: &str,
        ip: &str,
        is_local: bool,
    ) -> AppResult<(User, RegisterOutcome)> {
        register(
            db,
            p,
            NewAccount {
                username: "n",
                password: "password123",
                email: Some(email),
                phone: None,
            },
            RegisterSource {
                ip: Some(ip.into()),
                is_local,
            },
        )
        .await
    }

    #[tokio::test]
    async fn first_local_account_is_bootstrapped_and_second_is_pending() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);

        let (a, out) = reg(&db, &p, "a@x.com", "127.0.0.1", true).await.unwrap();
        assert_eq!(out, RegisterOutcome::Bootstrap);
        assert_eq!((a.is_active, a.approval_status.as_str()), (1, "approved"));
        assert_eq!(a.approval_decided_by.as_deref(), Some("bootstrap"));

        let (b, out) = reg(&db, &p, "b@x.com", "127.0.0.1", true).await.unwrap();
        assert_eq!(out, RegisterOutcome::Pending);
        assert_eq!((b.is_active, b.approval_status.as_str()), (0, "pending"));
        assert_eq!(b.registered_ip.as_deref(), Some("127.0.0.1"));
    }

    #[tokio::test]
    async fn remote_first_account_is_pending() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (u, out) = reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();
        assert_eq!(out, RegisterOutcome::Pending);
        assert_eq!(u.is_active, 0);
        // 之后即使本机来的也不再是引导（表非空）。
        let (_, out) = reg(&db, &p, "b@x.com", "127.0.0.1", true).await.unwrap();
        assert_eq!(out, RegisterOutcome::Pending);
    }

    #[tokio::test]
    async fn soft_deleted_rows_count_as_non_empty_for_bootstrap() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (a, _) = reg(&db, &p, "a@x.com", "127.0.0.1", true).await.unwrap();
        sqlx::query("UPDATE users SET is_active = 0, deleted_at = ? WHERE id = ?")
            .bind(now_rfc3339())
            .bind(&a.id)
            .execute(&db)
            .await
            .unwrap();
        let (_, out) = reg(&db, &p, "b@x.com", "127.0.0.1", true).await.unwrap();
        assert_eq!(out, RegisterOutcome::Pending);
    }

    #[tokio::test]
    async fn concurrent_first_registrations_bootstrap_at_most_one() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (r1, r2) = tokio::join!(
            reg(&db, &p, "a@x.com", "127.0.0.1", true),
            reg(&db, &p, "b@x.com", "127.0.0.1", true),
        );
        let boots = [r1, r2]
            .into_iter()
            .filter(|r| matches!(r, Ok((_, RegisterOutcome::Bootstrap))))
            .count();
        assert_eq!(boots, 1);
    }

    #[tokio::test]
    async fn open_mode_activates_immediately() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Open);
        let (u, out) = reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();
        assert_eq!(out, RegisterOutcome::Open);
        assert_eq!((u.is_active, u.approval_status.as_str()), (1, "approved"));
    }

    #[tokio::test]
    async fn closed_mode_refuses_registration() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Closed);
        let err = reg(&db, &p, "a@x.com", "127.0.0.1", true).await.unwrap_err();
        assert!(matches!(err, AppError::RegistrationClosed));
    }

    #[tokio::test]
    async fn pending_max_returns_busy() {
        let db = test_pool().await;
        let p = RegistrationPolicy {
            pending_max: 1,
            ..policy(RegistrationMode::Approval)
        };
        reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();
        let err = reg(&db, &p, "b@x.com", "9.9.9.9", false).await.unwrap_err();
        assert!(matches!(err, AppError::RegistrationBusy(_)));
    }

    #[tokio::test]
    async fn per_ip_rate_limit_returns_busy_only_for_that_ip() {
        let db = test_pool().await;
        let p = RegistrationPolicy {
            rate_per_ip_per_hour: 2,
            ..policy(RegistrationMode::Approval)
        };
        reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();
        reg(&db, &p, "b@x.com", "8.8.8.8", false).await.unwrap();
        let err = reg(&db, &p, "c@x.com", "8.8.8.8", false).await.unwrap_err();
        assert!(matches!(err, AppError::RegistrationBusy(_)));
        reg(&db, &p, "c@x.com", "9.9.9.9", false).await.unwrap();
    }

    #[tokio::test]
    async fn rejected_applicant_can_reapply_without_limit_then_be_approved() {
        use crate::services::approval_service::{Decision, decide};
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (u, _) = reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();

        // 连续多次「拒绝 → 立即重新注册」：不设冷却、不限次数，始终是同一账户回到 pending。
        for _ in 0..3 {
            decide(&db, &u.id, Decision::Reject, "admin-endpoint", None, true)
                .await
                .unwrap();
            assert_eq!(get(&db, &u.id).await.unwrap().approval_status, "rejected");

            let (n, out) = reg(&db, &p, "a@x.com", "9.9.9.9", false).await.unwrap();
            assert_eq!(out, RegisterOutcome::Pending);
            assert_eq!(n.id, u.id);
            assert_eq!(n.is_active, 0);
            assert_eq!(n.approval_status, "pending");
            assert!(n.approval_decided_at.is_none());
            assert_eq!(n.registered_ip.as_deref(), Some("9.9.9.9"));
        }

        // 重新提交的密码生效；批准后账户启用。
        decide(&db, &u.id, Decision::Approve, "admin-endpoint", None, true)
            .await
            .unwrap();
        let approved = get(&db, &u.id).await.unwrap();
        assert_eq!(approved.approval_status, "approved");
        assert_eq!(approved.is_active, 1);
    }

    #[tokio::test]
    async fn login_lookup_finds_pending_and_rejected_but_not_soft_deleted() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (pending, _) = reg(&db, &p, "p@x.com", "8.8.8.8", false).await.unwrap();
        let (gone, _) = reg(&db, &p, "g@x.com", "8.8.8.8", false).await.unwrap();

        let found = find_for_login(&db, "p@x.com").await.unwrap().unwrap();
        assert_eq!(found.id, pending.id);
        assert_eq!(found.is_active, 0);

        sqlx::query("UPDATE users SET approval_status = 'approved', deleted_at = ? WHERE id = ?")
            .bind(now_rfc3339())
            .bind(&gone.id)
            .execute(&db)
            .await
            .unwrap();
        assert!(find_for_login(&db, "g@x.com").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn restore_never_reactivates_a_non_approved_account() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        let (u, _) = reg(&db, &p, "a@x.com", "8.8.8.8", false).await.unwrap();

        // 未软删除的 pending：本就无从还原。
        assert!(restore(&db, &u.id).await.is_err());

        // 即便人为造出「pending 且带 deleted_at」，还原也必须拒绝并保持停用。
        sqlx::query("UPDATE users SET deleted_at = ? WHERE id = ?")
            .bind(now_rfc3339())
            .bind(&u.id)
            .execute(&db)
            .await
            .unwrap();
        assert!(matches!(
            restore(&db, &u.id).await.unwrap_err(),
            AppError::Conflict(_)
        ));
        assert_eq!(get(&db, &u.id).await.unwrap().is_active, 0);
    }

    #[tokio::test]
    async fn list_all_filters_by_approval_status() {
        let db = test_pool().await;
        let p = policy(RegistrationMode::Approval);
        reg(&db, &p, "a@x.com", "127.0.0.1", true).await.unwrap(); // bootstrap → approved
        reg(&db, &p, "b@x.com", "8.8.8.8", false).await.unwrap(); // pending

        assert_eq!(list_all(&db, None).await.unwrap().len(), 2);
        let pending = list_all(&db, Some("pending")).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].0.approval_status, "pending");
        assert_eq!(list_all(&db, Some("rejected")).await.unwrap().len(), 0);
    }
}
