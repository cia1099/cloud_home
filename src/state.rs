use std::sync::Arc;

use crate::config::Config;
use crate::db::Db;
use crate::drive::DriveManager;
use crate::events::ChangeNotifier;
use crate::lifecycle::ActivityTracker;
use tokio_util::sync::CancellationToken;

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
    /// 在途传输计数（上传/下载/打包），供在线更新判断是否空闲。
    pub activity: ActivityTracker,
    /// 关停令牌：取消即触发优雅关停（SIGTERM、更新重启都走它），SSE 流也监听它。
    pub shutdown: CancellationToken,
    /// 第三方登录 provider 注册表（含总开关）。
    pub oauth: crate::auth::oauth::OAuthRegistry,
    /// 在线更新器。
    pub updater: crate::update::Updater,
    /// 管理员聊天集成（`CHAT_PROVIDER=none` 时为空操作）。
    pub chat: crate::chat::ChatService,
}
