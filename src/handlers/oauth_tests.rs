//! 第三方登录端点的 HTTP 层测试（provider 用 `FakeProvider`，不碰网络）。

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, header};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use tower::ServiceExt;

use crate::auth::oauth::{ExternalProfile, FakeProvider, OAuthRegistry};
use crate::config::{
    ChatConfig, ChatLanguage, ChatProviderKind, Config, RegistrationMode, RegistrationPolicy,
};
use crate::state::AppState;
use crate::update::settings::{Channel, Mode};

fn profile(subject: &str, email: &str) -> ExternalProfile {
    ExternalProfile {
        subject: subject.into(),
        email: Some(email.into()),
        email_verified: true,
        name: Some("Dad".into()),
    }
}

/// `enabled` 控制总开关，`mode` 控制注册模式；FakeProvider 认三个预置授权码。
async fn app(enabled: bool, mode: RegistrationMode) -> Router {
    let root = std::env::temp_dir().join(format!("ch-oauth-http-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let chat = ChatConfig {
        provider: ChatProviderKind::None,
        bot_token: None,
        language: ChatLanguage::En,
        mask_identifiers: true,
    };
    let config = Arc::new(Config {
        server_host: "127.0.0.1".into(),
        server_port: 0,
        volumes_watch_path: PathBuf::from("/nonexistent"),
        drive_data_dir: "d".into(),
        jwt_secret: "s".into(),
        jwt_expires_in_hours: 1,
        max_upload_size_mb: 1,
        trash_retention_days: 7,
        trash_cleanup_interval_hours: 1,
        cors_allowed_origins: vec![],
        public_base_url: "http://localhost".into(),
        log_dir: root.clone(),
        log_format: "pretty".into(),
        log_file_enabled: false,
        log_max_size_mb: 1,
        trust_proxy_headers: false,
        admin_password: None,
        oauth_enabled: enabled,
        update_mode: Mode::Manual,
        update_window: "03:00-05:00".into(),
        update_channel: Channel::Stable,
        update_check_interval_hours: 6,
        update_root: root,
        update_base_url: None,
        registration: RegistrationPolicy {
            mode,
            rate_per_ip_per_hour: 100,
            pending_max: 100,
            pending_expire_days: 14,
        },
        chat,
    });
    let db = crate::db::test_pool().await;
    let drive_manager = crate::drive::DriveManager::new("d");
    let activity = crate::lifecycle::ActivityTracker::new();
    let shutdown = CancellationToken::new();
    let events = crate::events::ChangeNotifier::new();
    let updater = crate::update::Updater::new(
        config.clone(),
        db.clone(),
        drive_manager.clone(),
        activity.clone(),
        shutdown.clone(),
        events.clone(),
    )
    .unwrap();
    let chat = crate::chat::ChatService::from_config(
        db.clone(),
        &config.chat,
        crate::chat::AdminEnv {
            drive_manager: drive_manager.clone(),
            events: events.clone(),
            trash_retention_days: config.trash_retention_days,
        },
    )
    .unwrap();
    let fake = FakeProvider::new("google")
        .code("alice", profile("g-alice", "alice@gmail.com"))
        .code("bob", profile("g-bob", "bob@gmail.com"));
    let state = AppState {
        db,
        config,
        drive_manager,
        drive_id: "d".into(),
        events,
        activity,
        shutdown,
        oauth: OAuthRegistry::new(enabled).with(fake),
        updater,
        chat,
    };
    crate::handlers::build_router(state)
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    bearer: Option<&str>,
) -> (u16, Value, Option<String>) {
    let mut req = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"));
    if let Some(t) = bearer {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let body = match body {
        Some(v) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let mut req = req.body(body).unwrap();
    // 公网来源：不触发「本机首个账户」引导规则。
    let peer: SocketAddr = "8.8.8.8:50000".parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        cookie,
    )
}

fn code_body(code: &str) -> Option<Value> {
    Some(json!({ "code": code, "redirect_uri": "http://localhost/cb" }))
}

#[tokio::test]
async fn providers_reports_switch_and_list() {
    let on = app(true, RegistrationMode::Open).await;
    let (s, b, _) = call(&on, "GET", "/auth/providers", None, None).await;
    assert_eq!(s, 200);
    assert_eq!(
        b["data"],
        json!({ "enabled": true, "providers": ["google"] })
    );

    let off = app(false, RegistrationMode::Open).await;
    let (s, b, _) = call(&off, "GET", "/auth/providers", None, None).await;
    assert_eq!(s, 200);
    assert_eq!(b["data"], json!({ "enabled": false, "providers": [] }));
}

#[tokio::test]
async fn disabled_switch_makes_every_oauth_endpoint_404() {
    let off = app(false, RegistrationMode::Open).await;
    let (s, b, _) = call(
        &off,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!((s, b["error"]["code"].as_str()), (404, Some("NOT_FOUND")));
    let (s, _, _) = call(
        &off,
        "GET",
        "/auth/oauth/google/authorize?redirect_uri=http://x&state=s",
        None,
        None,
    )
    .await;
    assert_eq!(s, 404);
}

#[tokio::test]
async fn unconfigured_provider_is_404_and_bad_code_is_401() {
    let on = app(true, RegistrationMode::Open).await;
    let (s, ..) = call(
        &on,
        "POST",
        "/auth/oauth/apple/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(s, 404);
    let (s, ..) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("nope"),
        None,
    )
    .await;
    assert_eq!(s, 401);
}

#[tokio::test]
async fn authorize_returns_provider_url() {
    let on = app(true, RegistrationMode::Open).await;
    let (s, b, _) = call(
        &on,
        "GET",
        "/auth/oauth/google/authorize?redirect_uri=http://x&state=abc",
        None,
        None,
    )
    .await;
    assert_eq!(s, 200);
    assert!(b["data"]["url"].as_str().unwrap().contains("state=abc"));
}

#[tokio::test]
async fn open_mode_login_issues_token_and_repeat_is_same_user() {
    let on = app(true, RegistrationMode::Open).await;
    let (s, b, cookie) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(s, 200);
    assert!(cookie.unwrap().starts_with("auth_token="));
    let token = b["data"]["token"].as_str().unwrap().to_string();
    let id = b["data"]["user"]["id"].clone();
    assert_eq!(b["data"]["user"]["has_password"], false);

    let (s, b, _) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!((s, &b["data"]["user"]["id"]), (200, &id));

    let (s, b, _) = call(&on, "GET", "/auth/me", None, Some(&token)).await;
    assert_eq!((s, &b["data"]["id"]), (200, &id));
}

#[tokio::test]
async fn approval_mode_returns_202_without_token_then_403_until_approved() {
    let on = app(true, RegistrationMode::Approval).await;
    let (s, b, cookie) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(s, 202);
    assert_eq!(b["data"]["status"], "pending_approval");
    assert!(b["data"].get("token").is_none());
    assert!(cookie.is_none());

    let (s, b, cookie) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (403, Some("ACCOUNT_PENDING"))
    );
    assert!(cookie.is_none());
}

#[tokio::test]
async fn closed_mode_refuses_new_oauth_accounts() {
    let on = app(true, RegistrationMode::Closed).await;
    let (s, b, _) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(
        (s, b["error"]["code"].as_str()),
        (403, Some("REGISTRATION_CLOSED"))
    );
}

#[tokio::test]
async fn link_requires_auth_and_cannot_steal_another_accounts_identity() {
    let on = app(true, RegistrationMode::Open).await;
    let (s, ..) = call(
        &on,
        "POST",
        "/auth/oauth/google/link",
        code_body("alice"),
        None,
    )
    .await;
    assert_eq!(s, 401);

    // alice 用 Google 登录；bob 先有一个 Google 账号登录。
    let (_, a, _) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("alice"),
        None,
    )
    .await;
    let (_, b, _) = call(
        &on,
        "POST",
        "/auth/oauth/google/login",
        code_body("bob"),
        None,
    )
    .await;
    let a_token = a["data"]["token"].as_str().unwrap();
    let b_token = b["data"]["token"].as_str().unwrap();

    // bob 试图把 alice 的 Google 账号绑到自己名下：专用 409。
    let (s, e, _) = call(
        &on,
        "POST",
        "/auth/oauth/google/link",
        code_body("alice"),
        Some(b_token),
    )
    .await;
    assert_eq!(
        (s, e["error"]["code"].as_str()),
        (409, Some("OAUTH_ACCOUNT_ALREADY_LINKED"))
    );
    // bob 已绑了另一个 Google 账号，再绑 alice 之外的会得到另一个码；这里对自己幂等。
    let (s, ..) = call(
        &on,
        "POST",
        "/auth/oauth/google/link",
        code_body("bob"),
        Some(b_token),
    )
    .await;
    assert_eq!(s, 200);
    // alice 的账户没变。
    let (_, me, _) = call(&on, "GET", "/auth/me", None, Some(a_token)).await;
    assert_eq!(me["data"]["id"], a["data"]["user"]["id"]);
}
