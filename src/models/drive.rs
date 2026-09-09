use serde::Serialize;
use utoipa::ToSchema;

/// 单个已侦测到的外接硬盘信息。
#[derive(Debug, Serialize, ToSchema)]
pub struct DriveInfo {
    pub id: String,
    pub mount_path: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub detected_at: String,
    pub last_seen_at: String,
}

/// GET /drives 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct DrivesListResponse {
    pub active_drive: Option<String>,
    pub is_available: bool,
    pub drives: Vec<DriveInfo>,
}
