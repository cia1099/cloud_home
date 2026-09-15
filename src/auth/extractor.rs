//! `AuthUser` extractor：Cookie `auth_token` 优先，回退 `Authorization: Bearer`。

use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, COOKIE};
use axum::http::request::Parts;

use crate::auth::jwt;
use crate::error::AppError;
use crate::state::AppState;

/// 已认证用户，携带用户 id 与用户名。
///
/// `username` 目前仅在下方提取时被写入当前请求 span（`span.record`），
/// 从而让本次请求的所有日志行自动带上可读用户名——handler 内部不需要
/// 再显式传递。编译器看不到这层间接使用，故标注 `#[allow(dead_code)]`。
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,
    #[allow(dead_code)]
    pub username: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = match extract_token(parts) {
            Some(t) => t,
            None => {
                crate::audit!(warn, "auth.token_rejected", "failure", reason = "token_missing", "缺少认证令牌");
                return Err(AppError::Unauthorized);
            }
        };
        let claims = jwt::verify(&state.config.jwt_secret, &token)?;

        // 回填当前请求 span 的 user_id / username，使本次请求的所有日志行
        // （含 error.rs 中无上下文的那条）都带上可读身份。
        let span = tracing::Span::current();
        span.record("user_id", claims.sub.as_str());
        span.record("username", claims.username.as_str());

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
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
