//! 认证端点：注册、登录、登出、获取当前用户。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header::SET_COOKIE};
use serde_json::{Value, json};

use crate::auth::{AuthUser, jwt, password};
use crate::error::{AppError, AppResult};
use crate::models::user::{CreateUserDto, LoginDto, User, UserResponse};
use crate::state::AppState;
use crate::util::now_rfc3339;

/// POST /auth/register
pub async fn register(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> AppResult<(StatusCode, HeaderMap, Json<Value>)> {
    validate_registration(&dto)?;

    // 唯一性检查（username / email）。
    let existing: Option<(String,)> =
        sqlx::query_as("SELECT id FROM users WHERE username = ? OR email = ?")
            .bind(&dto.username)
            .bind(&dto.email)
            .fetch_optional(&state.db)
            .await?;
    if existing.is_some() {
        return Err(AppError::Conflict("用户名或邮箱已被注册".into()));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = now_rfc3339();
    let hash = password::hash(&dto.password)?;

    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, created_at, updated_at, is_active) \
         VALUES (?, ?, ?, ?, ?, ?, 1)",
    )
    .bind(&id)
    .bind(&dto.username)
    .bind(&dto.email)
    .bind(&hash)
    .bind(&now)
    .bind(&now)
    .execute(&state.db)
    .await?;

    let user = fetch_user(&state, &id).await?;
    issue_auth_response(&state, user, StatusCode::CREATED)
}

/// POST /auth/login
pub async fn login(
    State(state): State<AppState>,
    Json(dto): Json<LoginDto>,
) -> AppResult<(StatusCode, HeaderMap, Json<Value>)> {
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM users WHERE (username = ? OR email = ?) AND is_active = 1",
    )
    .bind(&dto.identifier)
    .bind(&dto.identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !password::verify(&dto.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    issue_auth_response(&state, user, StatusCode::OK)
}

/// POST /auth/logout
pub async fn logout(_user: AuthUser) -> AppResult<(HeaderMap, Json<Value>)> {
    let mut headers = HeaderMap::new();
    let clear = "auth_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";
    headers.insert(SET_COOKIE, clear.parse().unwrap());
    Ok((headers, Json(json!({ "data": { "ok": true } }))))
}

/// GET /auth/me
pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<Value>> {
    let u = fetch_user(&state, &user.user_id).await?;
    Ok(Json(json!({ "data": UserResponse::from(u) })))
}

// ---- 辅助 ----

fn validate_registration(dto: &CreateUserDto) -> AppResult<()> {
    if dto.username.trim().len() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if !dto.email.contains('@') {
        return Err(AppError::BadRequest("邮箱格式不正确".into()));
    }
    if dto.password.len() < 8 {
        return Err(AppError::BadRequest("密码至少 8 个字符".into()));
    }
    Ok(())
}

async fn fetch_user(state: &AppState, id: &str) -> AppResult<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

/// 签发 JWT，设置 auth_token Cookie，并返回 token + expires_at + user。
fn issue_auth_response(
    state: &AppState,
    user: User,
    status: StatusCode,
) -> AppResult<(StatusCode, HeaderMap, Json<Value>)> {
    let (token, expires_at) = jwt::issue(
        &state.config.jwt_secret,
        &user.id,
        state.config.jwt_expires_in_hours,
    )?;

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, build_auth_cookie(state, &token).parse().unwrap());

    let body = json!({
        "data": {
            "token": token,
            "expires_at": expires_at,
            "user": UserResponse::from(user),
        }
    });
    Ok((status, headers, Json(body)))
}

/// 构造 auth_token Set-Cookie 值。HTTPS 环境附加 Secure。
fn build_auth_cookie(state: &AppState, token: &str) -> String {
    let max_age = state.config.jwt_expires_in_hours * 3600;
    let secure = if state.config.public_base_url.starts_with("https") {
        "; Secure"
    } else {
        ""
    };
    format!("auth_token={token}; HttpOnly; SameSite=Lax; Path=/; Max-Age={max_age}{secure}")
}
