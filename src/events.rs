//! 全局广播通道：文件/资料夹发生变更时通知所有 SSE 订阅者。

use tokio::sync::broadcast;

use crate::models::event::FileChangeEvent;

/// 一次归属于具体账户的存储变更。
#[derive(Debug, Clone)]
pub struct Change {
    pub owner_id: String,
    pub payload: FileChangeEvent,
}

/// 站内分享通知的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareNoticeKind {
    /// 有人把东西分享给了你（SSE `share.received`）。
    Received,
    /// 分享被撤销或你被移出收件人（SSE `share.revoked`）。
    Revoked,
}

/// 发给某个收件人账户的分享通知。
#[derive(Debug, Clone)]
pub struct ShareNotice {
    pub user_id: String,
    pub kind: ShareNoticeKind,
    pub share_id: String,
}

/// 广播到所有 SSE 连接的事件。
#[derive(Debug, Clone)]
pub enum ChangeEvent {
    Changed(Box<Change>),
    /// 站内分享通知，只有 `user_id` 对应账户的连接会转发。
    Share(Box<ShareNotice>),
    /// 全局失效：跨账户批量操作（如回收站定时清理）后使用，不逐个通知。
    RefreshAll,
    /// 发现了新版本（载荷为版本号）。
    UpdateAvailable(String),
}

/// 存储变更通知器，克隆开销低（内部共享一个 `broadcast::Sender`）。
#[derive(Clone)]
pub struct ChangeNotifier {
    tx: broadcast::Sender<ChangeEvent>,
}

impl ChangeNotifier {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(256);
        Self { tx }
    }

    /// 通知某个账户的存储发生了变化。
    pub fn notify(&self, owner_id: impl Into<String>, payload: FileChangeEvent) {
        // 无订阅者时返回 Err，这是空闲时的正常状态，不应传播为错误。
        let _ = self.tx.send(ChangeEvent::Changed(Box::new(Change {
            owner_id: owner_id.into(),
            payload,
        })));
    }

    /// 通知某个收件人：分享到达或被收回。
    pub fn notify_share(&self, user_id: &str, kind: ShareNoticeKind, share_id: &str) {
        let _ = self.tx.send(ChangeEvent::Share(Box::new(ShareNotice {
            user_id: user_id.to_string(),
            kind,
            share_id: share_id.to_string(),
        })));
    }

    /// 通知所有账户重新拉取（范围未知的批量变更）。
    pub fn notify_all(&self) {
        let _ = self.tx.send(ChangeEvent::RefreshAll);
    }

    /// 通知所有连接：有新版本可装。
    pub fn notify_update(&self, version: &str) {
        let _ = self.tx.send(ChangeEvent::UpdateAvailable(version.to_string()));
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ChangeEvent> {
        self.tx.subscribe()
    }
}

impl Default for ChangeNotifier {
    fn default() -> Self {
        Self::new()
    }
}
