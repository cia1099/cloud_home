use serde::Serialize;
use sqlx::FromRow;

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
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct TrashItem {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub deleted_at: String,
    pub expires_at: String,
}
