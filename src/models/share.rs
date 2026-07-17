use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// 数据库中的分享链接行。
#[derive(Debug, Clone, FromRow)]
pub struct ShareLink {
    pub id: String,
    pub file_id: String,
    pub owner_id: String,
    pub token: String,
    pub can_download: i64,
    pub expires_at: Option<String>,
    pub access_count: i64,
    pub created_at: String,
    pub is_active: i64,
}

/// 创建分享请求体。
#[derive(Debug, Deserialize)]
pub struct CreateShareDto {
    pub file_id: String,
    #[serde(default = "default_can_download")]
    pub can_download: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
}

fn default_can_download() -> bool {
    true
}

/// 对外返回的分享信息（含完整 URL）。
#[derive(Debug, Serialize)]
pub struct ShareResponse {
    pub id: String,
    pub file_id: String,
    pub token: String,
    pub url: String,
    pub can_download: bool,
    pub expires_at: Option<String>,
    pub access_count: i64,
    pub created_at: String,
    pub is_active: bool,
}

impl ShareResponse {
    pub fn from_link(link: ShareLink, public_base_url: &str) -> Self {
        let url = format!("{}/api/v1/public/shares/{}", public_base_url, link.token);
        Self {
            id: link.id,
            file_id: link.file_id,
            token: link.token,
            url,
            can_download: link.can_download != 0,
            expires_at: link.expires_at,
            access_count: link.access_count,
            created_at: link.created_at,
            is_active: link.is_active != 0,
        }
    }
}

/// 公开访问时返回的分享文件信息。
#[derive(Debug, Serialize)]
pub struct PublicShareResponse {
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub can_download: bool,
    pub shared_by: String,
}
