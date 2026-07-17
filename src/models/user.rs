use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库中的用户行。
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: i64,
}

/// 注册请求体。
#[derive(Debug, Deserialize)]
pub struct CreateUserDto {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// 登录请求体：`identifier` 可为 username 或 email。
#[derive(Debug, Deserialize)]
pub struct LoginDto {
    pub identifier: String,
    pub password: String,
}

/// 对外返回的用户信息（不含密码哈希）。
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            created_at: u.created_at,
        }
    }
}
