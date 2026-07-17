//! 外接硬盘管理：持有当前激活硬盘挂载点，供各处查询与写操作前校验。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::error::{AppError, AppResult};

use super::path_resolver;

/// 硬盘管理器。挂载点由后台监控任务更新，卸载时置为 `None`。
#[derive(Clone)]
pub struct DriveManager {
    mount_path: Arc<RwLock<Option<PathBuf>>>,
    data_dir: String,
}

impl DriveManager {
    pub fn new(data_dir: impl Into<String>) -> Self {
        Self {
            mount_path: Arc::new(RwLock::new(None)),
            data_dir: data_dir.into(),
        }
    }

    /// 更新当前激活的挂载点（`None` 表示无可用硬盘）。
    pub async fn set_active(&self, path: Option<PathBuf>) {
        let mut guard = self.mount_path.write().await;
        *guard = path;
    }

    /// 当前挂载点，若无则为 `None`。
    pub async fn active_mount(&self) -> Option<PathBuf> {
        self.mount_path.read().await.clone()
    }

    /// 硬盘是否可用。
    pub async fn is_available(&self) -> bool {
        self.mount_path.read().await.is_some()
    }

    /// 获取数据根目录；硬盘不可用时返回 `503 DRIVE_UNAVAILABLE`。
    pub async fn require_data_root(&self) -> AppResult<PathBuf> {
        match self.active_mount().await {
            Some(mount) => Ok(path_resolver::data_root(&mount, &self.data_dir)),
            None => Err(AppError::DriveUnavailable),
        }
    }

    /// 数据目录名（如 `cloud_home_data`）。
    #[allow(dead_code)]
    pub fn data_dir(&self) -> &str {
        &self.data_dir
    }

    /// 基于给定挂载点计算数据根目录（不校验是否激活）。
    #[allow(dead_code)]
    pub fn data_root_of(&self, mount: &Path) -> PathBuf {
        path_resolver::data_root(mount, &self.data_dir)
    }
}
