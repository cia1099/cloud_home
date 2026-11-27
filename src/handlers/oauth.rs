//! 第三方登录端点：provider 列表、授权地址、授权码登录、绑定。
//!
//! 总开关 `OAUTH_ENABLED` 关闭（默认）时，除 `GET /auth/providers`（报告空列表）外
//! 一律 404。真实 provider 尚未接入，所以目前生产环境列表恒为空。

use axum::Json;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::auth::AuthUser;
use crate::error::{AppResult, ErrorResponse};
use crate::handlers::auth::issue_auth_response;
use crate::models::user::{AuthResponse, UserResponse};
use crate::openapi::ApiResponse;
use crate::services::{oauth_service, user_service};
use crate::state::AppState;

/// `GET /auth/providers` 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct ProvidersResponse {
    /// 总开关是否开启。
    pub enabled: bool,
    /// 已配置、可用的 provider（`google` / `apple`）；关闭时为空，前端据此隐藏按钮。
    pub providers: Vec<String>,
}

/// `GET /auth/oauth/{provider}/authorize` 查询参数。
#[derive(Debug, Deserialize, IntoParams)]
pub struct AuthorizeQuery {
    /// 授权完成后回到前端的地址（须在 provider 侧登记过）。
    pub redirect_uri: String,
    /// 前端生成并在回调时核对的 CSRF 随机串，服务端原样带进授权地址。
    pub state: String,
}

/// `GET /auth/oauth/{provider}/authorize` 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthorizeResponse {
    /// 把浏览器导向此地址开始授权。
    pub url: String,
}

/// 授权码登录 / 绑定请求体。
#[derive(Debug, Deserialize, ToSchema)]
pub struct OAuthCodeDto {
    /// provider 回调带回的授权码。
    pub code: String,
    /// 与授权时相同的 redirect_uri。
    pub redirect_uri: String,
}

/// GET /auth/providers
#[utoipa::path(
    get,
    path = "/auth/providers",
    tag = "auth",
    responses((status = 200, description = "已配置的第三方登录 provider", body = ApiResponse<ProvidersResponse>))
)]
pub async fn providers(State(state): State<AppState>) -> ApiResponse<ProvidersResponse> {
    ApiResponse::new(ProvidersResponse {
        enabled: state.oauth.enabled(),
        providers: state.oauth.names().into_iter().map(String::from).collect(),
    })
}

/// GET /auth/oauth/{provider}/authorize
#[utoipa::path(
    get,
    path = "/auth/oauth/{provider}/authorize",
    tag = "auth",
    params(("provider" = String, Path, description = "google | apple"), AuthorizeQuery),
    responses(
        (status = 200, description = "授权地址", body = ApiResponse<AuthorizeResponse>),
        (status = 404, description = "第三方登录未启用，或该 provider 未配置", body = ErrorResponse),
    )
)]
pub async fn authorize(
    State(state): State<AppState>,
    AxumPath(provider): AxumPath<String>,
    Query(q): Query<AuthorizeQuery>,
) -> AppResult<ApiResponse<AuthorizeResponse>> {
    let p = state.oauth.get(&provider)?;
    let url = p.authorize_url(&q.redirect_uri, &q.state)?;
    Ok(ApiResponse::new(AuthorizeResponse { url }))
}

/// POST /auth/oauth/{provider}/login
#[utoipa::path(
    post,
    path = "/auth/oauth/{provider}/login",
    tag = "auth",
    params(("provider" = String, Path, description = "google | apple")),
    request_body = OAuthCodeDto,
    responses(
        (status = 200, description = "登录成功（新建或关联账户），设置 auth_token Cookie", body = ApiResponse<AuthResponse>),
        (status = 400, description = "第三方未提供已验证的邮箱", body = ErrorResponse),
        (status = 401, description = "授权码无效，或账户已停用", body = ErrorResponse),
        (status = 404, description = "第三方登录未启用，或该 provider 未配置", body = ErrorResponse),
        (status = 409, description = "邮箱已属于一个未验证邮箱的账户，无法并入", body = ErrorResponse),
    )
)]
pub async fn login(
    State(state): State<AppState>,
    AxumPath(provider): AxumPath<String>,
    Json(dto): Json<OAuthCodeDto>,
) -> AppResult<(StatusCode, HeaderMap, ApiResponse<AuthResponse>)> {
    let p = state.oauth.get(&provider)?;
    let profile = p
        .exchange(&dto.code, &dto.redirect_uri)
        .await
        .inspect_err(|_| {
            crate::audit!(
                warn,
                "auth.oauth_login",
                "failure",
                reason = "exchange_failed",
                provider = %provider,
                "第三方登录失败：授权码换取资料失败"
            );
        })?;
    let user = oauth_service::login(&state.db, p.name(), &profile).await?;
    crate::audit!(
        info,
        "auth.oauth_login",
        "success",
        provider = %provider,
        user_id = %user.id,
        username = %user.username,
        "第三方登录成功"
    );
    issue_auth_response(&state, user, StatusCode::OK).await
}

/// POST /auth/oauth/{provider}/link
#[utoipa::path(
    post,
    path = "/auth/oauth/{provider}/link",
    tag = "auth",
    params(("provider" = String, Path, description = "google | apple")),
    request_body = OAuthCodeDto,
    responses(
        (status = 200, description = "已绑定，返回最新的当前用户信息", body = ApiResponse<UserResponse>),
        (status = 401, description = "认证失败，或授权码无效", body = ErrorResponse),
        (status = 404, description = "第三方登录未启用，或该 provider 未配置", body = ErrorResponse),
        (status = 409, description = "该第三方账号已绑定到别的账户，或当前账户已绑了另一个", body = ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn link(
    State(state): State<AppState>,
    user: AuthUser,
    AxumPath(provider): AxumPath<String>,
    Json(dto): Json<OAuthCodeDto>,
) -> AppResult<ApiResponse<UserResponse>> {
    let p = state.oauth.get(&provider)?;
    let profile = p.exchange(&dto.code, &dto.redirect_uri).await?;
    oauth_service::link(&state.db, &user.user_id, p.name(), &profile).await?;
    crate::audit!(
        info,
        "auth.oauth_link",
        "success",
        provider = %provider,
        user_id = %user.user_id,
        "已绑定第三方账号"
    );
    let u = user_service::get(&state.db, &user.user_id).await?;
    let ids = user_service::identities(&state.db, &user.user_id).await?;
    Ok(ApiResponse::new(UserResponse::build(&u, &ids)))
}
