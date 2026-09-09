//! 流式打包多个文件为归档（ZIP 或 tar.gz），用于批量下载。

use async_zip::tokio::write::ZipFileWriter;
use async_zip::{Compression as ZipCompression, ZipEntryBuilder};
use flate2::Compression as GzCompression;
use flate2::write::GzEncoder;
use tokio::io::DuplexStream;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_util::io::SyncIoBridge;

use crate::models::file::DownloadFormat;
use crate::services::file_service::DownloadEntry;

/// 将给定条目依序打包为归档，通过 `writer`（`tokio::io::duplex` 的一端）流式输出。
///
/// 在独立 task 内调用；写入过程中的任何 I/O 失败仅记录日志，不 panic
/// ——对端 `Body::from_stream` 会随流中断而自然结束响应。
pub async fn write_archive(format: DownloadFormat, writer: DuplexStream, entries: Vec<DownloadEntry>) {
    let result = match format {
        DownloadFormat::Zip => write_zip(writer, entries).await,
        DownloadFormat::TarGz => write_tar_gz(writer, entries).await,
    };
    if let Err(e) = result {
        tracing::warn!(error = %e, "归档打包失败");
    }
}

async fn write_zip(writer: DuplexStream, entries: Vec<DownloadEntry>) -> anyhow::Result<()> {
    let mut zip = ZipFileWriter::with_tokio(writer);

    for entry in entries {
        let compression = if is_precompressed(&entry.zip_path) {
            ZipCompression::Stored
        } else {
            ZipCompression::Deflate
        };
        // 显式设置 Unix 权限 0o644——不设置时默认落地为 0（extract 出来的文件不可读）。
        let builder = ZipEntryBuilder::new(entry.zip_path.clone().into(), compression)
            .unix_permissions(0o644);

        let file = tokio::fs::File::open(&entry.physical_path).await?;
        let mut entry_writer = zip.write_entry_stream(builder).await?;
        futures_lite::io::copy(file.compat(), &mut entry_writer).await?;
        entry_writer.close().await?;
    }

    zip.close().await?;
    Ok(())
}

/// tar.gz 打包：`tar`/`flate2` 均为同步 API，整体放入 `spawn_blocking`，
/// 通过 `SyncIoBridge` 把异步 `duplex` 一端桥接为同步 `Write`。
async fn write_tar_gz(writer: DuplexStream, entries: Vec<DownloadEntry>) -> anyhow::Result<()> {
    // 必须在异步上下文中构造（内部捕获当前 runtime 句柄），再整体移入 spawn_blocking。
    let sync_writer = SyncIoBridge::new(writer);

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let gz = GzEncoder::new(sync_writer, GzCompression::default());
        let mut builder = tar::Builder::new(gz);

        for entry in &entries {
            let mut file = std::fs::File::open(&entry.physical_path)?;
            let metadata = file.metadata()?;

            let mut header = tar::Header::new_gnu();
            header.set_size(metadata.len());
            header.set_mode(0o644);
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            header.set_mtime(mtime);

            builder.append_data(&mut header, &entry.zip_path, &mut file)?;
        }

        // `into_inner()` 会自动补齐 tar 结尾块；随后 `finish()` 落地 gzip trailer。
        let gz = builder.into_inner()?;
        gz.finish()?;
        Ok(())
    })
    .await??;

    Ok(())
}

/// 已是压缩格式的常见扩展名，ZIP 打包时用 `Stored`（不再压缩）以节省 CPU。
fn is_precompressed(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    const EXTS: &[&str] = &[
        ".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic", ".mp4", ".mov", ".m4a", ".mp3", ".zip",
        ".rar", ".7z",
    ];
    EXTS.iter().any(|ext| lower.ends_with(ext))
}
