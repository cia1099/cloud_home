//! JWT 签发与验证。

use chrono::{Duration, Utc};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// JWT Claims。`sub` 为用户 id。
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

/// 签发一个有效期为 `expires_in_hours` 小时的 token，返回 (token, 过期时间 RFC3339)。
pub fn issue(secret: &str, user_id: &str, expires_in_hours: i64) -> AppResult<(String, String)> {
    let now = Utc::now();
    let exp = now + Duration::hours(expires_in_hours);
    let claims = Claims {
        sub: user_id.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    let token = encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Other(anyhow::anyhow!("JWT 签发失败: {e}")))?;

    Ok((token, exp.to_rfc3339()))
}

/// 验证 token 并返回 Claims，失败返回 `401`。
pub fn verify(secret: &str, token: &str) -> AppResult<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::Unauthorized)?;

    Ok(data.claims)
}
