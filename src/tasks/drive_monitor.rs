//! 外接硬盘挂载监控：定期核对当前数据盘是否仍然挂载，更新 [`DriveManager`]。
//!
//! 使用 `notify` 监听挂载根目录变化以尽快感知插拔，并以 5 秒定时轮询兜底。
//! 卸载时将激活挂载点置为 `None`（写操作随即返回 503），重新挂载后恢复。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::Config;
use crate::drive::{DriveManager, detector};

/// 启动监控任务。`chosen_mount` 为启动时选定的数据盘挂载点。
pub fn spawn(config: Arc<Config>, drive_manager: DriveManager, chosen_mount: PathBuf) {
    tokio::spawn(async move {
        // notify 回调在独立线程运行，仅用于将“可能有变化”置位，实际判定由轮询完成。
        let dirty = Arc::new(AtomicBool::new(false));
        let dirty_cb = dirty.clone();
        let watcher = RecommendedWatcher::new(
            move |_evt| dirty_cb.store(true, Ordering::Relaxed),
            notify::Config::default(),
        );

        // 持有 watcher，避免被 drop 后停止监听。
        let _watcher = match watcher {
            Ok(mut w) => {
                for root in detector::watch_roots(&config.volumes_watch_path) {
                    if let Err(e) = w.watch(&root, RecursiveMode::NonRecursive) {
                        tracing::warn!(root = %root.display(), error = %e, "watch 挂载根目录失败");
                    }
                }
                Some(w)
            }
            Err(e) => {
                tracing::warn!(error = %e, "无法创建硬盘监控 watcher，改用纯轮询");
                None
            }
        };

        let data_dir = config.drive_data_dir.clone();
        let mut last_available = mount_is_available(&chosen_mount, &data_dir);

        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        loop {
            ticker.tick().await;
            // 消费 notify 置位（本轮无论如何都会重新判定）。
            dirty.swap(false, Ordering::Relaxed);

            let available = mount_is_available(&chosen_mount, &data_dir);
            if available != last_available {
                if available {
                    tracing::info!(mount = %chosen_mount.display(), "数据盘重新挂载");
                    drive_manager.set_active(Some(chosen_mount.clone())).await;
                } else {
                    tracing::warn!(mount = %chosen_mount.display(), "数据盘已卸载，写操作将返回 503");
                    drive_manager.set_active(None).await;
                }
                last_available = available;
            }
        }
    });
}

/// 数据盘是否仍挂载：挂载点存在且其下的数据目录存在。
fn mount_is_available(mount: &Path, data_dir: &str) -> bool {
    mount.exists() && mount.join(data_dir).exists()
}
