//! 实时存储事件（SSE `files` 频道负载）。

use serde::Serialize;
use utoipa::ToSchema;

use crate::models::file::FileResponse;

/// 文件/资料夹变更类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Created,
    Updated,
    Deleted,
    Restored,
    Purged,
}

/// SSE `files` 事件负载：告知前端哪个目录的列表已过期，以及具体发生了什么变化。
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileChangeEvent {
    pub kind: ChangeKind,
    /// 批量操作（清空回收站、定时清理）时为 null。
    pub file: Option<FileResponse>,
    /// listing 已失效的目录 id；数组中的 `null` 表示根目录，空数组表示范围未知（前端应刷新当前视图）。
    pub parent_ids: Vec<Option<String>>,
}

impl FileChangeEvent {
    /// 范围未知的失效事件（跨账户批量操作后使用，例如回收站定时清理）。
    pub fn refresh_all() -> Self {
        Self {
            kind: ChangeKind::Purged,
            file: None,
            parent_ids: Vec::new(),
        }
    }
}
