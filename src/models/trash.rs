use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;

/// 数据库中的回收站行。
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct TrashEntry {
    pub id: String,
    pub file_id: String,
    pub owner_id: String,
    pub original_parent_id: Option<String>,
    pub original_name: String,
    pub deleted_at: String,
    pub expires_at: String,
}

/// 回收站列表项（联接 files 取得类型与大小）。
#[derive(Debug, Clone, FromRow, Serialize, ToSchema)]
pub struct TrashItem {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub deleted_at: String,
    pub expires_at: String,
}

/// GET /trash 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct TrashListResponse {
    pub items: Vec<TrashItem>,
}

/// DELETE /trash/:id 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct FreedBytesResponse {
    pub freed_bytes: i64,
}

/// DELETE /trash 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct TrashEmptyResponse {
    pub deleted_count: i64,
    pub freed_bytes: i64,
}
