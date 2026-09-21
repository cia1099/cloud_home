//! 个人资料端点：显示名、密码、邮箱 / 手机号的增改删与验证。
//!
//! 标识变更的规则（两个因素，缺一不可）：
//! 1. 当前密码；
//! 2. 发往「现有标识」的验证码。
//!
//! 第 2 条才是关键：只验密码的话，密码一旦泄露，攻击者就能把邮箱换成自己的，
//! 从而永久接管账户。验证旧通道能证明请求者确实持有这个账户。同理也堵住了
//! 迂回路径——否则可以先凭密码删掉邮箱、再添加自己的，两步达成同样的接管。
//!
//! 「证明新值」则与「设置新值」解耦：新标识一律以未验证状态落库，
//! 用户随时可通过下方的验证端点把它标记为已验证。

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::auth::{AuthUser, password};
use crate::error::{AppError, AppResult, ErrorResponse};
use crate::models::common::OkResponse;
use crate::models::user::{
    ChallengeDto, ChangeIdentifierDto, ChangePasswordDto, RemoveIdentifierDto, UpdateProfileDto,
    UserResponse, VerifyIdentifierDto, VerifyRequestDto,
};
use crate::openapi::ApiResponse;
use crate::services::{otp_service, user_service};
use crate::state::AppState;

/// PATCH /me — 修改显示名。
///
/// 显示名不是登录标识，因此不需要二次验证。
#[utoipa::path(
    patch,
    path = "/me",
    tag = "me",
    request_body = UpdateProfileDto,
    responses(
        (status = 200, description = "已更新", body = ApiResponse<UserResponse>),
        (status = 400, description = "用户名不合法", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<UpdateProfileDto>,
) -> AppResult<ApiResponse<UserResponse>> {
    let name = dto.username.trim();
    if name.chars().count() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if name.chars().count() > 64 {
        return Err(AppError::BadRequest("用户名最多 64 个字符".into()));
    }

    let updated = user_service::update_username(&state.db, &user.user_id, name).await?;
    crate::audit!(
        info,
        "user.profile_update",
        "success",
        user_id = %updated.id,
        new_username = %updated.username,
        "修改显示名"
    );

    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&updated, &ids)))
}

