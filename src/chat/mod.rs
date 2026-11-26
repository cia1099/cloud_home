//! 管理员聊天集成（R-063）：把待批准的注册申请推给已配对的管理员，
//! 管理员在聊天里一键批准 / 拒绝。
//!
//! - `provider`：提供方 trait 与归一化类型；`telegram`：长轮询实现（只出站）；
//! - `pairing`：一次性配对码与管理员身份；
//! - `outbox`：待发送队列（指数退避重试）、突发合并、每日提醒、决定后编辑原消息；
//! - `dispatch`：处理指令与按钮点击。
//!
//! 审批本身永远走 `approval_service::decide`，与 `/admin/*` 端点权限、审计一致。
//! 聊天不可用不影响注册与审批：申请照常 pending，消息留在队列里等恢复。

mod dispatch;
mod outbox;
mod pairing;
pub mod provider;
pub mod telegram;
mod text;
#[cfg(test)]
mod flow_tests;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

use crate::config::{ChatConfig, ChatProviderKind};
use crate::db::Db;
use crate::error::AppResult;
use crate::models::chat::{ChatAdminItem, ChatStatusResponse};
use provider::ChatProvider;
use text::Texts;

/// 按钮令牌有效期。
const TOKEN_TTL_HOURS: i64 = 24;
/// 发送队列轮询间隔（有新消息时会被唤醒提前处理）。
const OUTBOX_TICK: Duration = Duration::from_secs(15);
/// 拉取更新失败后的等待。
const POLL_RETRY: Duration = Duration::from_secs(5);

/// 聊天服务句柄，可廉价克隆；`CHAT_PROVIDER=none` 时所有操作都是空操作。
#[derive(Clone)]
pub struct ChatService {
    inner: Arc<Inner>,
}

struct Inner {
    db: Db,
    kind: ChatProviderKind,
    provider: Option<Arc<dyn ChatProvider>>,
    texts: Texts,
    mask_identifiers: bool,
    /// 最近一次拉取更新是否成功。
    connected: AtomicBool,
    /// 有新消息入队时唤醒发送循环。
    wake: Notify,
    /// 配对码错误次数的限流（内存；重启即清零）。
    pair_failures: Mutex<RateLimiter>,
    /// 未授权发送者的审计限流，防日志被灌满。
    unauthorized_audit: Mutex<RateLimiter>,
}

impl ChatService {
    /// 按配置建立服务：`telegram` 时创建长轮询提供方，`none` 时为空操作。
    pub fn from_config(db: Db, cfg: &ChatConfig) -> anyhow::Result<Self> {
        let provider: Option<Arc<dyn ChatProvider>> = match (cfg.provider, &cfg.bot_token) {
            (ChatProviderKind::Telegram, Some(token)) => {
                Some(Arc::new(telegram::TelegramProvider::new(token)?))
            }
            _ => None,
        };
        Ok(Self::new(db, cfg, provider))
    }

    pub fn new(db: Db, cfg: &ChatConfig, provider: Option<Arc<dyn ChatProvider>>) -> Self {
        Self {
            inner: Arc::new(Inner {
                db,
                kind: cfg.provider,
                provider,
                texts: Texts(cfg.language),
                mask_identifiers: cfg.mask_identifiers,
                connected: AtomicBool::new(false),
                wake: Notify::new(),
                pair_failures: Mutex::new(RateLimiter::new(Duration::from_secs(600))),
                unauthorized_audit: Mutex::new(RateLimiter::new(Duration::from_secs(60))),
            }),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.provider.is_some()
    }

    /// `GET /admin/chat` 的内容。
    pub async fn status(&self) -> AppResult<ChatStatusResponse> {
        let admins = self
            .identities()
            .await?
            .into_iter()
            .map(|i| ChatAdminItem {
                id: i.id,
                label: i.label,
                paired_at: i.paired_at,
            })
            .collect();
        Ok(ChatStatusResponse {
            provider: self.inner.kind.as_str().into(),
            connected: self.is_enabled() && self.inner.connected.load(Ordering::Relaxed),
            admins,
        })
    }

    /// 启动后台任务：长轮询收更新 + 发送队列循环。未启用时什么也不做。
    pub fn spawn(&self, shutdown: CancellationToken) {
        if !self.is_enabled() {
            return;
        }
        let svc = self.clone();
        let stop = shutdown.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop.cancelled() => break,
                    ok = svc.poll_once() => {
                        if !ok {
                            tokio::select! {
                                _ = stop.cancelled() => break,
                                _ = tokio::time::sleep(POLL_RETRY) => {}
                            }
                        }
                    }
                }
            }
        });

        let svc = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = svc.inner.wake.notified() => {}
                    _ = tokio::time::sleep(OUTBOX_TICK) => {}
                }
                svc.deliver_due().await;
                svc.remind_if_due().await;
            }
        });
    }

    /// 拉取并处理一批更新；拉取失败返回 false（调用方稍后重试）。
    async fn poll_once(&self) -> bool {
        let Some(provider) = &self.inner.provider else {
            return false;
        };
        match provider.next_updates().await {
            Ok(updates) => {
                self.inner.connected.store(true, Ordering::Relaxed);
                for u in updates {
                    self.handle_update(u).await;
                }
                true
            }
            Err(e) => {
                self.inner.connected.store(false, Ordering::Relaxed);
                tracing::warn!(error = %e, "聊天：拉取更新失败，稍后重试");
                false
            }
        }
    }
}

/// 令牌 / 配对码在库里只存 SHA-256 十六进制摘要。
fn digest(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

/// 滑动窗口限流（内存）。
struct RateLimiter {
    window: Duration,
    hits: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    fn new(window: Duration) -> Self {
        Self {
            window,
            hits: HashMap::new(),
        }
    }

    /// 窗口内已记录次数少于 `max` 才返回 true。
    fn allow(&mut self, key: &str, max: usize) -> bool {
        let now = Instant::now();
        let window = self.window;
        let v = self.hits.entry(key.to_string()).or_default();
        v.retain(|t| now.duration_since(*t) < window);
        v.len() < max
    }

    fn record(&mut self, key: &str) {
        self.hits
            .entry(key.to_string())
            .or_default()
            .push(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::RateLimiter;

    #[test]
    fn rate_limiter_blocks_after_max_and_is_per_key() {
        let mut rl = RateLimiter::new(Duration::from_secs(60));
        for _ in 0..3 {
            assert!(rl.allow("a", 3));
            rl.record("a");
        }
        assert!(!rl.allow("a", 3));
        assert!(rl.allow("b", 3));
    }
}
