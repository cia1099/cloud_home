use serde::Serialize;
use utoipa::ToSchema;

/// 通用确认响应。
#[derive(Debug, Serialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
}
