//! 图片缩略图生成与磁盘缓存。

use std::path::Path;

use image::ImageReader;
use image::imageops::FilterType;

use crate::drive::path_resolver;
use crate::error::{AppError, AppResult};
use crate::models::file::FileEntry;

/// 允许的缩略图尺寸（最长边像素），限制枚举值以防止任意 size 参数导致缓存膨胀。
pub const ALLOWED_SIZES: &[u32] = &[64, 128, 256, 512, 1024];

/// 判断某个 `mime_type` 是否属于图片，仅图片才可生成缩略图。
pub fn is_image(mime: &Option<String>) -> bool {
    mime.as_deref()
        .map(|m| m.starts_with("image/"))
        .unwrap_or(false)
}

/// 生成（或读取磁盘缓存的）缩略图，返回 JPEG 字节。
pub async fn get_or_create(
    data_root: &Path,
    owner_id: &str,
    entry: &FileEntry,
    size: u32,
) -> AppResult<Vec<u8>> {
    if !is_image(&entry.mime_type) {
        return Err(AppError::BadRequest("仅支持图片文件生成缩略图".into()));
    }
    if !ALLOWED_SIZES.contains(&size) {
        return Err(AppError::BadRequest(format!(
            "size 必须是以下之一: {ALLOWED_SIZES:?}"
        )));
    }

    let cache_path = path_resolver::thumbnail_path(data_root, owner_id, &entry.id, size);
    if let Ok(bytes) = tokio::fs::read(&cache_path).await {
        return Ok(bytes);
    }

    let source_path = path_resolver::file_path(data_root, owner_id, &entry.id);

    tokio::task::spawn_blocking(move || generate(&source_path, &cache_path, size))
        .await
        .map_err(|e| AppError::Other(anyhow::anyhow!("缩略图任务失败: {e}")))?
}

/// 阻塞式解码 + 缩放 + 编码 + 写入缓存，供 `spawn_blocking` 调用。
fn generate(source_path: &Path, cache_path: &Path, size: u32) -> AppResult<Vec<u8>> {
    // 物理文件名为不带扩展名的 UUID，需按内容猜测格式而非按扩展名。
    let img = ImageReader::open(source_path)
        .map_err(|_| AppError::NotFound)?
        .with_guessed_format()
        .map_err(|e| AppError::Other(anyhow::anyhow!("读取图片失败: {e}")))?
        .decode()
        .map_err(|e| AppError::BadRequest(format!("无法解码图片: {e}")))?;

    let thumb = img.resize(size, size, FilterType::Lanczos3);

    let mut buf = Vec::new();
    thumb
        .write_to(
            &mut std::io::Cursor::new(&mut buf),
            image::ImageFormat::Jpeg,
        )
        .map_err(|e| AppError::Other(anyhow::anyhow!("缩略图编码失败: {e}")))?;

    if let Some(parent) = cache_path.parent() {
        // 缓存目录创建失败不影响本次响应，仅跳过缓存写入。
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(cache_path, &buf);

    Ok(buf)
}
