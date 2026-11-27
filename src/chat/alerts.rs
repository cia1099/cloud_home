//! 系统告警（R-065）：新版本（带「立即安装」按钮）、数据盘插拔、空间不足、管理口令反复失败。
//!
//! - 每类告警在配置里各有开关（`CHAT_ALERT_*`），关掉的类别什么都不发；
//! - 防刷屏：新版本每个版本只提醒一次；数据盘插拔每 10 分钟最多 6 条；空间不足每 6 小时最多一条；
//!   管理口令失败 10 分钟内累计 5 次才提醒一次，之后 30 分钟内不再提醒。限流全在内存里，重启即清零；
//! - 告警**直接发送**、不走发送队列：它们有时效，过期重发只是噪音；而且「数据盘被拔出」时
//!   数据库就在那块盘上，入队会失败。收件人取自数据库，读不到时退回内存里最近一次读到的管理员名单；
//! - 只发给已配对的管理员，内容只含版本号、盘的状态与百分比、失败次数与来源 IP，不含任何用户数据或密钥。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio_util::sync::CancellationToken;

use super::outbox::new_token;
use super::pairing::Identity;
use super::provider::{Button, Message};
use super::{ChatService, RateLimiter, TOKEN_TTL_HOURS, digest};
use crate::error::AppResult;
use crate::events::ChangeEvent;
use crate::util::now_rfc3339;

/// 同一版本的更新提醒间隔（实际上等于「每个版本一次」）。
const UPDATE_COOLDOWN: Duration = Duration::from_secs(24 * 3600);
/// 数据盘插拔提醒：窗口内最多这么多条（接触不良反复抖动时不刷屏）。
const DRIVE_WINDOW: Duration = Duration::from_secs(600);
const DRIVE_MAX: usize = 6;
/// 空间不足的复提醒间隔与检查间隔。
const DISK_COOLDOWN: Duration = Duration::from_secs(6 * 3600);
const DISK_CHECK_EVERY: Duration = Duration::from_secs(300);
/// 管理口令失败：窗口、触发次数、触发后的静默期。
const AUTH_WINDOW: Duration = Duration::from_secs(600);
const AUTH_THRESHOLD: usize = 5;
const AUTH_COOLDOWN: Duration = Duration::from_secs(1800);

/// 告警的内存状态。
pub(super) struct AlertState {
    cooldowns: HashMap<String, Instant>,
    drive: RateLimiter,
    auth: RateLimiter,
    /// 最近一次成功读到的管理员名单（数据库读不到时的后备）。
    admins: Vec<Identity>,
}

impl AlertState {
    pub(super) fn new() -> Self {
        Self {
            cooldowns: HashMap::new(),
            drive: RateLimiter::new(DRIVE_WINDOW),
            auth: RateLimiter::new(AUTH_WINDOW),
            admins: Vec::new(),
        }
    }

    /// 该 key 距上次放行已超过 `every` 才放行（并记下本次）。
    fn cooldown_ok(&mut self, key: &str, every: Duration) -> bool {
        let now = Instant::now();
        match self.cooldowns.get(key) {
            Some(t) if now.duration_since(*t) < every => false,
            _ => {
                self.cooldowns.insert(key.to_string(), now);
                true
            }
        }
    }
}

/// 启动告警的后台观察者：订阅「发现新版本」事件，并定期检查剩余空间。
pub(super) fn spawn_watchers(svc: &ChatService, shutdown: CancellationToken) {
    let this = svc.clone();
    let stop = shutdown.clone();
    tokio::spawn(async move {
        // 先读一次名单放进后备缓存。
        let _ = this.alert_recipients().await;
        let mut rx = this.inner.env.events.subscribe();
        loop {
            tokio::select! {
                _ = stop.cancelled() => break,
                ev = rx.recv() => match ev {
                    Ok(ev) => this.on_event(ev).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                },
            }
        }
    });

    let this = svc.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(DISK_CHECK_EVERY);
        loop {
            tokio::select! {
                _ = shutdown.cancelled() => break,
                _ = ticker.tick() => this.check_disk().await,
            }
        }
    });
}

impl ChatService {
    fn wants(&self, switch: bool) -> bool {
        switch && self.is_enabled()
    }

    pub(super) async fn on_event(&self, ev: ChangeEvent) {
        if let ChangeEvent::UpdateAvailable(version) = ev {
            self.alert_update(&version).await;
        }
    }

    /// 发现新版本：每个版本只提醒一次，带「立即安装」按钮。
    async fn alert_update(&self, version: &str) {
        if !self.wants(self.inner.alerts.update) {
            return;
        }
        let first = self
            .inner
            .alert_state
            .lock()
            .unwrap()
            .cooldown_ok(&format!("update:{version}"), UPDATE_COOLDOWN);
        if first {
            self.broadcast(self.inner.texts.update_available(version), Some(version))
                .await;
        }
    }

    /// 数据盘被拔出 / 重新接上。
    pub async fn alert_drive(&self, attached: bool) {
        if !self.wants(self.inner.alerts.drive) {
            return;
        }
        let allowed = {
            let mut st = self.inner.alert_state.lock().unwrap();
            let ok = st.drive.allow("drive", DRIVE_MAX);
            if ok {
                st.drive.record("drive");
            }
            ok
        };
        if allowed {
            let texts = self.inner.texts;
            let text = if attached {
                texts.drive_plugged()
            } else {
                texts.drive_unplugged()
            };
            self.broadcast(text, None).await;
        }
    }

