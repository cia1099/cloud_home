//! 后台任务：定时检查更新；自动模式下在时间窗内、设备空闲时安装。

use std::time::Duration;

use chrono::Timelike;

use super::settings::Mode;
use super::{UpdateError, Updater};

/// 每隔多久醒来评估一次（检查是否到期 + 自动安装条件）。
const TICK: Duration = Duration::from_secs(10 * 60);
/// 启动后先等一会再首检，别和启动期的磁盘/数据库初始化抢资源。
const FIRST_DELAY: Duration = Duration::from_secs(60);

pub fn spawn(updater: Updater) {
    tokio::spawn(async move {
        tokio::time::sleep(FIRST_DELAY).await;
        let mut ticker = tokio::time::interval(TICK);
        loop {
            ticker.tick().await;
            tick(&updater).await;
        }
    });
}

async fn tick(updater: &Updater) {
    if updater.check_due() {
        match updater.check().await {
            Ok(Some(a)) => tracing::info!(version = %a.version, "有可用更新"),
            Ok(None) => tracing::debug!("已是最新版本"),
            Err(UpdateError::InProgress) => return,
            Err(e) => tracing::warn!(error = %e, "检查更新失败"),
        }
    }

    let settings = updater.settings();
    if settings.mode != Mode::Auto || updater.snapshot().available.is_none() {
        return;
    }
    let Ok(window) = settings.window() else {
        return;
    };
    let now = chrono::Local::now();
    if !window.contains(now.hour() * 60 + now.minute()) {
        return;
    }
    // 不强制：有传输进行中就留到下一轮，绝不打断用户的上传/下载。
    match updater.install(false).await {
        Ok(()) | Err(UpdateError::Busy(_)) => {}
        Err(e) => tracing::warn!(error = %e, "自动安装更新失败"),
    }
}
