//! 发送队列：新申请通知、突发合并摘要、每日提醒；决定后编辑原消息。
//!
//! 所有通知先落库再由后台循环投递——聊天不可用时消息留在队列里按指数退避重试，
//! 不会丢，也不会阻塞注册。

use chrono::{Duration, Utc};

use super::provider::{Button, Message, MessageRef};
use super::text::{clean_name, fmt_time, show_identifier};
use super::pairing::Identity;
use super::{ChatService, TOKEN_TTL_HOURS, digest};
use crate::error::AppResult;
use crate::models::user::{
    APPROVAL_APPROVED, APPROVAL_PENDING, PROVIDER_EMAIL, PROVIDER_PHONE, User,
};
use crate::services::user_service;
use crate::util::now_rfc3339;

/// 窗口内超过这个数量的新申请只发一条摘要。
const BURST_LIMIT: i64 = 5;
const BURST_WINDOW_MINUTES: i64 = 10;
/// 超过这个次数仍失败就放弃（标 failed 并审计）。
const MAX_ATTEMPTS: i64 = 8;
/// 单次投递循环最多处理的消息数。
const BATCH: i64 = 20;
/// `/pending` 一次最多展示的申请数。
pub(super) const PENDING_LIST_MAX: usize = 10;

/// 第 `attempts` 次失败后的等待：30s、60s、120s……上限 1 小时。
fn backoff(attempts: i64) -> Duration {
    let secs = 30_i64.saturating_mul(1_i64 << (attempts - 1).clamp(0, 20));
    Duration::seconds(secs.min(3600))
}

/// 不透明按钮令牌：16 字节 CSPRNG（32 个十六进制字符，适合 Telegram 的 64 字节 callback_data 上限）。
fn new_token() -> String {
    hex::encode(rand::random::<[u8; 16]>())
}

#[derive(sqlx::FromRow)]
struct Due {
    id: i64,
    chat_id: String,
    kind: String,
    user_id: Option<String>,
    text: String,
    buttons: Option<String>,
    attempts: i64,
}

impl ChatService {
    /// 新注册进入 pending 时调用：给每位已配对管理员入队一条通知。失败只记日志，绝不影响注册。
    pub async fn notify_registration(&self, user: &User) {
        if !self.is_enabled() {
            return;
        }
        let result: AppResult<()> = async {
            for admin in self.identities().await? {
                self.enqueue_notify(&admin, user, true).await?;
            }
            Ok(())
        }
        .await;
        match result {
            Ok(()) => self.inner.wake.notify_one(),
            Err(e) => tracing::warn!(error = %e, "聊天：新申请通知入队失败"),
        }
    }