    /// 定期检查数据盘剩余空间。盘不在（已被拔出）时跳过，那是另一类告警的事。
    async fn check_disk(&self) {
        if !self.wants(self.inner.alerts.disk) {
            return;
        }
        let Some(mount) = self.inner.env.drive_manager.active_mount().await else {
            return;
        };
        if let Ok((total, avail)) = crate::services::usage_service::statvfs_bytes(&mount).await {
            self.alert_disk(total.max(0) as u64, avail.max(0) as u64)
                .await;
        }
    }

    /// 剩余比例低于阈值时告警；仍然不足则每 6 小时复提醒一次。
    pub(super) async fn alert_disk(&self, total: u64, avail: u64) {
        if !self.wants(self.inner.alerts.disk) || total == 0 {
            return;
        }
        let free_percent = avail.saturating_mul(100) / total;
        if free_percent >= self.inner.alerts.disk_free_percent {
            return;
        }
        let first = self
            .inner
            .alert_state
            .lock()
            .unwrap()
            .cooldown_ok("disk", DISK_COOLDOWN);
        if first {
            let text = self.inner.texts.low_disk(free_percent, avail >> 30);
            self.broadcast(text, None).await;
        }
    }

    /// 管理端点口令验证失败一次（`ip` 是来源地址，仅用于提醒文字）。
    /// 窗口内累计到阈值才提醒，提醒后静默一段时间。
    pub async fn note_admin_auth_failure(&self, ip: &str) {
        if !self.wants(self.inner.alerts.admin_auth) {
            return;
        }
        let count = {
            let mut st = self.inner.alert_state.lock().unwrap();
            st.auth.record("admin");
            let count = st.auth.count("admin");
            if count < AUTH_THRESHOLD || !st.cooldown_ok("admin_auth", AUTH_COOLDOWN) {
                return;
            }
            count
        };
        let text = self
            .inner
            .texts
            .admin_auth_failures(count, AUTH_WINDOW.as_secs() / 60, ip);
        self.broadcast(text, None).await;
    }

    /// 告警收件人：数据库里的已配对管理员；读不到时用最近一次读到的名单。
    async fn alert_recipients(&self) -> Vec<Identity> {
        match self.identities().await {
            Ok(list) => {
                self.inner.alert_state.lock().unwrap().admins = list.clone();
                list
            }
            Err(e) => {
                tracing::warn!(error = %e, "聊天告警：读取管理员名单失败，改用缓存");
                self.inner.alert_state.lock().unwrap().admins.clone()
            }
        }
    }

    /// 直接发给每位管理员；`install_version` 有值时附「立即安装」按钮。失败只记日志。
    async fn broadcast(&self, text: String, install_version: Option<&str>) {
        let Some(provider) = &self.inner.provider else {
            return;
        };
        for admin in self.alert_recipients().await {
            let mut msg = Message::text(text.clone());
            if let Some(version) = install_version {
                match self.new_update_token(&admin, version).await {
                    Ok(token) => msg.buttons.push(Button {
                        label: self.inner.texts.install_button(),
                        token,
                    }),
                    Err(e) => tracing::warn!(error = %e, "聊天告警：签发安装按钮令牌失败"),
                }
            }
            if let Err(e) = provider.send(&admin.chat_id, &msg).await {
                tracing::warn!(error = %e, "聊天告警：发送失败");
            }
        }
    }

    async fn new_update_token(&self, admin: &Identity, version: &str) -> AppResult<String> {
        let token = new_token();
        let expires = (Utc::now() + chrono::Duration::hours(TOKEN_TTL_HOURS)).to_rfc3339();
        sqlx::query(
            "INSERT INTO chat_update_tokens (token_hash, identity_id, version, expires_at) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(digest(&token))
        .bind(&admin.id)
        .bind(version)
        .bind(&expires)
        .execute(&self.inner.db)
        .await?;
        Ok(token)
    }

    /// 按钮令牌若是「安装更新」令牌则原子消费并返回版本号；否则返回 None（交给审批流程）。
    /// 令牌绑定发起它的管理员：别人的令牌（即使泄露）无效。
    pub(super) async fn claim_update_token(
        &self,
        admin: &Identity,
        token: &str,
    ) -> AppResult<Option<String>> {
        let now = now_rfc3339();
        let claimed: Option<(String,)> = sqlx::query_as(
            "UPDATE chat_update_tokens SET used_at = ? \
             WHERE token_hash = ? AND identity_id = ? AND used_at IS NULL AND expires_at > ? \
             RETURNING version",
        )
        .bind(&now)
        .bind(digest(token))
        .bind(&admin.id)
        .bind(&now)
        .fetch_optional(&self.inner.db)
        .await?;
        Ok(claimed.map(|(v,)| v))
    }

    /// 点击「立即安装」：只有令牌里的版本仍是当前可装版本才安装，走与 `/admin/update/install`
    /// 相同的前置检查（有传输进行中时拒绝，不强制）。
    pub(super) async fn install_from_chat(&self, admin: &Identity, sender_id: &str, version: &str) {
        use crate::update::UpdateError;

        let texts = self.inner.texts;
        let updater = &self.inner.env.updater;
        let offered = updater.snapshot().available.map(|a| a.version);
        if offered.as_deref() != Some(version) {
            self.reply(&admin.chat_id, texts.update_gone()).await;
            return;
        }
        let text = match updater.install_in_background(false) {
            Ok(()) => {
                crate::audit!(
                    info,
                    "chat.update",
                    "install",
                    provider = self.inner.kind.as_str(),
                    sender_id = %sender_id,
                    version = %version,
                    "聊天里点击安装更新"
                );
                texts.update_started(version)
            }
            Err(UpdateError::Busy(n)) => texts.update_busy(n),
            Err(UpdateError::NoUpdate) => texts.update_gone(),
            Err(_) => texts.update_refused(),
        };
        self.reply(&admin.chat_id, text).await;
    }
}
