//! 认证端点：注册、登录、登出、获取当前用户。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header::SET_COOKIE};

use crate::auth::{AuthUser, jwt, password};
use crate::error::{AppError, AppResult, ErrorResponse};
use crate::models::common::OkResponse;
use crate::models::user::{AuthResponse, CreateUserDto, LoginDto, User, UserResponse};
use crate::openapi::ApiResponse;
use crate::services::user_service;
use crate::state::AppState;

/// POST /auth/register
#[utoipa::path(
    post,
    path = "/auth/register",
    tag = "auth",
    request_body = CreateUserDto,
    responses(
        (status = 201, description = "注册成功", body = ApiResponse<AuthResponse>),
        (status = 400, description = "参数校验失败，或未提供邮箱与手机号", body = ErrorResponse),
        (status = 409, description = "邮箱或手机号已被注册", body = ErrorResponse),
    )
)]
pub async fn register(
    State(state): State<AppState>,
    Json(dto): Json<CreateUserDto>,
) -> AppResult<(StatusCode, HeaderMap, ApiResponse<AuthResponse>)> {
    let username = dto.username.trim();
    if username.chars().count() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if dto.password.len() < 8 {
        return Err(AppError::BadRequest("密码至少 8 个字符".into()));
    }

    // 邮箱与手机号至少给一个——否则账户没有任何可登录的标识。
    let email = dto
        .email
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(user_service::normalize_email)
        .transpose()?;
    let phone = dto
        .phone
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(user_service::normalize_phone)
        .transpose()?;
    if email.is_none() && phone.is_none() {
        return Err(AppError::BadRequest("请至少提供邮箱或手机号".into()));
    }

    // 用户名不再要求唯一；重复的邮箱/手机号由唯一索引在 service 层转成 409。
    let user = user_service::create(
        &state.db,
        username,
        &dto.password,
        email.as_deref(),
        phone.as_deref(),
    )
    .await
    .inspect_err(|e| {
        if matches!(e, AppError::Conflict(_)) {
            crate::audit!(
                warn,
                "auth.register",
                "failure",
                reason = "duplicate_identifier",
                username = %username,
                "注册失败：邮箱或手机号已被使用"
            );
        }
    })?;

    crate::audit!(
        info,
        "auth.register",
        "success",
        user_id = %user.id,
        username = %user.username,
        "注册成功"
    );
    issue_auth_response(&state, user, StatusCode::CREATED).await
}

/// POST /auth/login
#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    request_body = LoginDto,
    responses(
        (status = 200, description = "登录成功", body = ApiResponse<AuthResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    Json(dto): Json<LoginDto>,
) -> AppResult<(StatusCode, HeaderMap, ApiResponse<AuthResponse>)> {
    // identifier 可以是邮箱或手机号；两者都规范化后再查，避免大小写/空白错配。
    let identifier = normalize_login_identifier(&dto.identifier);

    let Some(user) = user_service::find_for_login(&state.db, &identifier).await? else {
        crate::audit!(
            warn,
            "auth.login",
            "failure",
            reason = "unknown_user",
            identifier = %crate::services::otp_service::mask(&identifier),
            "登录失败：用户不存在"
        );
        return Err(AppError::Unauthorized);
    };

    let Some(credential) = user_service::password_credential(&state.db, &user.id).await? else {
        crate::audit!(
            warn,
            "auth.login",
            "failure",
            reason = "no_password_credential",
            user_id = %user.id,
            "登录失败：该账户未设置密码（仅限第三方登录）"
        );
        return Err(AppError::Unauthorized);
    };

    if !password::verify(&dto.password, credential.secret.as_deref().unwrap_or("")) {
        crate::audit!(
            warn,
            "auth.login",
            "failure",
            reason = "bad_password",
            user_id = %user.id,
            username = %user.username,
            "登录失败：密码错误"
        );
        return Err(AppError::Unauthorized);
    }

    crate::audit!(
        info,
        "auth.login",
        "success",
        user_id = %user.id,
        username = %user.username,
        "登录成功"
    );
    issue_auth_response(&state, user, StatusCode::OK).await
}

/// 登录标识既可能是邮箱也可能是手机号，按形态分别规范化；
/// 规范化失败就原样返回，交给后续查表落到「用户不存在」。
fn normalize_login_identifier(raw: &str) -> String {
    if raw.contains('@') {
        user_service::normalize_email(raw).unwrap_or_else(|_| raw.trim().to_lowercase())
    } else {
        user_service::normalize_phone(raw).unwrap_or_else(|_| raw.trim().to_string())
    }
}

/// POST /auth/logout
#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    responses(
        (status = 200, description = "已登出", body = ApiResponse<OkResponse>,
         headers(("Set-Cookie" = String, description = "清除 auth_token cookie"))),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn logout(user: AuthUser) -> AppResult<(HeaderMap, ApiResponse<OkResponse>)> {
    crate::audit!(info, "auth.logout", "success", user_id = %user.user_id, "登出");
    let mut headers = HeaderMap::new();
    let clear = "auth_token=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0";
    headers.insert(SET_COOKIE, clear.parse().unwrap());
    Ok((headers, ApiResponse::new(OkResponse { ok: true })))
}

/// GET /auth/me
#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    responses(
        (status = 200, description = "当前用户信息", body = ApiResponse<UserResponse>),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 404, description = "用户不存在", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<ApiResponse<UserResponse>> {
    let u = user_service::get(&state.db, &user.user_id).await?;
    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&u, &ids)))
}

// ---- 辅助 ----

/// 签发 JWT，设置 auth_token Cookie，并返回 token + expires_at + user。
pub(crate) async fn issue_auth_response(
    state: &AppState,
    user: User,
    status: StatusCode,
) -> AppResult<(StatusCode, HeaderMap, ApiResponse<AuthResponse>)> {
    let (token, expires_at) = jwt::issue(
        &state.config.jwt_secret,
        &user.id,
        state.config.jwt_expires_in_hours,
    )?;

    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, build_auth_cookie(state, &token).parse().unwrap());

    let ids = user_service::identities(&state.db, &user.id).await?;
    let body = AuthResponse {
        token,
        expires_at,
        user: UserResponse::build(&user, &ids),
    };
    Ok((status, headers, ApiResponse::new(body)))
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
