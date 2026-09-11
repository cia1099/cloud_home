//! 空间占用相关的响应模型。

use serde::Serialize;
use utoipa::ToSchema;

/// 单个账户的空间占用快照。
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
pub struct UsageResponse {
    /// 有效文件占用字节数（不含已软删除）。
    pub used_bytes: i64,
    /// 回收站中仍占用磁盘的字节数。
    pub trashed_bytes: i64,
    /// 有效文件数量。
    pub file_count: i64,
    /// 数据盘总容量（字节）。
    pub volume_total_bytes: i64,
    /// 数据盘可用空间（字节）。
    pub volume_available_bytes: i64,
    /// 数据盘当前是否可用（未插入/未挂载时为 false，此时以上各字段均为 0）。
    pub drive_available: bool,
}

/// 单个账户在整盘中的占比。
#[derive(Debug, Serialize, ToSchema)]
pub struct UserUsageItem {
    pub user_id: String,
    pub username: String,
    pub used_bytes: i64,
    pub trashed_bytes: i64,
    pub file_count: i64,
    /// (used_bytes + trashed_bytes) / volume_total_bytes * 100，保留两位小数。
    pub percent_of_volume: f64,
}

/// GET /usage/breakdown 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct UsageBreakdownResponse {
    pub items: Vec<UserUsageItem>,
    /// 所有账户占用字节数之和。
    pub accounted_bytes: i64,
    pub volume_total_bytes: i64,
    pub volume_available_bytes: i64,
    pub drive_available: bool,
}
