//! 回收站定时清理任务：每 `interval_hours` 小时清理一次过期文件。

use std::time::Duration;

use crate::db::Db;
use crate::drive::DriveManager;
use crate::services::trash_service;

/// 启动清理循环（在 `main` 中 `tokio::spawn`）。
pub fn spawn(db: Db, drive_manager: DriveManager, interval_hours: u64) {
    let interval = Duration::from_secs(interval_hours.max(1) * 3600);

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;

            let data_root = match drive_manager.require_data_root().await {
                Ok(root) => root,
                Err(_) => {
                    tracing::debug!("硬盘不可用，跳过本轮回收站清理");
                    continue;
                }
            };

            match trash_service::cleanup_expired(&db, &data_root).await {
                Ok(0) => {}
                Ok(n) => tracing::info!(cleaned = n, "回收站定时清理完成"),
                Err(e) => tracing::warn!(error = %e, "回收站定时清理出错"),
            }
        }
    });
}
