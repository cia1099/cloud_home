//! 一次性验证码：签发、投递、校验。
//!
//! 两种用途：
//! - `identifier_old`：证明持有「现有标识」，用于放行标识的增 / 改 / 删；
//! - `identifier_verify`：证明控制「本人已登记的标识」，用于置 `verified_at`。
//!
//! 用途是隔离的——`identifier_old` 的码不能拿去当 `identifier_verify` 用。

use chrono::{Duration, Utc};

use crate::auth::password;
use crate::db::Db;
use crate::error::{AppError, AppResult};
use crate::util::now_rfc3339;

pub const PURPOSE_IDENTIFIER_OLD: &str = "identifier_old";
pub const PURPOSE_IDENTIFIER_VERIFY: &str = "identifier_verify";

/// 验证码有效期。
const TTL_MINUTES: i64 = 5;
/// 单个码允许的最大尝试次数。
const MAX_ATTEMPTS: i64 = 5;
/// 同一目标每小时最多签发次数。
const MAX_PER_HOUR: i64 = 3;

/// 签发验证码并投递。成功返回 ()，验证码本身不回传给调用方。
pub async fn issue(
    db: &Db,
    user_id: Option<&str>,
    purpose: &str,
    destination: &str,
) -> AppResult<()> {
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let one_hour_ago = (now - Duration::hours(1)).to_rfc3339();

    // 只清理超出限流窗口的老行。窗口内的行必须留着——它们就是限流的计数依据，
    // 提前删掉（哪怕已作废）会让下面的 COUNT 永远归零，限流形同虚设。
    sqlx::query("DELETE FROM otp_codes WHERE created_at <= ?")
        .bind(&one_hour_ago)
        .execute(db)
        .await?;

    let (recent,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM otp_codes WHERE destination = ? AND created_at > ?",
    )
    .bind(destination)
    .bind(&one_hour_ago)
    .fetch_one(db)
    .await?;
    if recent >= MAX_PER_HOUR {
        return Err(AppError::BadRequest(
            "验证码请求过于频繁，请稍后再试".into(),
        ));
    }

    // 同一目标 + 用途下的旧码作废：标记为已消费而非删除，这样它既不能再用
    // （verify 只认 consumed_at IS NULL），又仍然计入上面的限流窗口。
    sqlx::query(
        "UPDATE otp_codes SET consumed_at = ? \
         WHERE destination = ? AND purpose = ? AND consumed_at IS NULL",
    )
    .bind(&now_str)
    .bind(destination)
    .bind(purpose)
    .execute(db)
    .await?;

    let code = generate_code();
    let expires_at = (now + Duration::minutes(TTL_MINUTES)).to_rfc3339();

    sqlx::query(
        "INSERT INTO otp_codes \
         (id, user_id, purpose, destination, code_hash, attempts, expires_at, consumed_at, created_at) \
         VALUES (?, ?, ?, ?, ?, 0, ?, NULL, ?)",
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(user_id)
    .bind(purpose)
    .bind(destination)
    .bind(password::hash(&code)?)
    .bind(&expires_at)
    .bind(&now_str)
    .execute(db)
    .await?;

    deliver(destination, &code);
    Ok(())
}

/// 校验验证码。成功后标记为已消费，同一个码不能再用第二次。
pub async fn verify(db: &Db, purpose: &str, destination: &str, code: &str) -> AppResult<()> {
    let now = now_rfc3339();

    let row: Option<(String, String, i64)> = sqlx::query_as(
        "SELECT id, code_hash, attempts FROM otp_codes \
         WHERE destination = ? AND purpose = ? AND consumed_at IS NULL AND expires_at > ? \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(destination)
    .bind(purpose)
    .bind(&now)
    .fetch_optional(db)
    .await?;

    let Some((id, hash, attempts)) = row else {
        crate::audit!(
            warn,
            "otp.verify_failed",
            "failure",
            reason = "no_active_code",
            purpose = %purpose,
            destination = %mask(destination),
            "验证码不存在或已过期"
        );
        return Err(AppError::BadRequest("验证码无效或已过期".into()));
    };

    if attempts >= MAX_ATTEMPTS {
        crate::audit!(
            warn,
            "otp.verify_failed",
            "failure",
            reason = "too_many_attempts",
            purpose = %purpose,
            destination = %mask(destination),
            "验证码尝试次数过多"
        );
        return Err(AppError::BadRequest(
            "尝试次数过多，请重新获取验证码".into(),
        ));
    }

    if !password::verify(code, &hash) {
        sqlx::query("UPDATE otp_codes SET attempts = attempts + 1 WHERE id = ?")
            .bind(&id)
            .execute(db)
            .await?;
        crate::audit!(
            warn,
            "otp.verify_failed",
            "failure",
            reason = "bad_code",
            purpose = %purpose,
            destination = %mask(destination),
            "验证码错误"
        );
        return Err(AppError::BadRequest("验证码错误".into()));
    }

    sqlx::query("UPDATE otp_codes SET consumed_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&id)
        .execute(db)
        .await?;
    Ok(())
}

/// 生成 6 位数字验证码。
fn generate_code() -> String {
    format!("{:06}", rand::random::<u32>() % 1_000_000)
}

/// 投递验证码。
///
/// **这是开发期占位实现**：尚无 SMTP / SMS 通道，验证码直接写进审计日志，
/// 便于本地联调。接入真实发送方时只需替换本函数——代码库其他任何地方
/// 都不知道验证码是怎么送出去的。
fn deliver(destination: &str, code: &str) {
    let channel = if destination.contains('@') {
        "email"
    } else {
        "sms"
    };
    crate::audit!(
        info,
        "otp.issued",
        "success",
        channel = %channel,
        destination = %mask(destination),
        code = %code,
        "验证码已签发（开发期：仅写入日志，未真实发送）"
    );
}

/// 遮蔽收件目标，仅保留足以辨认的首尾片段。
pub fn mask(destination: &str) -> String {
    match destination.split_once('@') {
        Some((local, domain)) => {
            let head: String = local.chars().take(2).collect();
            format!("{head}***@{domain}")
        }
        None => {
            let n = destination.chars().count();
            let tail: String = destination.chars().skip(n.saturating_sub(4)).collect();
            format!("***{tail}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_is_six_digits() {
        for _ in 0..200 {
            let c = generate_code();
            assert_eq!(c.len(), 6);
            assert!(c.chars().all(|ch| ch.is_ascii_digit()));
        }
    }

    #[test]
    fn mask_email_keeps_domain() {
        assert_eq!(mask("dad@gmail.com"), "da***@gmail.com");
    }

    #[test]
    fn mask_phone_keeps_last_four() {
        assert_eq!(mask("+886912345678"), "***5678");
    }
}
