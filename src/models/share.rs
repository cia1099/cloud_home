use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

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
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateShareDto {
    pub file_id: String,
    #[serde(default = "default_can_download")]
    pub can_download: bool,
    #[serde(default)]
    pub expires_at: Option<String>,
    /// 非空时是站内定向分享（只有这些账户登录后可见，没有公开链接）；
    /// 省略或为空则是原来的公开链接。
    #[serde(default)]
    pub recipient_ids: Vec<String>,
}

/// 用户的最小公开身份：id 与显示名，不含任何联系方式。
#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct PersonRef {
    pub id: String,
    pub display_name: String,
}

fn default_can_download() -> bool {
    true
}

/// 对外返回的分享信息（含完整 URL）。
#[derive(Debug, Serialize, ToSchema)]
pub struct ShareResponse {
    pub id: String,
    pub file_id: String,
    /// 定向分享（`recipients` 非空）没有公开令牌，恒为 null。
    pub token: Option<String>,
    /// 公开链接地址；定向分享恒为 null。
    pub url: Option<String>,
    /// 定向分享的收件人；公开链接为空数组。
    pub recipients: Vec<PersonRef>,
    pub can_download: bool,
    pub expires_at: Option<String>,
    pub access_count: i64,
    pub created_at: String,
    pub is_active: bool,
}

impl ShareResponse {
    pub fn from_link(link: ShareLink, recipients: Vec<PersonRef>, public_base_url: &str) -> Self {
        let public = recipients.is_empty();
        let url = format!("{}/api/v1/public/shares/{}", public_base_url, link.token);
        Self {
            id: link.id,
            file_id: link.file_id,
            token: public.then_some(link.token),
            url: public.then_some(url),
            recipients,
            can_download: link.can_download != 0,
            expires_at: link.expires_at,
            access_count: link.access_count,
            created_at: link.created_at,
            is_active: link.is_active != 0,
        }
    }
}

/// 公开访问时返回的分享文件信息。
#[derive(Debug, Serialize, ToSchema)]
pub struct PublicShareResponse {
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub can_download: bool,
    pub shared_by: String,
}

/// GET /shares 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct ShareListResponse {
    pub items: Vec<ShareResponse>,
}

/// 修改定向分享的收件人（整组替换，不能清空——要停止分享请撤销）。
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateShareDto {
    pub recipient_ids: Vec<String>,
}

/// `GET /shares/received` 的一条数据库记录。
#[derive(Debug, FromRow)]
pub struct ReceivedShareRow {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub child_count: i64,
    pub owner_id: String,
    pub owner_name: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub can_download: i64,
    pub seen_at: Option<String>,
}

/// 收到的分享（「与我共享」列表项）。
#[derive(Debug, Serialize, ToSchema)]
pub struct ReceivedShare {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    /// 仅资料夹有值：直接子项数量。
    pub child_count: Option<i64>,
    pub owner: PersonRef,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub can_download: bool,
    /// 首次打开时间；null 表示未读。
    pub seen_at: Option<String>,
}

impl From<ReceivedShareRow> for ReceivedShare {
    fn from(r: ReceivedShareRow) -> Self {
        let is_folder = r.file_type == crate::models::file::FILE_TYPE_FOLDER;
        Self {
            id: r.id,
            file_id: r.file_id,
            name: r.name,
            file_type: r.file_type,
            size_bytes: r.size_bytes,
            mime_type: r.mime_type,
            child_count: is_folder.then_some(r.child_count),
            owner: PersonRef {
                id: r.owner_id,
                display_name: r.owner_name,
            },
            created_at: r.created_at,
            expires_at: r.expires_at,
            can_download: r.can_download != 0,
            seen_at: r.seen_at,
        }
    }
}

/// GET /shares/received 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct ReceivedShareListResponse {
    pub items: Vec<ReceivedShare>,
}

/// GET /users/directory 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct DirectoryResponse {
    pub items: Vec<PersonRef>,
}

/// 收件人浏览共享资料夹时的目录查询。
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ReceivedListQuery {
    /// 要列出的子资料夹 id；省略则列出被分享的资料夹本身。
    pub parent_id: Option<String>,
}
