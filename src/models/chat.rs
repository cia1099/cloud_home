//! 管理员聊天集成的 API 数据结构。

use serde::Serialize;
use utoipa::ToSchema;

/// 新签发的一次性配对码。
#[derive(Debug, Serialize, ToSchema)]
pub struct PairingResponse {
    /// 8 位配对码；管理员在聊天里发 `/pair <code>`。只显示这一次，库里只存哈希。
    pub code: String,
    pub expires_at: String,
}

/// 一位已配对的管理员。
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatAdminItem {
    pub id: String,
    pub label: String,
    pub paired_at: String,
}

/// 聊天集成状态。
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatStatusResponse {
    /// `none` | `telegram`。
    pub provider: String,
    /// 最近一次拉取更新是否成功（`provider = none` 时恒为 false）。
    pub connected: bool,
    pub admins: Vec<ChatAdminItem>,
}

/// 取消配对的结果。
#[derive(Debug, Serialize, ToSchema)]
pub struct ChatUnpairResponse {
    pub id: String,
    pub label: String,
}
