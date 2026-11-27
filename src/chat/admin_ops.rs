//! 聊天里的运维指令：`/users`、`/status`、`/restore`，以及需要确认的 `/delete`、`/purge`。
//!
//! 全部走与 `/admin/*` 端点相同的服务函数（`user_service` / `admin_service`），
//! 本模块只负责解析指令、确认流程与回复文案。
//!
//! 破坏性操作（删除账户、清空回收站）两步走：指令先只登记一条「待确认」并回一个短码，
//! 管理员再发 `/confirm <码>` 才执行。待确认绑定到发起的管理员 + 目标账户 + 操作，
//! 单次有效、两分钟过期、只存内存（重启即作废）；码不对会直接取消该待确认，不给猜第二次。

use std::time::{Duration, Instant};

use super::ChatService;
use super::pairing::{Identity, new_code, normalize};
use super::provider::Sender;
use super::text::{clean_name, show_identifier};
use crate::error::{AppError, AppResult};
use crate::models::user::{
    APPROVAL_APPROVED, APPROVAL_PENDING, APPROVAL_REJECTED, PROVIDER_PASSWORD, User, UserIdentity,
};
use crate::services::{admin_service, user_service};

/// 确认码长度与有效期。
const CONFIRM_CODE_LEN: usize = 6;
const CONFIRM_TTL: Duration = Duration::from_secs(120);
/// `/users` 一次最多列出的账户数。
const USERS_LIST_MAX: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
    Delete,
    Purge,
}

/// 一条等待确认的破坏性操作。
pub(super) struct Confirmation {
    action: Action,
    user_id: String,
    code: String,
    pub(super) expires: Instant,
}

/// 解析 `<标识> [email|phone]`。
fn parse_target(args: &str) -> Option<(&str, Option<&str>)> {
    let mut parts = args.split_whitespace();
    let identifier = parts.next()?;
    let provider = parts.next().filter(|p| matches!(*p, "email" | "phone"));
    Some((identifier, provider))
}

/// 账户展示用的标识：第一个非密码标识，按配置打码。
fn shown_identifier(ids: &[UserIdentity], mask: bool) -> String {
    ids.iter()
        .find(|i| i.provider != PROVIDER_PASSWORD)
        .map(|i| show_identifier(&i.provider, &i.identifier, mask))
        .unwrap_or_default()
}

impl ChatService {
    /// 操作者标记，写进审计与审批记录。
    fn actor(&self, from: &Sender) -> String {
        format!("chat:{}:{}", self.inner.kind.as_str(), from.id)
    }

