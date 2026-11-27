//! 在线更新的运维端点：查看状态、立即检查、触发安装、修改更新设置。
//!
//! 与 `/admin/users*` 同一道 `AdminGuard`（共享口令 + 内网来源）。这些端点不碰用户数据，
//! 只读写进程级的更新器状态与 `update/settings.json`，因此不涉及账户隔离。
//! 一个产品版本同时覆盖后端与内嵌 UI（见 `spec/update.md`），`current_version` 即二者共同的版本。

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::error::{AppError, AppResult, ErrorResponse};
use crate::handlers::admin::AdminGuard;
use crate::models::update::{InstallUpdateDto, UpdateSettingsDto, UpdateStatusResponse};
use crate::openapi::ApiResponse;
use crate::state::AppState;
use crate::update::UpdateError;

fn status_response(state: &AppState) -> UpdateStatusResponse {
    UpdateStatusResponse {
        enabled: state.updater.enabled(),
        target: crate::version::TARGET,
        snapshot: state.updater.snapshot(),
        settings: state.updater.settings(),
    }
}

/// 把更新器的前置条件失败映射为带稳定错误码的 409；其余是内部错误。
fn map_update_error(e: UpdateError) -> AppError {
    match e {
        UpdateError::Disabled => AppError::CodedConflict("UPDATE_DISABLED", e.to_string()),
        UpdateError::NoUpdate => AppError::CodedConflict("UPDATE_NOT_AVAILABLE", e.to_string()),
        UpdateError::Busy(_) => AppError::CodedConflict("UPDATE_TRANSFERS_ACTIVE", e.to_string()),
        UpdateError::InProgress => AppError::CodedConflict("UPDATE_IN_PROGRESS", e.to_string()),
        UpdateError::Other(e) => AppError::Other(e),
    }
}

/// GET /admin/update — 当前版本、可用新版本、更新阶段、设置与最近一次错误。
#[utoipa::path(
    get,
    path = "/admin/update",
    tag = "admin",
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "更新器状态（`phase`：idle|checking|downloading|installing|restarting|failed）", body = ApiResponse<UpdateStatusResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
    )
)]
pub async fn status(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> AppResult<ApiResponse<UpdateStatusResponse>> {
    Ok(ApiResponse::new(status_response(&state)))
}

/// POST /admin/update/check — 立即检查一次更新（同步，最长约一分钟）。
///
/// 检查本身失败（网络、验签等）不是请求错误：返回 200，原因在 `last_error`。
#[utoipa::path(
    post,
    path = "/admin/update/check",
    tag = "admin",
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "检查完成；结果见 `available` / `last_error`", body = ApiResponse<UpdateStatusResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
        (status = 409, description = "`UPDATE_DISABLED`（非安装目录运行）或 `UPDATE_IN_PROGRESS`（正在检查 / 安装）", body = ErrorResponse),
    )
)]
pub async fn check(
    State(state): State<AppState>,
    _guard: AdminGuard,
) -> AppResult<ApiResponse<UpdateStatusResponse>> {
    // 放进独立任务：客户端断开不会取消检查，否则阶段会永远停在 checking。
    let updater = state.updater.clone();
    let result = tokio::spawn(async move { updater.check().await })
        .await
        .map_err(|e| AppError::Other(e.into()))?;
    match result {
        Ok(avail) => {
            crate::audit!(
                info,
                "admin.update.check",
                "success",
                available = avail.as_ref().map(|a| a.version.as_str()).unwrap_or("none"),
                "运维手动检查更新"
            );
        }
        Err(UpdateError::Other(e)) => {
            crate::audit!(warn, "admin.update.check", "failure", error = %format!("{e:#}"), "运维手动检查更新失败");
        }
        Err(e) => return Err(map_update_error(e)),
    }
    Ok(ApiResponse::new(status_response(&state)))
}