    /// 为一位管理员入队一条带 Approve / Reject 按钮的通知。
    /// `coalesce` 为 true 时套用突发合并（`/pending` 的主动查看不合并）。
    pub(super) async fn enqueue_notify(
        &self,
        admin: &Identity,
        user: &User,
        coalesce: bool,
    ) -> AppResult<()> {
        let db = &self.inner.db;
        let now = now_rfc3339();

        if coalesce {
            let since = (Utc::now() - Duration::minutes(BURST_WINDOW_MINUTES)).to_rfc3339();
            let (recent,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM chat_outbox \
                 WHERE identity_id = ? AND kind = 'notify' AND created_at > ?",
            )
            .bind(&admin.id)
            .bind(&since)
            .fetch_one(db)
            .await?;
            if recent >= BURST_LIMIT {
                // 记一条不投递的占位行以便继续计数，并保证窗口内只有一条摘要。
                self.insert_row(&admin.id, "notify", Some(&user.id), "", None, "suppressed")
                    .await?;
                let (summaries,): (i64,) = sqlx::query_as(
                    "SELECT COUNT(*) FROM chat_outbox \
                     WHERE identity_id = ? AND kind = 'summary' AND created_at > ?",
                )
                .bind(&admin.id)
                .bind(&since)
                .fetch_one(db)
                .await?;
                if summaries == 0 {
                    let text = self.inner.texts.burst_summary();
                    self.insert_row(&admin.id, "summary", None, &text, None, "queued")
                        .await?;
                }
                return Ok(());
            }
        }

        let contacts: Vec<String> = user_service::identities(db, &user.id)
            .await?
            .iter()
            .filter(|i| i.provider == PROVIDER_EMAIL || i.provider == PROVIDER_PHONE)
            .map(|i| show_identifier(&i.provider, &i.identifier, self.inner.mask_identifiers))
            .collect();
        let text = self.inner.texts.new_registration(
            &clean_name(&user.username),
            &contacts.join(", "),
            &fmt_time(&user.created_at),
            user.registered_ip.as_deref().unwrap_or("?"),
        );

        // 令牌与消息同事务落库：要么都在，要么都不在。
        let expires = (Utc::now() + Duration::hours(TOKEN_TTL_HOURS)).to_rfc3339();
        let mut buttons = Vec::new();
        let mut tx = db.begin().await?;
        for (action, label) in [
            ("approve", self.inner.texts.approve_button()),
            ("reject", self.inner.texts.reject_button()),
        ] {
            let token = new_token();
            sqlx::query(
                "INSERT INTO chat_action_tokens (token_hash, identity_id, user_id, action, expires_at) \
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(digest(&token))
            .bind(&admin.id)
            .bind(&user.id)
            .bind(action)
            .bind(&expires)
            .execute(&mut *tx)
            .await?;
            buttons.push(Button { label, token });
        }
        sqlx::query(
            "INSERT INTO chat_outbox \
             (identity_id, kind, user_id, text, buttons, state, next_attempt_at, created_at) \
             VALUES (?, 'notify', ?, ?, ?, 'queued', ?, ?)",
        )
        .bind(&admin.id)
        .bind(&user.id)
        .bind(&text)
        .bind(serde_json::to_string(&buttons).unwrap_or_default())
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn insert_row(
        &self,
        identity_id: &str,
        kind: &str,
        user_id: Option<&str>,
        text: &str,
        buttons: Option<&str>,
        state: &str,
    ) -> AppResult<()> {
        let now = now_rfc3339();
        sqlx::query(
            "INSERT INTO chat_outbox \
             (identity_id, kind, user_id, text, buttons, state, next_attempt_at, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(identity_id)
        .bind(kind)
        .bind(user_id)
        .bind(text)
        .bind(buttons)
        .bind(state)
        .bind(&now)
        .bind(&now)
        .execute(&self.inner.db)
        .await?;
        Ok(())
    }

    /// 投递所有到期的排队消息。失败按指数退避重试，超过 `MAX_ATTEMPTS` 放弃并审计。
    pub(super) async fn deliver_due(&self) {
        if let Err(e) = self.try_deliver_due().await {
            tracing::warn!(error = %e, "聊天：投递队列出错");
        }
    }

    async fn try_deliver_due(&self) -> AppResult<()> {
        let Some(provider) = &self.inner.provider else {
            return Ok(());
        };
        let db = &self.inner.db;
        let due: Vec<Due> = sqlx::query_as(
            "SELECT o.id, i.chat_id, o.kind, o.user_id, o.text, o.buttons, o.attempts \
             FROM chat_outbox o JOIN admin_chat_identities i ON i.id = o.identity_id \
             WHERE o.state = 'queued' AND o.next_attempt_at <= ? ORDER BY o.id LIMIT ?",
        )
        .bind(now_rfc3339())
        .bind(BATCH)
        .fetch_all(db)
        .await?;

        for row in due {
            // 排队期间已被决定的申请不再打扰管理员。
            if row.kind == "notify"
                && let Some(uid) = &row.user_id
            {
                let status: Option<(String,)> =
                    sqlx::query_as("SELECT approval_status FROM users WHERE id = ?")
                        .bind(uid)
                        .fetch_optional(db)
                        .await?;
                if status.is_none_or(|(s,)| s != APPROVAL_PENDING) {
                    sqlx::query("UPDATE chat_outbox SET state = 'suppressed', buttons = NULL WHERE id = ?")
                        .bind(row.id)
                        .execute(db)
                        .await?;
                    continue;
                }
            }

            let msg = Message {
                text: row.text.clone(),
                buttons: row
                    .buttons
                    .as_deref()
                    .and_then(|b| serde_json::from_str(b).ok())
                    .unwrap_or_default(),
            };
            match provider.send(&row.chat_id, &msg).await {
                Ok(sent) => {
                    sqlx::query(
                        "UPDATE chat_outbox SET state = 'sent', buttons = NULL, sent_ref = ? WHERE id = ?",
                    )
                    .bind(serde_json::to_string(&sent).unwrap_or_default())
                    .bind(row.id)
                    .execute(db)
                    .await?;
                }
                Err(e) => {
                    let attempts = row.attempts + 1;
                    if attempts >= MAX_ATTEMPTS {
                        sqlx::query(
                            "UPDATE chat_outbox SET state = 'failed', buttons = NULL, attempts = ? WHERE id = ?",
                        )
                        .bind(attempts)
                        .bind(row.id)
                        .execute(db)
                        .await?;
                        crate::audit!(
                            warn,
                            "chat.delivery",
                            "failure",
                            provider = self.inner.kind.as_str(),
                            kind = %row.kind,
                            attempts = attempts,
                            error = %e,
                            "聊天消息多次重试后仍未送达，已放弃"
                        );
                    } else {
                        let next = (Utc::now() + backoff(attempts)).to_rfc3339();
                        sqlx::query(
                            "UPDATE chat_outbox SET attempts = ?, next_attempt_at = ? WHERE id = ?",
                        )
                        .bind(attempts)
                        .bind(&next)
                        .bind(row.id)
                        .execute(db)
                        .await?;
                        tracing::warn!(error = %e, attempts, "聊天：投递失败，稍后重试");
                    }
                    // 提供方整体不可达时别逐条硬撞，等下一轮。
                    break;
                }
            }
        }
        Ok(())
    }

    /// 每日提醒：有申请已 pending 超过 24 小时时，每位管理员 24 小时内最多收到一条摘要。
    pub(super) async fn remind_if_due(&self) {
        if let Err(e) = self.try_remind().await {
            tracing::warn!(error = %e, "聊天：每日提醒出错");
        }
    }

    async fn try_remind(&self) -> AppResult<()> {
        let db = &self.inner.db;
        let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
        let (stale,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM users WHERE approval_status = 'pending' AND created_at < ?",
        )
        .bind(&cutoff)
        .fetch_one(db)
        .await?;
        if stale == 0 {
            return Ok(());
        }
        let text = self.inner.texts.reminder(stale as usize);
        for admin in self.identities().await? {
            let (recent,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM chat_outbox \
                 WHERE identity_id = ? AND kind = 'reminder' AND created_at > ?",
            )
            .bind(&admin.id)
            .bind(&cutoff)
            .fetch_one(db)
            .await?;
            if recent == 0 {
                self.insert_row(&admin.id, "reminder", None, &text, None, "queued")
                    .await?;
                self.inner.wake.notify_one();
            }
        }
        Ok(())
    }

    /// 「✅ 已由 X 于 T 批准」；账户仍 pending 则返回 None。
    pub(super) async fn decided_line(&self, user: &User) -> Option<String> {
        if user.approval_status == APPROVAL_PENDING {
            return None;
        }
        let by = user.approval_decided_by.as_deref().unwrap_or("?");
        let who = match by.strip_prefix("chat:").and_then(|r| r.split_once(':')) {
            Some((provider, sender)) => sqlx::query_as::<_, (String,)>(
                "SELECT label FROM admin_chat_identities WHERE provider = ? AND sender_id = ?",
            )
            .bind(provider)
            .bind(sender)
            .fetch_optional(&self.inner.db)
            .await
            .ok()
            .flatten()
            .map(|(l,)| l)
            .unwrap_or_else(|| by.to_string()),
            None => by.to_string(),
        };
        let time = fmt_time(user.approval_decided_at.as_deref().unwrap_or(""));
        Some(self.inner.texts.decided(
            user.approval_status == APPROVAL_APPROVED,
            &clean_name(&who),
            &time,
        ))
    }

    /// 账户已被决定：把它所有已发出的通知改成结果文字并移除按钮。
    /// 先用一条带条件的 UPDATE 认领这些行，并发调用时每条消息只会被编辑一次。
    pub(super) async fn refresh_notifications(&self, user: &User) {
        let Some(provider) = &self.inner.provider else {
            return;
        };
        let Some(line) = self.decided_line(user).await else {
            return;
        };
        let claimed: Result<Vec<(String, Option<String>)>, _> = sqlx::query_as(
            "UPDATE chat_outbox SET state = 'decided' \
             WHERE user_id = ? AND kind = 'notify' AND state = 'sent' \
             RETURNING text, sent_ref",
        )
        .bind(&user.id)
        .fetch_all(&self.inner.db)
        .await;
        let rows = match claimed {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(error = %e, "聊天：读取已发通知失败");
                return;
            }
        };
        for (text, sent_ref) in rows {
            let Some(target) = sent_ref.and_then(|r| serde_json::from_str::<MessageRef>(&r).ok())
            else {
                continue;
            };
            let msg = Message::text(format!("{text}\n\n{line}"));
            if let Err(e) = provider.edit(&target, &msg).await {
                tracing::warn!(error = %e, "聊天：编辑原消息失败");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps_at_one_hour() {
        assert_eq!(backoff(1), Duration::seconds(30));
        assert_eq!(backoff(2), Duration::seconds(60));
        assert_eq!(backoff(4), Duration::seconds(240));
        assert_eq!(backoff(30), Duration::seconds(3600));
    }
}
