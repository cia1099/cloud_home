//! OpenAPI 文档生成与 Scalar 交互式 UI 挂载。

use axum::Json;
use axum::Router;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde::Serialize;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::{Components, OpenApi};
use utoipa::{Modify, OpenApi as OpenApiDerive, ToSchema};
use utoipa_scalar::{Scalar, Servable};

use crate::state::AppState;

/// 统一响应信封：`{ "data": T }`。
#[derive(Debug, Serialize, ToSchema)]
pub struct ApiResponse<T> {
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self { data }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// OpenAPI 顶层文档定义。具体 `paths` 由 `handlers::build_router` 中 `routes!` 宏收集的
/// 路由信息合并而来，此处只负责安全方案等全局元数据。
#[derive(OpenApiDerive)]
#[openapi(modifiers(&SecurityAddon))]
pub struct ApiDoc;

/// 注册 Cookie / Bearer 两种认证方式，对应 `AuthUser` 提取器的双模式校验逻辑。
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::new);
        components.add_security_scheme(
            "cookie_auth",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("auth_token"))),
        );
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );
    }
}

/// 挂载 `/docs`（Scalar 交互式 UI）与 `/openapi.json`（原始 spec）。
pub fn docs_router(openapi: OpenApi) -> Router<AppState> {
    let spec_json = serde_json::to_string(&openapi).expect("OpenApi 序列化失败");

    let scalar: Router<AppState> = Scalar::with_url("/docs", openapi).into();

    scalar.route(
        "/openapi.json",
        get(move || {
            let body = spec_json.clone();
            async move { ([(CONTENT_TYPE, "application/json")], body) }
        }),
    )
}
