use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

/// 文件类型。
pub const FILE_TYPE_FILE: &str = "file";
pub const FILE_TYPE_FOLDER: &str = "folder";

/// 数据库中的文件/资料夹行。
#[derive(Debug, Clone, FromRow)]
pub struct FileEntry {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub file_type: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
#[derive(Debug, Clone, Serialize, ToSchema)]
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

/// 缩略图查询参数。
#[derive(Debug, Deserialize, IntoParams)]
pub struct ThumbnailQuery {
    #[serde(default = "default_thumbnail_size")]
    pub size: u32,
}

fn default_thumbnail_size() -> u32 {
    256
}

/// 批量下载归档格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, ToSchema)]
pub enum DownloadFormat {
    #[default]
    #[serde(rename = "zip")]
    Zip,
    #[serde(rename = "tar.gz")]
    TarGz,
}

impl DownloadFormat {
    /// 归档文件扩展名（含前导点）。
    pub fn extension(self) -> &'static str {
        match self {
            DownloadFormat::Zip => ".zip",
            DownloadFormat::TarGz => ".tar.gz",
        }
    }

    /// 归档的 `Content-Type`。
    pub fn content_type(self) -> &'static str {
        match self {
            DownloadFormat::Zip => "application/zip",
            DownloadFormat::TarGz => "application/gzip",
        }
    }
}

/// 批量下载请求体：文件与资料夹 id 列表（资料夹会递归展开），可选归档格式（默认 ZIP）。
#[derive(Debug, Deserialize, ToSchema)]
pub struct DownloadRequest {
    pub ids: Vec<String>,
    #[serde(default)]
    pub format: DownloadFormat,
}

/// 文档专用：描述 `POST /files/upload` 的 multipart/form-data 请求体形状。
///
/// handler 实际使用 axum 的 `Multipart` 提取器手动解析字段，没有对应的数据结构，
/// 所以这里是不带字段的标记类型，schema 手写——
/// 若日后修改 `files::upload` 手动匹配的字段名（`file` / `name` / `parent_id`），需同步更新此处。
pub struct UploadForm;

impl utoipa::PartialSchema for UploadForm {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::{KnownFormat, ObjectBuilder, SchemaFormat, Type};
        let text = || ObjectBuilder::new().schema_type(Type::String);
        ObjectBuilder::new()
            .property(
                "file",
                text().format(Some(SchemaFormat::KnownFormat(KnownFormat::Binary))),
            )
            .required("file")
            .property("name", text())
            .property("parent_id", text())
            .into()
    }
}

impl ToSchema for UploadForm {}

#[cfg(test)]
mod tests {
    use super::*;
    use utoipa::PartialSchema;

    #[test]
    fn upload_form_schema_documents_multipart_fields() {
        let json = serde_json::to_value(UploadForm::schema()).unwrap();
        assert_eq!(json["properties"]["file"]["format"], "binary");
        assert_eq!(json["required"][0], "file");
        assert!(json["properties"]["name"].is_object());
        assert!(json["properties"]["parent_id"].is_object());
    }
}
