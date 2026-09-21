//! Router 组装与中间件注册。

pub mod admin;
pub mod auth;
pub mod drives;
pub mod events;
pub mod files;
pub mod folders;
pub mod me;
pub mod public;
pub mod search;
pub mod shares;
pub mod trash;
pub mod usage;

use axum::Router;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::logging;
use crate::openapi::{ApiDoc, docs_router};
use crate::state::AppState;

/// 构造完整的应用 Router（含 `/api/v1` 前缀、`/docs` 文档与中间件）。
pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state);
    let body_limit = RequestBodyLimitLayer::new(state.config.max_upload_size_bytes());
    let trust_proxy = state.config.trust_proxy_headers;

    let api: OpenApiRouter<AppState> = OpenApiRouter::new()
        .routes(routes!(health))
        // 认证
        .routes(routes!(auth::register))
        .routes(routes!(auth::login))
        .routes(routes!(auth::logout))
        .routes(routes!(auth::me))
        // 个人资料
        .routes(routes!(me::update_profile))
        .routes(routes!(me::change_password))
        .routes(routes!(me::challenge))
        .routes(routes!(me::change_identifier, me::remove_identifier))
        .routes(routes!(me::verify_request))
        .routes(routes!(me::verify_identifier))
        // 运维（共享口令 + 内网限制保护）
        .routes(routes!(admin::list_users))
        .routes(routes!(admin::delete_user))
        .routes(routes!(admin::restore_user))
        .routes(routes!(admin::purge_trash))
        // 硬盘
        .routes(routes!(drives::list))
        // 文件
        .routes(routes!(files::list))
        .routes(routes!(files::upload))
        .routes(routes!(files::download_archive))
        .routes(routes!(files::get_metadata, files::rename, files::delete))
        .routes(routes!(files::download))
        .routes(routes!(files::raw))
        .routes(routes!(files::thumbnail))
        .routes(routes!(files::move_file))
        // 资料夹
        .routes(routes!(folders::create))
        // 空间占用
        .routes(routes!(usage::current))
        .routes(routes!(usage::breakdown))
        // 实时事件（SSE）
        .routes(routes!(events::stream))
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

    // 中间件栈（自外向内）：
    //   请求 id 生成 → 传播到响应 → 捕获 panic（转 500，服务不中断）
    //   → 访问日志 span（含 request_id/method/route/client_ip）
    //   → CORS → 请求体大小限制 → 路由
    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(CatchPanicLayer::new())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(move |req: &axum::http::Request<axum::body::Body>| {
                    logging::make_span(req, trust_proxy)
                })
                .on_request(())
                .on_response(logging::on_response)
                .on_failure(()),
        )
        .layer(cors)
        .layer(body_limit);

    api_router
        .merge(docs_router(openapi))
        .layer(middleware)
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
