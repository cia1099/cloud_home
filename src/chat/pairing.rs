//! 配对：运维调私有端点拿到一次性配对码，管理员在聊天里发 `/pair <code>`，
//! 其提供方 id 才进入 `admin_chat_identities`。没有别的途径成为管理员。

use chrono::{Duration, Utc};

use super::provider::Sender;
use super::text::clean_name;
use super::{ChatService, digest};
use crate::error::{AppError, AppResult};
use crate::models::chat::{ChatUnpairResponse, PairingResponse};
use crate::util::now_rfc3339;

/// 配对码有效期（分钟）。
const CODE_TTL_MINUTES: i64 = 10;
/// 配对码字母表：去掉易混的 0/O/1/I，32 个字符，8 位约 40 bit。
const ALPHABET: &[u8; 32] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// 同一发送者 10 分钟内最多试错次数。
const MAX_FAILS_PER_SENDER: usize = 5;
/// 全部发送者合计 10 分钟内最多试错次数（防用大量账号分散猜码）。
const MAX_FAILS_GLOBAL: usize = 20;

/// 已配对的管理员。
#[derive(Debug, Clone, sqlx::FromRow)]
pub(super) struct Identity {
    pub id: String,
    pub sender_id: String,
    pub chat_id: String,
    pub label: String,
    pub paired_at: String,
}

/// 生成 `len` 位随机码（配对码 8 位；破坏性操作的确认码 6 位）。
pub(super) fn new_code(len: usize) -> String {
    (0..len)
        .map(|_| ALPHABET[(rand::random::<u8>() & 31) as usize] as char)
        .collect()
}

/// 用户输入的配对码规整：去空白与连字符、转大写。
pub(super) fn normalize(code: &str) -> String {
    code.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

impl ChatService {
    pub(super) async fn identities(&self) -> AppResult<Vec<Identity>> {
        Ok(sqlx::query_as::<_, Identity>(
            "SELECT id, sender_id, chat_id, label, paired_at FROM admin_chat_identities \
             WHERE provider = ? ORDER BY paired_at, id",
        )
        .bind(self.inner.kind.as_str())
        .fetch_all(&self.inner.db)
        .await?)
    }

    /// 授权检查：每条更新都现查表，取消配对立即生效。
    pub(super) async fn identity_for(&self, sender_id: &str) -> AppResult<Option<Identity>> {
        Ok(sqlx::query_as::<_, Identity>(
            "SELECT id, sender_id, chat_id, label, paired_at FROM admin_chat_identities \
             WHERE provider = ? AND sender_id = ?",
        )
        .bind(self.inner.kind.as_str())
        .bind(sender_id)
        .fetch_optional(&self.inner.db)
        .await?)
    }

    /// `POST /admin/chat/pairing`：签发一次性配对码（只存哈希）。
    pub async fn create_pairing_code(&self) -> AppResult<PairingResponse> {
        if !self.is_enabled() {
            return Err(AppError::Conflict(
                "聊天集成未启用（CHAT_PROVIDER=none）".into(),
            ));
        }
        let code = new_code(8);
        let now = now_rfc3339();
        let expires_at = (Utc::now() + Duration::minutes(CODE_TTL_MINUTES)).to_rfc3339();
        sqlx::query("DELETE FROM chat_pairing_codes WHERE expires_at < ? OR used_at IS NOT NULL")
            .bind(&now)
            .execute(&self.inner.db)
            .await?;
        sqlx::query("INSERT INTO chat_pairing_codes (code_hash, expires_at) VALUES (?, ?)")
            .bind(digest(&code))
            .bind(&expires_at)
            .execute(&self.inner.db)
            .await?;
        crate::audit!(
            info,
            "chat.pair",
            "code_issued",
            provider = self.inner.kind.as_str(),
            expires_at = %expires_at,
            "已签发聊天配对码"
        );
        Ok(PairingResponse { code, expires_at })
    }

    /// 处理 `/pair <code>`：成功返回新身份；码错 / 过期 / 已用 / 被限流都返回 `None`，
    /// 调用方对陌生人一律保持沉默，不暴露机器人存在。
    pub(super) async fn redeem_pairing(
        &self,
        from: &Sender,
        code: &str,
    ) -> AppResult<Option<Identity>> {
        {
            let mut rl = self.inner.pair_failures.lock().unwrap();
            if !rl.allow(&from.id, MAX_FAILS_PER_SENDER) || !rl.allow("*", MAX_FAILS_GLOBAL) {
                return Ok(None);
            }
        }

        let now = now_rfc3339();
        let mut tx = self.inner.db.begin().await?;
        let used = sqlx::query(
            "UPDATE chat_pairing_codes SET used_at = ? \
             WHERE code_hash = ? AND used_at IS NULL AND expires_at > ?",
        )
        .bind(&now)
        .bind(digest(&normalize(code)))
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        if used.rows_affected() != 1 {
            tx.rollback().await?;
            let mut rl = self.inner.pair_failures.lock().unwrap();
            rl.record(&from.id);
            rl.record("*");
            return Ok(None);
        }

        let label = match clean_name(&from.label) {
            l if l.is_empty() => from.id.clone(),
            l => l,
        };
        let identity = Identity {
            id: uuid::Uuid::new_v4().to_string(),
            sender_id: from.id.clone(),
            chat_id: from.chat.clone(),
            label,
            paired_at: now,
        };
        sqlx::query(
            "INSERT INTO admin_chat_identities (id, provider, sender_id, chat_id, label, paired_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&identity.id)
        .bind(self.inner.kind.as_str())
        .bind(&identity.sender_id)
        .bind(&identity.chat_id)
        .bind(&identity.label)
        .bind(&identity.paired_at)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        crate::audit!(
            info,
            "chat.pair",
            "paired",
            provider = self.inner.kind.as_str(),
            sender_id = %identity.sender_id,
            "管理员已配对"
        );
        Ok(Some(identity))
    }

    /// `DELETE /admin/chat/admins/{id}`：取消配对。授权每次现查表，所以立即生效；
    /// 该管理员尚未使用的按钮令牌与待发消息随外键级联删除。
    pub async fn unpair(&self, id: &str) -> AppResult<ChatUnpairResponse> {
        let removed: Option<(String, String)> = sqlx::query_as(
            "DELETE FROM admin_chat_identities WHERE id = ? AND provider = ? \
             RETURNING label, sender_id",
        )
        .bind(id)
        .bind(self.inner.kind.as_str())
        .fetch_optional(&self.inner.db)
        .await?;
        let Some((label, sender_id)) = removed else {
            return Err(AppError::NotFound);
        };
        crate::audit!(
            info,
            "chat.unpair",
            "success",
            provider = self.inner.kind.as_str(),
            sender_id = %sender_id,
            "管理员已取消配对"
        );
        Ok(ChatUnpairResponse {
            id: id.to_string(),
            label,
        })
    }
}