    /// 把「缺参数 / 账户不存在 / 标识有歧义」翻成给管理员的话；其余错误原样上抛。
    async fn find_target(
        &self,
        admin: &Identity,
        args: &str,
        cmd: &str,
    ) -> AppResult<Option<User>> {
        let texts = self.inner.texts;
        let Some((identifier, provider)) = parse_target(args) else {
            self.reply(&admin.chat_id, texts.usage(cmd)).await;
            return Ok(None);
        };
        match user_service::find_any(&self.inner.db, identifier, provider).await {
            Ok(user) => Ok(Some(user)),
            Err(AppError::NotFound) => {
                self.reply(&admin.chat_id, texts.account_not_found()).await;
                Ok(None)
            }
            Err(AppError::Conflict(_)) => {
                self.reply(&admin.chat_id, texts.account_ambiguous()).await;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    /// `/users [状态]`：列出账户（名称、打码标识、状态）。
    pub(super) async fn on_users(&self, admin: &Identity, args: &str) -> AppResult<()> {
        let texts = self.inner.texts;
        let status = match args.trim() {
            "" => None,
            s @ (APPROVAL_PENDING | APPROVAL_APPROVED | APPROVAL_REJECTED) => Some(s),
            _ => {
                self.reply(&admin.chat_id, texts.users_usage()).await;
                return Ok(());
            }
        };
        let rows = user_service::list_all(&self.inner.db, status).await?;
        if rows.is_empty() {
            self.reply(&admin.chat_id, texts.no_users()).await;
            return Ok(());
        }
        let shown = rows.len().min(USERS_LIST_MAX);
        let mut text = texts.users_header(shown, rows.len());
        for (u, ids) in rows.iter().take(USERS_LIST_MAX) {
            let tag = if u.deleted_at.is_some() {
                format!("{}, deleted", u.approval_status)
            } else {
                u.approval_status.clone()
            };
            text.push_str(&format!(
                "\n- {} | {} | {}",
                clean_name(&u.username),
                shown_identifier(ids, self.inner.mask_identifiers),
                tag
            ));
        }
        self.reply(&admin.chat_id, text).await;
        Ok(())
    }

    /// `/status`：版本、硬盘是否可用、账户概况。
    pub(super) async fn on_status(&self, admin: &Identity) -> AppResult<()> {
        let rows = user_service::list_all(&self.inner.db, None).await?;
        let deleted = rows.iter().filter(|(u, _)| u.deleted_at.is_some()).count();
        let count = |f: &dyn Fn(&User) -> bool| {
            rows.iter()
                .filter(|(u, _)| u.deleted_at.is_none() && f(u))
                .count()
        };
        let pending = count(&|u| u.approval_status == APPROVAL_PENDING);
        let rejected = count(&|u| u.approval_status == APPROVAL_REJECTED);
        let active = count(&|u| u.is_active != 0);
        let drive_ok = self.inner.env.drive_manager.is_available().await;
        let text = self.inner.texts.status_summary(
            crate::version::VERSION,
            drive_ok,
            active,
            pending,
            rejected,
            deleted,
        );
        self.reply(&admin.chat_id, text).await;
        Ok(())
    }

    /// `/restore`：不具破坏性，直接执行。
    pub(super) async fn on_restore(
        &self,
        admin: &Identity,
        from: &Sender,
        args: &str,
    ) -> AppResult<()> {
        let Some(target) = self.find_target(admin, args, "restore").await? else {
            return Ok(());
        };
        let texts = self.inner.texts;
        match admin_service::restore_account(&self.inner.db, &target, &self.actor(from)).await {
            Ok((user, n)) => {
                let name = clean_name(&user.username);
                self.reply(&admin.chat_id, texts.account_restored(&name, n))
                    .await;
            }
            Err(AppError::Conflict(_)) => self.reply(&admin.chat_id, texts.not_done()).await,
            Err(e) => return Err(e),
        }
        Ok(())
    }

    /// `/delete`、`/purge` 的第一步：只登记待确认并回一个短码，什么都不执行。
    pub(super) async fn on_request_destructive(
        &self,
        admin: &Identity,
        args: &str,
        action: Action,
    ) -> AppResult<()> {
        let cmd = match action {
            Action::Delete => "delete",
            Action::Purge => "purge",
        };
        let Some(target) = self.find_target(admin, args, cmd).await? else {
            return Ok(());
        };
        let ids = user_service::identities(&self.inner.db, &target.id).await?;
        let name = clean_name(&target.username);
        let ident = shown_identifier(&ids, self.inner.mask_identifiers);
        let code = new_code(CONFIRM_CODE_LEN);
        let minutes = CONFIRM_TTL.as_secs() / 60;
        let texts = self.inner.texts;
        let text = match action {
            Action::Delete => texts.confirm_delete(&name, &ident, &code, minutes),
            Action::Purge => texts.confirm_purge(&name, &ident, &code, minutes),
        };
        // 同一管理员只保留最新一条待确认。
        self.inner.confirmations.lock().unwrap().insert(
            admin.id.clone(),
            Confirmation {
                action,
                user_id: target.id,
                code,
                expires: Instant::now() + CONFIRM_TTL,
            },
        );
        self.reply(&admin.chat_id, text).await;
        Ok(())
    }

    /// `/cancel`：丢弃待确认。
    pub(super) async fn on_cancel(&self, admin: &Identity) {
        self.inner.confirmations.lock().unwrap().remove(&admin.id);
        self.reply(&admin.chat_id, self.inner.texts.cancelled())
            .await;
    }

    /// `/confirm <码>`：待确认被取出即作废（单次有效）；过期或码不对都什么也不执行。
    pub(super) async fn on_confirm(
        &self,
        admin: &Identity,
        from: &Sender,
        args: &str,
    ) -> AppResult<()> {
        let texts = self.inner.texts;
        let pending = self.inner.confirmations.lock().unwrap().remove(&admin.id);
        let Some(pending) = pending else {
            self.reply(&admin.chat_id, texts.nothing_to_confirm()).await;
            return Ok(());
        };
        if pending.expires <= Instant::now() {
            self.reply(&admin.chat_id, texts.confirmation_expired())
                .await;
            return Ok(());
        }
        if normalize(args) != pending.code {
            crate::audit!(
                warn,
                "chat.command",
                "invalid_confirmation",
                provider = self.inner.kind.as_str(),
                sender_id = %from.id,
                "聊天：确认码错误，待确认操作已取消"
            );
            self.reply(&admin.chat_id, texts.confirmation_wrong()).await;
            return Ok(());
        }

        let target = match user_service::get(&self.inner.db, &pending.user_id).await {
            Ok(u) => u,
            Err(AppError::NotFound) => {
                self.reply(&admin.chat_id, texts.account_not_found()).await;
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        let name = clean_name(&target.username);
        let actor = self.actor(from);
        let env = &self.inner.env;
        let outcome = match pending.action {
            Action::Delete => {
                admin_service::delete_account(
                    &self.inner.db,
                    env.trash_retention_days,
                    &target,
                    &actor,
                )
                .await
                .map(|(_, n)| texts.account_deleted(&name, n))
            }
            Action::Purge => admin_service::purge_trash(
                &self.inner.db,
                &env.drive_manager,
                &env.events,
                &target,
                &actor,
            )
            .await
            .map(|(n, bytes)| texts.trash_purged(&name, n, bytes)),
        };
        match outcome {
            Ok(text) => self.reply(&admin.chat_id, text).await,
            Err(AppError::Conflict(_)) => self.reply(&admin.chat_id, texts.not_done()).await,
            Err(AppError::DriveUnavailable) => {
                self.reply(&admin.chat_id, texts.drive_unavailable()).await
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }
}
