mod auth;
mod chat;
mod config;
mod db;
mod drive;
mod error;
mod events;
mod handlers;
mod lifecycle;
mod logging;
mod models;
mod openapi;
mod services;
mod state;
mod tasks;
mod update;
mod util;
mod version;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;

use crate::config::Config;
use crate::drive::{DriveManager, detector, path_resolver};
use crate::state::AppState;
use crate::util::now_rfc3339;

/// 优雅关停的最长等待：超过即放弃等在途传输，强制退出，避免被慢客户端无限期拖住。
const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(60);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 命令行开关必须在加载配置之前处理：更新器会在新版本上跑 `--self-test`，
    // 此时不应要求 .env、也不应碰数据库。
    match std::env::args().nth(1).as_deref() {
        Some("--version") | Some("-V") => {
            println!("cloud_home {} ({})", version::VERSION, version::TARGET);
            return Ok(());
        }
        Some("--self-test") => {
            // 能走到这里，说明二进制在本机 CPU/OS 上可执行、动态依赖齐全。
            println!("ok {} {}", version::VERSION, version::TARGET);
            return Ok(());
        }
        _ => {}
    }

    // 先加载配置（内部会 dotenvy 读取 .env），再初始化日志——
    // 这样 .env 里的 RUST_LOG 才能对 EnvFilter 生效。
    let config = Arc::new(Config::from_env().context("加载配置失败")?);
    // 必须持有 guard 到进程结束，否则文件日志的非阻塞写线程会停止刷新。
    let _log_guards = logging::init(&config);

    let drive_manager = DriveManager::new(config.drive_data_dir.clone());

    // 1. 侦测外接硬盘，确定数据盘挂载点。
    let mount = select_mount(&config)?;
    tracing::info!(mount = %mount.display(), "使用数据盘");

    // 2. 准备硬盘目录结构。
    let data_root = path_resolver::data_root(&mount, &config.drive_data_dir);
    tokio::fs::create_dir_all(data_root.join("users"))
        .await
        .context("创建数据目录失败")?;
    let marker = data_root.join(detector::DRIVE_MARKER);
    if !marker.exists() {
        tokio::fs::write(&marker, b"cloud_home\n").await.ok();
    }

    // 3. 初始化数据库连接池并运行迁移。
    let db_path = path_resolver::db_path(&data_root);
    // 启动守卫必须在迁移之前：新版本若反复起不来，要在它再次改动数据库前回滚（并还原备份）。
    update::boot_guard::run(
        &config.update_root,
        update::is_managed(&config.update_root),
        version::VERSION,
        &db_path,
    )?;
    let pool = db::init_pool(&db_path).await.context("初始化数据库失败")?;
    db::run_migrations(&pool).await.context("运行迁移失败")?;

    // 4. 登记数据盘到 drives 表，取得 drive_id。
    let drive_id = upsert_drive(&pool, &mount).await?;

    // 5. 标记硬盘可用。
    drive_manager.set_active(Some(mount.clone())).await;

    let events = crate::events::ChangeNotifier::new();
    let shutdown = tokio_util::sync::CancellationToken::new();

    let activity = lifecycle::ActivityTracker::new();
    let updater = update::Updater::new(
        config.clone(),
        pool.clone(),
        drive_manager.clone(),
        activity.clone(),
        shutdown.clone(),
        events.clone(),
    )?;

    let chat = chat::ChatService::from_config(
        pool.clone(),
        &config.chat,
        chat::AdminEnv {
            drive_manager: drive_manager.clone(),
            events: events.clone(),
            trash_retention_days: config.trash_retention_days,
        },
    )
    .context("初始化聊天集成失败")?;

    let state = AppState {
        db: pool.clone(),
        config: config.clone(),
        drive_manager: drive_manager.clone(),
        drive_id,
        events: events.clone(),
        activity,
        shutdown: shutdown.clone(),
        updater: updater.clone(),
        chat: chat.clone(),
    };

    // 6. 启动后台任务：回收站清理 + 硬盘监控。
    tasks::trash_cleaner::spawn(
        pool.clone(),
        drive_manager.clone(),
        config.trash_cleanup_interval_hours,
        events.clone(),
    );
    tasks::pending_expirer::spawn(pool.clone(), config.registration.pending_expire_days);
    tasks::drive_monitor::spawn(config.clone(), drive_manager.clone(), mount.clone());
    update::start(&updater);
    chat.spawn(shutdown.clone());

    // 7. 启动 HTTP 服务。
    let app = handlers::build_router(state);
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("绑定 {addr} 失败"))?;
    tracing::info!(%addr, "Cloud Home 后端已启动");

    // into_make_service_with_connect_info 让 handler 能拿到 TCP 对端地址（client_ip）。
    let serve = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(lifecycle::shutdown_signal(shutdown.clone()));

    // 优雅关停：停止接新连接、等在途请求收尾；但最多等 SHUTDOWN_GRACE。
    // 令牌一旦被取消（信号或更新器），就开始计时。
    tokio::select! {
        res = serve => res.context("服务运行出错")?,
        _ = async {
            shutdown.cancelled().await;
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        } => {
            tracing::warn!(grace_secs = SHUTDOWN_GRACE.as_secs(), "优雅关停超时，强制退出");
        }
    }
    tracing::info!("Cloud Home 后端已停止");
    Ok(())
}

/// 选择数据盘挂载点：
/// 1. 侦测到的外接硬盘（优先含 marker 者）；
/// 2. 环境变量 `DEV_DRIVE_PATH`（开发环境无外接硬盘时使用）；
/// 3. 当前目录下的 `./dev_drive`（最后兜底，便于本地开发/测试）。
fn select_mount(config: &Config) -> anyhow::Result<PathBuf> {
    let detected = detector::scan(&config.volumes_watch_path, &config.drive_data_dir);
    if let Some(first) = detected.into_iter().next() {
        return Ok(first);
    }

    if let Ok(dev) = std::env::var("DEV_DRIVE_PATH") {
        let path = PathBuf::from(dev);
        std::fs::create_dir_all(&path).context("创建 DEV_DRIVE_PATH 失败")?;
        tracing::warn!(path = %path.display(), "未侦测到外接硬盘，使用 DEV_DRIVE_PATH");
        return Ok(path);
    }

    let fallback = std::env::current_dir()?.join("dev_drive");
    std::fs::create_dir_all(&fallback).context("创建 dev_drive 兜底目录失败")?;
    tracing::warn!(path = %fallback.display(), "未侦测到外接硬盘，使用 ./dev_drive 兜底（仅供开发）");
    Ok(fallback)
}

/// 将数据盘登记到 drives 表；已存在则更新 last_seen_at，返回 drive_id。
async fn upsert_drive(pool: &db::Db, mount: &std::path::Path) -> anyhow::Result<String> {
    let mount_str = mount.display().to_string();
    let now = now_rfc3339();

    let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM drives WHERE mount_path = ?")
        .bind(&mount_str)
        .fetch_optional(pool)
        .await?;

    if let Some((id,)) = existing {
        sqlx::query("UPDATE drives SET is_active = 1, last_seen_at = ? WHERE id = ?")
            .bind(&now)
            .bind(&id)
            .execute(pool)
            .await?;
        return Ok(id);
    }

    let id = uuid::Uuid::new_v4().to_string();
    let label = mount
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());
    sqlx::query(
        "INSERT INTO drives (id, mount_path, label, is_active, detected_at, last_seen_at) \
         VALUES (?, ?, ?, 1, ?, ?)",
    )
    .bind(&id)
    .bind(&mount_str)
    .bind(label)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await?;

    Ok(id)
}
