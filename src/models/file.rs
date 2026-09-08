use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

/// 文件类型。
pub const FILE_TYPE_FILE: &str = "file";
pub const FILE_TYPE_FOLDER: &str = "folder";

/// 数据库中的文件/资料夹行。
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct FileEntry {
    pub id: String,
    pub owner_id: String,
    pub parent_id: Option<String>,
    pub drive_id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub physical_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: i64,
}

impl FileEntry {
    pub fn is_folder(&self) -> bool {
        self.file_type == FILE_TYPE_FOLDER
    }
}

/// 列目录查询参数。
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListQuery {
    pub parent_id: Option<String>,
}

/// 建立资料夹请求体。
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFolderDto {
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
}

/// 重命名请求体。
#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameDto {
    pub name: String,
}

/// 移动请求体。
#[derive(Debug, Deserialize, ToSchema)]
pub struct MoveDto {
    pub target_parent_id: Option<String>,
}

/// 对外返回的文件信息。
#[derive(Debug, Serialize, ToSchema)]
pub struct FileResponse {
    pub id: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<FileEntry> for FileResponse {
    fn from(f: FileEntry) -> Self {
        Self {
            id: f.id,
            name: f.name,
            file_type: f.file_type,
            size_bytes: f.size_bytes,
            mime_type: f.mime_type,
            parent_id: f.parent_id,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

/// GET /files 响应体。
#[derive(Debug, Serialize, ToSchema)]
pub struct FileListResponse {
    pub items: Vec<FileResponse>,
}

/// 文档专用：描述 `POST /files/upload` 的 multipart/form-data 请求体形状。
///
/// handler 实际使用 axum 的 `Multipart` 提取器手动解析字段，并不直接反序列化此结构体——
/// 若日后修改 `files::upload` 手动匹配的字段名（`file` / `name` / `parent_id`），需同步更新此处。
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadForm {
    #[schema(value_type = String, format = Binary)]
    pub file: Vec<u8>,
    pub name: Option<String>,
    pub parent_id: Option<String>,
}
