//! Router 组装与中间件注册。

pub mod auth;
pub mod drives;
pub mod files;
pub mod folders;
pub mod public;
pub mod search;
pub mod shares;
pub mod trash;

use axum::Router;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::openapi::{ApiDoc, docs_router};
use crate::state::AppState;

/// 构造完整的应用 Router（含 `/api/v1` 前缀、`/docs` 文档与中间件）。
pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state);
    let body_limit = RequestBodyLimitLayer::new(state.config.max_upload_size_bytes());

    let api: OpenApiRouter<AppState> = OpenApiRouter::new()
        .routes(routes!(health))
        // 认证
        .routes(routes!(auth::register))
        .routes(routes!(auth::login))
        .routes(routes!(auth::logout))
        .routes(routes!(auth::me))
        // 硬盘
        .routes(routes!(drives::list))
        // 文件
        .routes(routes!(files::list))
        .routes(routes!(files::upload))
        .routes(routes!(files::get_metadata, files::rename, files::delete))
        .routes(routes!(files::download))
        .routes(routes!(files::move_file))
        // 资料夹
        .routes(routes!(folders::create))
        // 搜索
        .routes(routes!(search::search))
        // 回收站
        .routes(routes!(trash::list, trash::empty))
        .routes(routes!(trash::restore))
        .routes(routes!(trash::delete_one))
        // 分享
        .routes(routes!(shares::create, shares::list))
        .routes(routes!(shares::revoke))
        // 公开访问（无 auth）
        .routes(routes!(public::info))
        .routes(routes!(public::download));

    let (api_router, openapi) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", api)
        .split_for_parts();

    api_router
        .merge(docs_router(openapi))
        .layer(body_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[utoipa::path(get, path = "/health", tag = "health", responses(
    (status = 200, description = "服务健康", body = String)
))]
async fn health() -> &'static str {
    "ok"
}

fn build_cors(state: &AppState) -> CorsLayer {
    let origins: Vec<HeaderValue> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
}
