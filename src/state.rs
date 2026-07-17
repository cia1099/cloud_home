use std::sync::Arc;

use crate::config::Config;
use crate::db::Db;
use crate::drive::DriveManager;

/// 各 handler 共享的应用状态。
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub drive_manager: DriveManager,
    /// 当前数据盘在 drives 表中的行 id。
    pub drive_id: String,
}
