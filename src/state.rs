use std::sync::Arc;

use crate::config::Config;
use crate::db::Db;
use crate::drive::DriveManager;
use crate::events::ChangeNotifier;

/// 各 handler 共享的应用状态。
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub config: Arc<Config>,
    pub drive_manager: DriveManager,
    /// 当前数据盘在 drives 表中的行 id。
    pub drive_id: String,
    /// 存储变更广播通知器，供 SSE 端点订阅。
    pub events: ChangeNotifier,
}