/// POST /admin/update/install — 安装已发现的新版本；成功后进程重启。
///
/// 立即返回 202，安装在后台进行，用 `GET /admin/update` 观察 `phase`
/// （downloading → installing → restarting，失败为 failed 并带 `last_error`）。
#[utoipa::path(
    post,
    path = "/admin/update/install",
    tag = "admin",
    request_body = Option<InstallUpdateDto>,
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 202, description = "已受理，安装在后台进行", body = ApiResponse<UpdateStatusResponse>),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
        (status = 409, description = "`UPDATE_DISABLED` / `UPDATE_NOT_AVAILABLE`（没有已发现的新版本）/ `UPDATE_IN_PROGRESS` / `UPDATE_TRANSFERS_ACTIVE`（有上传下载在进行，传 `force: true` 可强制）", body = ErrorResponse),
    )
)]
pub async fn install(
    State(state): State<AppState>,
    _guard: AdminGuard,
    dto: Option<Json<InstallUpdateDto>>,
) -> AppResult<(StatusCode, ApiResponse<UpdateStatusResponse>)> {
    let force = dto.map(|Json(d)| d.force).unwrap_or(false);
    let version = state
        .updater
        .snapshot()
        .available
        .map(|a| a.version)
        .unwrap_or_default();
    state
        .updater
        .install_in_background(force)
        .map_err(map_update_error)?;
    crate::audit!(
        info,
        "admin.update.install",
        "accepted",
        version = %version,
        force,
        "运维触发安装更新"
    );
    Ok((
        StatusCode::ACCEPTED,
        ApiResponse::new(status_response(&state)),
    ))
}

/// PUT /admin/update/settings — 修改更新设置并持久化到 `update/settings.json`（立即生效）。
///
/// 只改请求里给出的字段。`window` 须为 `HH:MM-HH:MM`（可跨午夜、起止不同），
/// `check_interval_hours` 须在 1..=720。
#[utoipa::path(
    put,
    path = "/admin/update/settings",
    tag = "admin",
    request_body = UpdateSettingsDto,
    params(("X-Admin-Password" = String, Header, description = "共享运维口令（ADMIN_PASSWORD）")),
    responses(
        (status = 200, description = "已保存；返回最新状态", body = ApiResponse<UpdateStatusResponse>),
        (status = 400, description = "取值无效", body = ErrorResponse),
        (status = 401, description = "口令缺失或错误", body = ErrorResponse),
        (status = 403, description = "非本机 / 内网调用", body = ErrorResponse),
        (status = 404, description = "未配置 ADMIN_PASSWORD，功能未启用", body = ErrorResponse),
        (status = 409, description = "`UPDATE_DISABLED`：非安装目录运行，设置无处持久化", body = ErrorResponse),
    )
)]
pub async fn put_settings(
    State(state): State<AppState>,
    _guard: AdminGuard,
    Json(dto): Json<UpdateSettingsDto>,
) -> AppResult<ApiResponse<UpdateStatusResponse>> {
    if !state.updater.enabled() {
        return Err(map_update_error(UpdateError::Disabled));
    }
    let new = dto.apply(state.updater.settings());
    new.validate()
        .map_err(|e| AppError::BadRequest(format!("{e:#}")))?;
    state.updater.set_settings(new.clone())?;
    crate::audit!(
        info,
        "admin.update.settings",
        "success",
        mode = ?new.mode,
        window = %new.window,
        channel = ?new.channel,
        check_interval_hours = new.check_interval_hours,
        "运维修改更新设置"
    );
    Ok(ApiResponse::new(status_response(&state)))
}

#[cfg(test)]
mod tests {
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

    use super::*;
    use crate::config::{Config, RegistrationMode, RegistrationPolicy};
    use crate::update::settings::{Channel, Mode};

    const PW: &str = "test-admin-pw";

    struct Harness {
        app: Router,
        state: AppState,
        root: PathBuf,
    }