/// POST /me/password — 修改密码。
#[utoipa::path(
    post,
    path = "/me/password",
    tag = "me",
    request_body = ChangePasswordDto,
    responses(
        (status = 200, description = "密码已修改", body = ApiResponse<OkResponse>),
        (status = 400, description = "新密码不合法", body = ErrorResponse),
        (status = 401, description = "当前密码错误", body = ErrorResponse),
        (status = 403, description = "该账户由第三方登录托管，无需密码", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<ChangePasswordDto>,
) -> AppResult<ApiResponse<OkResponse>> {
    if dto.new_password.len() < 8 {
        return Err(AppError::BadRequest("密码至少 8 个字符".into()));
    }
    assert_password(&state, &user.user_id, &dto.current_password).await?;

    user_service::change_password(&state.db, &user.user_id, &dto.new_password).await?;
    crate::audit!(
        info,
        "user.password_change",
        "success",
        user_id = %user.user_id,
        "修改密码"
    );
    Ok(ApiResponse::new(OkResponse { ok: true }))
}

/// POST /me/identifier/challenge — 向「现有标识」发送验证码，用于随后的增改删。
///
/// 新增某个 provider 时本账户还没有同类旧值，此时验证码发往已有的另一条标识。
#[utoipa::path(
    post,
    path = "/me/identifier/challenge",
    tag = "me",
    request_body = ChallengeDto,
    responses(
        (status = 204, description = "验证码已发送"),
        (status = 400, description = "provider 不合法或请求过于频繁", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 403, description = "标识由第三方托管，不可在此修改", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn challenge(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<ChallengeDto>,
) -> AppResult<StatusCode> {
    check_provider(&dto.provider)?;
    let target = challenge_target(&state, &user.user_id, &dto.provider).await?;

    otp_service::issue(
        &state.db,
        Some(&user.user_id),
        otp_service::PURPOSE_IDENTIFIER_OLD,
        &target,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /me/identifier — 新增或修改邮箱 / 手机号。
#[utoipa::path(
    put,
    path = "/me/identifier",
    tag = "me",
    request_body = ChangeIdentifierDto,
    responses(
        (status = 200, description = "已更新；新值为未验证状态", body = ApiResponse<UserResponse>),
        (status = 400, description = "参数不合法或验证码错误", body = ErrorResponse),
        (status = 401, description = "当前密码错误", body = ErrorResponse),
        (status = 403, description = "标识由第三方托管，不可在此修改", body = ErrorResponse),
        (status = 409, description = "该邮箱或手机号已被使用", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn change_identifier(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<ChangeIdentifierDto>,
) -> AppResult<ApiResponse<UserResponse>> {
    check_provider(&dto.provider)?;
    let new_value = user_service::normalize_for(&dto.provider, &dto.new_value)?;

    assert_password(&state, &user.user_id, &dto.current_password).await?;
    let target = challenge_target(&state, &user.user_id, &dto.provider).await?;
    otp_service::verify(
        &state.db,
        otp_service::PURPOSE_IDENTIFIER_OLD,
        &target,
        &dto.old_code,
    )
    .await?;

    user_service::set_identifier(&state.db, &user.user_id, &dto.provider, &new_value).await?;
    crate::audit!(
        info,
        "user.identifier_change",
        "success",
        user_id = %user.user_id,
        provider = %dto.provider,
        new_value = %otp_service::mask(&new_value),
        "修改登录标识"
    );

    let u = user_service::get(&state.db, &user.user_id).await?;
    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&u, &ids)))
}

/// DELETE /me/identifier — 删除邮箱 / 手机号（账户须至少保留一条）。
#[utoipa::path(
    delete,
    path = "/me/identifier",
    tag = "me",
    request_body = RemoveIdentifierDto,
    responses(
        (status = 200, description = "已删除", body = ApiResponse<UserResponse>),
        (status = 400, description = "验证码错误，或这是最后一个标识", body = ErrorResponse),
        (status = 401, description = "当前密码错误", body = ErrorResponse),
        (status = 403, description = "标识由第三方托管，不可在此修改", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn remove_identifier(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<RemoveIdentifierDto>,
) -> AppResult<ApiResponse<UserResponse>> {
    check_provider(&dto.provider)?;
    assert_password(&state, &user.user_id, &dto.current_password).await?;

    let identity = user_service::user_owned_identity(&state.db, &user.user_id, &dto.provider).await?;
    otp_service::verify(
        &state.db,
        otp_service::PURPOSE_IDENTIFIER_OLD,
        &identity.identifier,
        &dto.old_code,
    )
    .await?;

    user_service::remove_identifier(&state.db, &user.user_id, &dto.provider).await?;
    crate::audit!(
        info,
        "user.identifier_change",
        "success",
        user_id = %user.user_id,
        provider = %dto.provider,
        action = "remove",
        "删除登录标识"
    );

    let u = user_service::get(&state.db, &user.user_id).await?;
    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&u, &ids)))
}

/// POST /me/identifier/verify/request — 向自己已登记的标识发送验证码。
#[utoipa::path(
    post,
    path = "/me/identifier/verify/request",
    tag = "me",
    request_body = VerifyRequestDto,
    responses(
        (status = 204, description = "验证码已发送"),
        (status = 400, description = "provider 不合法或请求过于频繁", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 403, description = "标识由第三方托管", body = ErrorResponse),
        (status = 409, description = "该标识已验证过", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn verify_request(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<VerifyRequestDto>,
) -> AppResult<StatusCode> {
    check_provider(&dto.provider)?;
    let identity = user_service::user_owned_identity(&state.db, &user.user_id, &dto.provider).await?;
    if identity.verified_at.is_some() {
        return Err(AppError::Conflict("该标识已完成验证".into()));
    }

    otp_service::issue(
        &state.db,
        Some(&user.user_id),
        otp_service::PURPOSE_IDENTIFIER_VERIFY,
        &identity.identifier,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /me/identifier/verify — 提交验证码，把标识标记为已验证。
#[utoipa::path(
    post,
    path = "/me/identifier/verify",
    tag = "me",
    request_body = VerifyIdentifierDto,
    responses(
        (status = 200, description = "已验证", body = ApiResponse<UserResponse>),
        (status = 400, description = "验证码错误或已过期", body = ErrorResponse),
        (status = 401, description = "认证失败", body = ErrorResponse),
        (status = 403, description = "标识由第三方托管", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn verify_identifier(
    State(state): State<AppState>,
    user: AuthUser,
    Json(dto): Json<VerifyIdentifierDto>,
) -> AppResult<ApiResponse<UserResponse>> {
    check_provider(&dto.provider)?;
    let identity = user_service::user_owned_identity(&state.db, &user.user_id, &dto.provider).await?;

    otp_service::verify(
        &state.db,
        otp_service::PURPOSE_IDENTIFIER_VERIFY,
        &identity.identifier,
        &dto.code,
    )
    .await?;

    user_service::mark_verified(&state.db, &user.user_id, &dto.provider).await?;
    crate::audit!(
        info,
        "user.identifier_verified",
        "success",
        user_id = %user.user_id,
        provider = %dto.provider,
        identifier = %otp_service::mask(&identity.identifier),
        "标识验证通过"
    );

    let u = user_service::get(&state.db, &user.user_id).await?;
    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&u, &ids)))
}

// ---- 辅助 ----

fn check_provider(provider: &str) -> AppResult<()> {
    if crate::models::user::USER_OWNED_PROVIDERS.contains(&provider) {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "provider 只能是 email 或 phone".into(),
        ))
    }
}

/// 校验当前密码。纯第三方登录账户没有密码可验，直接 403——
/// 它们的标识本就由 Google/Apple 托管，不该走到这里。
async fn assert_password(state: &AppState, user_id: &str, supplied: &str) -> AppResult<()> {
    let Some(credential) = user_service::password_credential(&state.db, user_id).await? else {
        return Err(AppError::ForbiddenReason(
            "该账户由第三方登录托管，未设置密码".into(),
        ));
    };
    if !password::verify(supplied, credential.secret.as_deref().unwrap_or("")) {
        crate::audit!(
            warn,
            "user.reauth",
            "failure",
            reason = "bad_password",
            user_id = %user_id,
            "个人资料变更：当前密码错误"
        );
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

/// 决定「旧通道」验证码该发往哪里：优先发往正在被改 / 删的同类标识；
/// 若该 provider 尚不存在（即新增），则退回到账户已有的另一条自有标识。
async fn challenge_target(
    state: &AppState,
    user_id: &str,
    provider: &str,
) -> AppResult<String> {
    match user_service::user_owned_identity(&state.db, user_id, provider).await {
        Ok(identity) => Ok(identity.identifier),
        Err(AppError::NotFound) => Ok(user_service::any_user_owned_identity(&state.db, user_id)
            .await?
            .identifier),
        Err(e) => Err(e),
    }
}
