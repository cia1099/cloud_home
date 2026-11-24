//! 待批准申请过期任务：每小时把超过 `PENDING_EXPIRE_DAYS` 仍未处理的申请自动拒绝。

use std::time::Duration;

use crate::db::Db;
use crate::services::approval_service;

/// 启动过期循环（在 `main` 中调用）。
pub fn spawn(db: Db, expire_days: i64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        loop {
            ticker.tick().await;
            if let Err(e) = approval_service::expire_pending(&db, expire_days).await {
                tracing::warn!(error = %e, "待批准申请过期处理出错");
            }
        }
    });
}
