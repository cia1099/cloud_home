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
use axum::routing::{delete, get, post};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use crate::state::AppState;

/// 构造完整的应用 Router（含 `/api/v1` 前缀与中间件）。
pub fn build_router(state: AppState) -> Router {
    let cors = build_cors(&state);
    let body_limit = RequestBodyLimitLayer::new(state.config.max_upload_size_bytes());

    let api = Router::new()
        .route("/health", get(health))
        // 认证
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/auth/logout", post(auth::logout))
        .route("/auth/me", get(auth::me))
        // 硬盘
        .route("/drives", get(drives::list))
        // 文件
        .route("/files", get(files::list))
        .route("/files/upload", post(files::upload))
        .route(
            "/files/{id}",
            get(files::get_metadata)
                .patch(files::rename)
                .delete(files::delete),
        )
        .route("/files/{id}/download", get(files::download))
        .route("/files/{id}/move", post(files::move_file))
        // 资料夹
        .route("/folders", post(folders::create))
        // 搜索
        .route("/search", get(search::search))
        // 回收站
        .route("/trash", get(trash::list).delete(trash::empty))
        .route("/trash/{id}/restore", post(trash::restore))
        .route("/trash/{id}", delete(trash::delete_one))
        // 分享
        .route("/shares", post(shares::create).get(shares::list))
        .route("/shares/{id}", delete(shares::revoke))
        // 公开访问（无 auth）
        .route("/public/shares/{token}", get(public::info))
        .route("/public/shares/{token}/download", get(public::download));

    Router::new()
        .nest("/api/v1", api)
        .layer(body_limit)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

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