    /// `managed = true` 时在临时安装根里造一个 `current` 符号链接，让更新器认为自己受管。
    async fn harness(managed: bool, base_url: Option<&str>) -> Harness {
        let root = std::env::temp_dir().join(format!("ch-admin-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("versions/0.0.0")).unwrap();
        if managed {
            std::os::unix::fs::symlink(root.join("versions/0.0.0"), root.join("current")).unwrap();
        }
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
            oauth_enabled: false,
            update_mode: Mode::Manual,
            update_window: "03:00-05:00".into(),
            update_channel: Channel::Stable,
            update_check_interval_hours: 6,
            update_root: root.clone(),
            update_base_url: base_url.map(str::to_string),
            registration: RegistrationPolicy {
                mode: RegistrationMode::Approval,
                rate_per_ip_per_hour: 5,
                pending_max: 20,
                pending_expire_days: 14,
            },
            chat: crate::config::ChatConfig {
                provider: crate::config::ChatProviderKind::None,
                bot_token: None,
                language: crate::config::ChatLanguage::En,
                mask_identifiers: true,
            },
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
        let state = AppState {
            db,
            config,
            drive_manager,
            drive_id: "d".into(),
            events,
            activity,
            shutdown,
            oauth: crate::auth::oauth::OAuthRegistry::new(false),
            updater,
            chat,
        };
        Harness {
            app: crate::handlers::build_router(state.clone()),
            state,
            root,
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
        let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, json)
    }

    async fn admin(h: &Harness, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        call(h, method, path, body, Some(PW), "127.0.0.1").await
    }

    fn code(v: &Value) -> &str {
        v["error"]["code"].as_str().unwrap()
    }

    #[tokio::test]
    async fn all_endpoints_are_guarded() {
        let h = harness(true, None).await;
        let routes = [
            ("GET", "/admin/update"),
            ("POST", "/admin/update/check"),
            ("POST", "/admin/update/install"),
            ("PUT", "/admin/update/settings"),
        ];
        for (m, p) in routes {
            let body = (m == "PUT").then(|| json!({}));
            assert_eq!(
                call(&h, m, p, body.clone(), None, "127.0.0.1").await.0,
                401,
                "{m} {p}"
            );
            assert_eq!(
                call(&h, m, p, body.clone(), Some("wrong"), "127.0.0.1")
                    .await
                    .0,
                401,
                "{m} {p}"
            );
            assert_eq!(
                call(&h, m, p, body, Some(PW), "8.8.8.8").await.0,
                403,
                "{m} {p}"
            );
        }
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn status_reports_version_phase_and_settings() {
        let h = harness(true, None).await;
        let (s, v) = admin(&h, "GET", "/admin/update", None).await;
        assert_eq!(s, 200);
        let d = &v["data"];
        assert_eq!(d["enabled"], true);
        assert_eq!(d["current_version"], crate::version::VERSION);
        assert_eq!(d["phase"], "idle");
        assert_eq!(d["available"], Value::Null);
        assert_eq!(d["last_error"], Value::Null);
        assert_eq!(d["settings"]["mode"], "manual");
        assert_eq!(d["settings"]["window"], "03:00-05:00");
        assert_eq!(d["settings"]["channel"], "stable");
        assert_eq!(d["settings"]["check_interval_hours"], 6);

        h.state.updater.seed_available_for_test("99.0.0");
        let (_, v) = admin(&h, "GET", "/admin/update", None).await;
        assert_eq!(v["data"]["available"]["version"], "99.0.0");
        assert!(v["data"]["available"]["notes_url"].is_string());
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn unmanaged_install_reports_disabled() {
        let h = harness(false, None).await;
        let (_, v) = admin(&h, "GET", "/admin/update", None).await;
        assert_eq!(v["data"]["enabled"], false);
        for (m, p, b) in [
            ("POST", "/admin/update/check", None),
            ("POST", "/admin/update/install", None),
            (
                "PUT",
                "/admin/update/settings",
                Some(json!({"mode": "auto"})),
            ),
        ] {
            let (s, v) = admin(&h, m, p, b).await;
            assert_eq!((s, code(&v)), (409, "UPDATE_DISABLED"), "{p}");
        }
        // 没有写出任何设置文件。
        assert!(!h.root.join("update").exists());
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn check_failure_is_recorded_not_an_http_error() {
        // 端口 1 无人监听：连接被拒，检查失败。
        let h = harness(true, Some("http://127.0.0.1:1")).await;
        let (s, v) = admin(&h, "POST", "/admin/update/check", None).await;
        assert_eq!(s, 200);
        assert_eq!(v["data"]["phase"], "idle");
        assert!(v["data"]["last_error"].is_string());
        assert!(v["data"]["last_checked_at"].is_string());
        assert_eq!(v["data"]["available"], Value::Null);
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn install_without_available_update_is_409() {
        let h = harness(true, None).await;
        let (s, v) = admin(&h, "POST", "/admin/update/install", None).await;
        assert_eq!((s, code(&v)), (409, "UPDATE_NOT_AVAILABLE"));
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn install_refuses_active_transfers_unless_forced() {
        let h = harness(true, None).await;
        h.state.updater.seed_available_for_test("99.0.0");
        let transfer = h.state.activity.begin();

        let (s, v) = admin(&h, "POST", "/admin/update/install", None).await;
        assert_eq!((s, code(&v)), (409, "UPDATE_TRANSFERS_ACTIVE"));
        let (s, v) = admin(
            &h,
            "POST",
            "/admin/update/install",
            Some(json!({"force": false})),
        )
        .await;
        assert_eq!((s, code(&v)), (409, "UPDATE_TRANSFERS_ACTIVE"));

        // force：受理（202）。种子清单没有安装包，后台安装在下载前失败，
        // 状态落到 failed 且带原因；进程不会被关停。
        let (s, v) = admin(
            &h,
            "POST",
            "/admin/update/install",
            Some(json!({"force": true})),
        )
        .await;
        assert_eq!(s, 202);
        assert_eq!(v["data"]["current_version"], crate::version::VERSION);
        let mut phase = Value::Null;
        for _ in 0..50 {
            let (_, v) = admin(&h, "GET", "/admin/update", None).await;
            phase = v["data"]["phase"].clone();
            if phase == "failed" {
                assert!(v["data"]["last_error"].is_string());
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert_eq!(phase, "failed");
        assert!(!h.state.shutdown.is_cancelled());
        drop(transfer);

        // 无传输时无需 force 也受理。
        let (s, _) = admin(&h, "POST", "/admin/update/install", Some(json!({}))).await;
        assert_eq!(s, 202);
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn settings_are_validated_merged_and_persisted() {
        let h = harness(true, None).await;

        let (s, v) = admin(
            &h,
            "PUT",
            "/admin/update/settings",
            Some(json!({"mode": "auto", "window": "23:00-02:00", "channel": "beta", "check_interval_hours": 12})),
        )
        .await;
        assert_eq!(s, 200);
        assert_eq!(v["data"]["settings"]["mode"], "auto");
        assert_eq!(v["data"]["settings"]["window"], "23:00-02:00");
        assert_eq!(v["data"]["settings"]["channel"], "beta");
        assert_eq!(v["data"]["settings"]["check_interval_hours"], 12);

        // 部分更新：只改 mode，其余保持。
        let (s, v) = admin(
            &h,
            "PUT",
            "/admin/update/settings",
            Some(json!({"mode": "manual"})),
        )
        .await;
        assert_eq!(s, 200);
        assert_eq!(v["data"]["settings"]["mode"], "manual");
        assert_eq!(v["data"]["settings"]["window"], "23:00-02:00");

        // 已落盘：用同一设置文件重新加载得到相同结果。
        let reloaded = crate::update::settings::Settings::load(
            &h.root,
            crate::update::settings::Settings {
                mode: Mode::Auto,
                window: "00:00-01:00".into(),
                channel: Channel::Stable,
                check_interval_hours: 1,
            },
        );
        assert_eq!(reloaded, h.state.updater.settings());
        assert_eq!(reloaded.channel, Channel::Beta);

        // 无效取值：400，且不改动现有设置。
        for bad in [
            json!({"window": "25:00-01:00"}),
            json!({"window": "03:00-03:00"}),
            json!({"window": "nonsense"}),
            json!({"check_interval_hours": 0}),
            json!({"check_interval_hours": 721}),
        ] {
            let (s, v) = admin(&h, "PUT", "/admin/update/settings", Some(bad.clone())).await;
            assert_eq!((s, code(&v)), (400, "BAD_REQUEST"), "{bad}");
        }
        for bad in [
            json!({"mode": "sometimes"}),
            json!({"channel": "nightly"}),
            json!({"extra": 1}),
        ] {
            let (s, _) = admin(&h, "PUT", "/admin/update/settings", Some(bad.clone())).await;
            assert!(s == 400 || s == 422, "{bad} -> {s}");
        }
        assert_eq!(h.state.updater.settings(), reloaded);
        let _ = std::fs::remove_dir_all(&h.root);
    }

    #[tokio::test]
    async fn openapi_lists_update_endpoints() {
        let h = harness(true, None).await;
        let req = Request::builder()
            .uri("/openapi.json")
            .body(Body::empty())
            .unwrap();
        let resp = h.app.clone().oneshot(req).await.unwrap();
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 22)
            .await
            .unwrap();
        let spec: Value = serde_json::from_slice(&bytes).unwrap();
        let paths = &spec["paths"];
        assert!(paths["/api/v1/admin/update"]["get"].is_object());
        assert!(paths["/api/v1/admin/update/check"]["post"].is_object());
        assert!(paths["/api/v1/admin/update/install"]["post"].is_object());
        assert!(paths["/api/v1/admin/update/settings"]["put"].is_object());
        let _ = std::fs::remove_dir_all(&h.root);
    }
}
