//! 处理入站更新：指令（`/start /help /pair /pending /whoami`）与审批按钮。
//!
//! 授权在**每条**更新上现查（`admin_chat_identities`），不是连接时一次性判定：
//! 取消配对立即生效。陌生发送者一律沉默（只审计，限流），机器人不向陌生人暴露自己。

use super::ChatService;
use super::outbox::PENDING_LIST_MAX;
use super::pairing::Identity;
use super::provider::{Message, Sender, Update};
use crate::error::{AppError, AppResult};
use crate::models::user::APPROVAL_PENDING;
use crate::services::approval_service::{self, Decision};
use crate::services::user_service;
use crate::util::now_rfc3339;

/// 未授权发送者每分钟最多写这么多条审计。
const UNAUTHORIZED_AUDIT_PER_MIN: usize = 3;

/// 审计里只记录已知的指令名，绝不记任意用户输入。
fn known_command(name: &str) -> &'static str {
    match name {
        "start" => "start",
        "help" => "help",
        "pair" => "pair",
        "pending" => "pending",
        "whoami" => "whoami",
        _ => "other",
    }
}

impl ChatService {
    /// 处理一条更新；出错只记日志，不影响轮询循环。
    pub(super) async fn handle_update(&self, update: Update) {
        if let Err(e) = self.try_handle(update).await {
            tracing::warn!(error = %e, "聊天：处理更新出错");
        }
    }

    async fn try_handle(&self, update: Update) -> AppResult<()> {
        let from = match &update {
            Update::Command { from, .. } | Update::ButtonPress { from, .. } => from,
        };
        let command = match &update {
            Update::Command { name, .. } => known_command(name),
            Update::ButtonPress { .. } => "button",
        };

        // 只处理与机器人的私聊；群聊里的一切都当陌生人。
        let admin = if from.private {
            self.identity_for(&from.id).await?
        } else {
            None
        };
        let Some(admin) = admin else {
            // 陌生人唯一可能成功的动作是带正确配对码的 /pair，其余（含失败的 /pair）沉默。
            if from.private
                && let Update::Command { name, args, .. } = &update
                && name == "pair"
                && let Some(new_admin) = self.redeem_pairing(from, args).await?
            {
                self.reply(&from.chat, self.inner.texts.paired(&new_admin.label))
                    .await;
                return Ok(());
            }
            self.audit_unauthorized(from, command);
            return Ok(());
        };

        crate::audit!(
            info,
            "chat.command",
            "authorized",
            provider = self.inner.kind.as_str(),
            sender_id = %from.id,
            command = command,
            "聊天指令（已授权）"
        );

        match &update {
            Update::Command { name, .. } => match name.as_str() {
                "pair" => {
                    self.reply(&from.chat, self.inner.texts.already_paired())
                        .await
                }
                "pending" => self.on_pending(&admin).await?,
                "whoami" => {
                    let text = self.inner.texts.whoami(
                        &admin.label,
                        self.inner.kind.as_str(),
                        &admin.sender_id,
                    );
                    self.reply(&from.chat, text).await;
                }
                // /start、/help 与任何未知指令都回帮助。
                _ => self.reply(&from.chat, self.inner.texts.help()).await,
            },
            Update::ButtonPress { token, ack, .. } => {
                self.on_button(&admin, from, token, ack).await?;
            }
        }
        Ok(())
    }

    fn audit_unauthorized(&self, from: &Sender, command: &str) {
        let allowed = {
            let mut rl = self.inner.unauthorized_audit.lock().unwrap();
            let ok = rl.allow(&from.id, UNAUTHORIZED_AUDIT_PER_MIN);
            if ok {
                rl.record(&from.id);
            }
            ok
        };
        if allowed {
            crate::audit!(
                warn,
                "chat.command",
                "unauthorized",
                provider = self.inner.kind.as_str(),
                sender_id = %from.id,
                private_chat = from.private,
                command = command,
                "聊天：未授权发送者被忽略"
            );
        }
    }

    /// 直接回复（指令回应不进队列；失败只记日志）。
    async fn reply(&self, chat: &str, text: String) {
        let Some(provider) = &self.inner.provider else {
            return;
        };
        if let Err(e) = provider.send(chat, &Message::text(text)).await {
            tracing::warn!(error = %e, "聊天：回复失败");
        }
    }

    /// `/pending`：给这位管理员重发每个待批准申请（带新的按钮令牌，旧消息过期也不怕）。
    async fn on_pending(&self, admin: &Identity) -> AppResult<()> {
        let pending = user_service::list_all(&self.inner.db, Some(APPROVAL_PENDING)).await?;
        if pending.is_empty() {
            self.reply(&admin.chat_id, self.inner.texts.no_pending())
                .await;
            return Ok(());
        }
        let shown = pending.len().min(PENDING_LIST_MAX);
        self.reply(
            &admin.chat_id,
            self.inner.texts.pending_header(shown, pending.len()),
        )
        .await;
        for (user, _) in pending.iter().take(PENDING_LIST_MAX) {
            self.enqueue_notify(admin, user, false).await?;
        }
        self.deliver_due().await;
        Ok(())
    }

    /// 审批按钮。校验顺序：授权（调用方已过）→ 令牌（存在、属于本管理员、未过期、未用，
    /// 一条 UPDATE 原子消费）→ 账户仍 pending → 走与管理端点相同的 `decide`。
    async fn on_button(
        &self,
        admin: &Identity,
        from: &Sender,
        token: &str,
        ack: &str,
    ) -> AppResult<()> {
        if let Some(provider) = &self.inner.provider {
            let _ = provider.ack_button(ack).await;
        }

        let now = now_rfc3339();
        let claimed: Option<(String, String)> = sqlx::query_as(
            "UPDATE chat_action_tokens SET used_at = ? \
             WHERE token_hash = ? AND identity_id = ? AND used_at IS NULL AND expires_at > ? \
             RETURNING user_id, action",
        )
        .bind(&now)
        .bind(super::digest(token))
        .bind(&admin.id)
        .bind(&now)
        .fetch_optional(&self.inner.db)
        .await?;
        let Some((user_id, action)) = claimed else {
            crate::audit!(
                warn,
                "chat.command",
                "invalid_token",
                provider = self.inner.kind.as_str(),
                sender_id = %from.id,
                "聊天：按钮令牌无效（已用 / 过期 / 不属于该管理员）"
            );
            self.reply(&from.chat, self.inner.texts.button_invalid())
                .await;
            return Ok(());
        };

        let user = user_service::get(&self.inner.db, &user_id).await?;
        let mut won = false;
        if user.approval_status == APPROVAL_PENDING {
            let decision = if action == "approve" {
                Decision::Approve
            } else {
                Decision::Reject
            };
            let decided_by = format!("chat:{}:{}", self.inner.kind.as_str(), from.id);
            match approval_service::decide(&self.inner.db, &user_id, decision, &decided_by, None)
                .await
            {
                Ok(d) => won = d.changed,
                // 并发的另一位管理员先到了：当作已被决定。
                Err(AppError::Conflict(_)) => {}
                Err(e) => return Err(e),
            }
        }

        let user = user_service::get(&self.inner.db, &user_id).await?;
        self.refresh_notifications(&user).await;
        if !won && let Some(line) = self.decided_line(&user).await {
            self.reply(&from.chat, self.inner.texts.already_decided(&line))
                .await;
        }
        Ok(())
    }
}
