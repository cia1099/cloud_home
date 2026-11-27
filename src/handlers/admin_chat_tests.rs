//! `/admin/chat/*` 端点与「注册触发通知」的 HTTP 层测试（聊天提供方用 `FakeProvider`，不碰网络）。

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

use crate::chat::ChatService;
use crate::chat::provider::{ChatProvider, FakeProvider};
use crate::config::{
    ChatConfig, ChatLanguage, ChatProviderKind, Config, RegistrationMode, RegistrationPolicy,
};
use crate::db::Db;
use crate::state::AppState;
use crate::update::settings::{Channel, Mode};

const PW: &str = "test-admin-pw";

struct Harness {
    app: Router,
    db: Db,
}

async fn harness(kind: ChatProviderKind) -> Harness {
    let root = std::env::temp_dir().join(format!("ch-admin-chat-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let chat_cfg = ChatConfig {
        provider: kind,
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
        admin_password: Some(PW.into()),
        update_mode: Mode::Manual,
        update_window: "03:00-05:00".into(),
        update_channel: Channel::Stable,
        update_check_interval_hours: 6,
        update_root: root,
        update_base_url: None,
        registration: RegistrationPolicy {
            mode: RegistrationMode::Approval,
            rate_per_ip_per_hour: 100,
            pending_max: 100,
            pending_expire_days: 14,
        },
        chat: chat_cfg.clone(),
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
    let provider: Option<Arc<dyn ChatProvider>> = (kind != ChatProviderKind::None)
        .then(|| Arc::new(FakeProvider::default()) as Arc<dyn ChatProvider>);
    let state = AppState {
        db: db.clone(),
        config,
        drive_manager,
        drive_id: "d".into(),
        events,
        activity,
        shutdown,
        updater,
        chat: ChatService::new(db.clone(), &chat_cfg, provider),
    };
    Harness {
        app: crate::handlers::build_router(state),
        db,
    }
}

async fn call(
    h: &Harness,
    method: &str,
    path: &str,
    body: Option<Value>,
    password: Option<&str>,
    peer: &str,
) -> (u16, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(format!("/api/v1{path}"));
    if let Some(p) = password {
        req = req.header("x-admin-password", p);
    }
    let body = match body {
        Some(v) => {
            req = req.header(header::CONTENT_TYPE, "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let mut req = req.body(body).unwrap();
    let peer: SocketAddr = format!("{peer}:50000").parse().unwrap();
    req.extensions_mut().insert(ConnectInfo(peer));
    let resp = h.app.clone().oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn admin(h: &Harness, method: &str, path: &str) -> (u16, Value) {
    call(h, method, path, None, Some(PW), "127.0.0.1").await
}

async fn insert_admin(db: &Db) {
    sqlx::query(
        "INSERT INTO admin_chat_identities (id, provider, sender_id, chat_id, label, paired_at) \
         VALUES ('a1', 'telegram', '42', '42', 'Ann', '2026-01-01T00:00:00+00:00')",
    )
    .execute(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn chat_endpoints_are_guarded() {
    let h = harness(ChatProviderKind::Telegram).await;
    for (m, p) in [
        ("POST", "/admin/chat/pairing"),
        ("GET", "/admin/chat"),
        ("DELETE", "/admin/chat/admins/x"),
    ] {
        assert_eq!(call(&h, m, p, None, None, "127.0.0.1").await.0, 401);
        assert_eq!(
            call(&h, m, p, None, Some("wrong"), "127.0.0.1").await.0,
            401
        );
        assert_eq!(call(&h, m, p, None, Some(PW), "8.8.8.8").await.0, 403);
    }
}

#[tokio::test]
async fn pairing_status_and_unpair_over_http() {
    let h = harness(ChatProviderKind::Telegram).await;

    let (s, body) = admin(&h, "POST", "/admin/chat/pairing").await;
    assert_eq!(s, 200);
    assert_eq!(body["data"]["code"].as_str().unwrap().len(), 8);
    assert!(body["data"]["expires_at"].is_string());

    let (s, body) = admin(&h, "GET", "/admin/chat").await;
    assert_eq!(s, 200);
    assert_eq!(body["data"]["provider"], "telegram");
    assert_eq!(body["data"]["admins"], json!([]));

    insert_admin(&h.db).await;
    let (_, body) = admin(&h, "GET", "/admin/chat").await;
    assert_eq!(body["data"]["admins"][0]["label"], "Ann");
    // 响应里不暴露提供方的 sender id。
    assert!(body["data"]["admins"][0].get("sender_id").is_none());

    assert_eq!(admin(&h, "DELETE", "/admin/chat/admins/nope").await.0, 404);
    let (s, body) = admin(&h, "DELETE", "/admin/chat/admins/a1").await;
    assert_eq!((s, body["data"]["label"].as_str()), (200, Some("Ann")));
    let (_, body) = admin(&h, "GET", "/admin/chat").await;
    assert_eq!(body["data"]["admins"], json!([]));
}

#[tokio::test]
async fn pairing_is_refused_when_chat_is_disabled() {
    let h = harness(ChatProviderKind::None).await;
    assert_eq!(admin(&h, "POST", "/admin/chat/pairing").await.0, 409);
    let (s, body) = admin(&h, "GET", "/admin/chat").await;
    assert_eq!(s, 200);
    assert_eq!(body["data"]["provider"], "none");
    assert_eq!(body["data"]["connected"], false);
}

#[tokio::test]
async fn registration_queues_a_masked_notification_for_paired_admins() {
    let h = harness(ChatProviderKind::Telegram).await;
    insert_admin(&h.db).await;

    // 首个账户按引导规则直接批准，不通知；第二个才是 pending。
    for (name, email, want) in [
        ("first", "first@x.org", 201),
        ("second", "second@x.org", 202),
    ] {
        let (s, _) = call(
            &h,
            "POST",
            "/auth/register",
            Some(json!({"username": name, "password": "password123", "email": email})),
            None,
            "127.0.0.1",
        )
        .await;
        assert_eq!(s, want);
    }
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT state, text FROM chat_outbox WHERE kind = 'notify'")
            .fetch_all(&h.db)
            .await
            .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "queued");
    assert!(
        rows[0].1.contains("s***@x.org") && !rows[0].1.contains("second@"),
        "{}",
        rows[0].1
    );
}
