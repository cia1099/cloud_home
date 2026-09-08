mod auth;
mod config;
mod db;
mod drive;
mod error;
mod handlers;
mod models;
mod openapi;
mod services;
mod state;
mod tasks;
mod util;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::drive::{DriveManager, detector, path_resolver};
use crate::state::AppState;
use crate::util::now_rfc3339;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Arc::new(Config::from_env().context("加载配置失败")?);

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
    let pool = db::init_pool(&db_path).await.context("初始化数据库失败")?;
    db::run_migrations(&pool).await.context("运行迁移失败")?;

    // 4. 登记数据盘到 drives 表，取得 drive_id。
    let drive_id = upsert_drive(&pool, &mount).await?;

    // 5. 标记硬盘可用。
    drive_manager.set_active(Some(mount.clone())).await;

    let state = AppState {
        db: pool.clone(),
        config: config.clone(),
        drive_manager: drive_manager.clone(),
        drive_id,
    };

    // 6. 启动后台任务：回收站清理 + 硬盘监控。
    tasks::trash_cleaner::spawn(
        pool.clone(),
        drive_manager.clone(),
        config.trash_cleanup_interval_hours,
    );
    tasks::drive_monitor::spawn(config.clone(), drive_manager.clone(), mount.clone());

    // 7. 启动 HTTP 服务。
    let app = handlers::build_router(state);
    let addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("绑定 {addr} 失败"))?;
    tracing::info!(%addr, "Cloud Home 后端已启动");

    axum::serve(listener, app).await.context("服务运行出错")?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cloud_home=info,tower_http=info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
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
