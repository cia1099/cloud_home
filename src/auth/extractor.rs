//! `AuthUser` extractor：Cookie `auth_token` 优先，回退 `Authorization: Bearer`。

use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;

use crate::auth::jwt;
use crate::error::AppError;
use crate::state::AppState;

/// 已认证用户，仅携带用户 id。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = extract_token(parts).ok_or(AppError::Unauthorized)?;
        let claims = jwt::verify(&state.config.jwt_secret, &token)?;
        Ok(AuthUser {
            user_id: claims.sub,
        })
    }
}

/// 从请求中取出 JWT：先找 Cookie `auth_token`，再找 `Authorization: Bearer`。
fn extract_token(parts: &Parts) -> Option<String> {
    if let Some(cookie_header) = parts.headers.get(COOKIE).and_then(|v| v.to_str().ok())
        && let Some(token) = parse_cookie(cookie_header, "auth_token")
    {
        return Some(token);
    }

    parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
}

/// 从 Cookie 头中解析指定名称的值。
fn parse_cookie(header: &str, name: &str) -> Option<String> {
    header.split(';').find_map(|pair| {
        let mut parts = pair.trim().splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();
        (key == name).then(|| value.to_string())
    })
}
