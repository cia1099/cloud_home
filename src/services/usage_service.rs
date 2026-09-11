//! 空间占用统计：单账户占用、全账户占比，均结合 `statvfs` 汇报整盘容量。

use std::path::Path;

use crate::db::Db;
use crate::drive::DriveManager;
use crate::error::AppResult;
use crate::models::usage::{UsageBreakdownResponse, UsageResponse, UserUsageItem};

/// 单账户空间占用快照。硬盘不可用时返回全零快照（`drive_available = false`），不查询数据库
/// ——数据库文件本身就存放在该硬盘上，此时查询必然失败。
pub async fn snapshot(db: &Db, dm: &DriveManager, owner_id: &str) -> AppResult<UsageResponse> {
    let Some(mount) = dm.active_mount().await else {
        return Ok(zero_usage());
    };

    let (used_bytes, trashed_bytes, file_count) = fetch_owner_totals(db, owner_id).await?;
    let (volume_total_bytes, volume_available_bytes) = statvfs_bytes(&mount).await?;

    Ok(UsageResponse {
        used_bytes,
        trashed_bytes,
        file_count,
        volume_total_bytes,
        volume_available_bytes,
        drive_available: true,
    })
}

/// 全部账户在整盘中的占比。
pub async fn breakdown(db: &Db, dm: &DriveManager) -> AppResult<UsageBreakdownResponse> {
    let Some(mount) = dm.active_mount().await else {
        return Ok(UsageBreakdownResponse {
            items: Vec::new(),
            accounted_bytes: 0,
            volume_total_bytes: 0,
            volume_available_bytes: 0,
            drive_available: false,
        });
    };

    let rows: Vec<(String, String, i64, i64, i64)> = sqlx::query_as(
        "SELECT u.id, u.username, \
                COALESCE(SUM(CASE WHEN f.is_deleted = 0 THEN f.size_bytes ELSE 0 END), 0) AS used, \
                COALESCE(SUM(CASE WHEN f.is_deleted = 1 THEN f.size_bytes ELSE 0 END), 0) AS trashed, \
                COALESCE(SUM(CASE WHEN f.is_deleted = 0 THEN 1 ELSE 0 END), 0) AS file_count \
         FROM users u \
         LEFT JOIN files f ON f.owner_id = u.id AND f.file_type = 'file' \
         GROUP BY u.id, u.username \
         ORDER BY used DESC",
    )
    .fetch_all(db)
    .await?;

    let (volume_total_bytes, volume_available_bytes) = statvfs_bytes(&mount).await?;

    let mut accounted_bytes = 0i64;
    let items = rows
        .into_iter()
        .map(|(user_id, username, used_bytes, trashed_bytes, file_count)| {
            accounted_bytes += used_bytes + trashed_bytes;
            let percent_of_volume = if volume_total_bytes > 0 {
                ((used_bytes + trashed_bytes) as f64 / volume_total_bytes as f64 * 100.0
                    * 100.0)
                    .round()
                    / 100.0
            } else {
                0.0
            };
            UserUsageItem {
                user_id,
                username,
                used_bytes,
                trashed_bytes,
                file_count,
                percent_of_volume,
            }
        })
        .collect();

    Ok(UsageBreakdownResponse {
        items,
        accounted_bytes,
        volume_total_bytes,
        volume_available_bytes,
        drive_available: true,
    })
}

fn zero_usage() -> UsageResponse {
    UsageResponse {
        used_bytes: 0,
        trashed_bytes: 0,
        file_count: 0,
        volume_total_bytes: 0,
        volume_available_bytes: 0,
        drive_available: false,
    }
}

/// 有效文件占用字节数、回收站占用字节数、有效文件数量。
async fn fetch_owner_totals(db: &Db, owner_id: &str) -> AppResult<(i64, i64, i64)> {
    let row: (i64, i64, i64) = sqlx::query_as(
        "SELECT \
            COALESCE(SUM(CASE WHEN is_deleted = 0 THEN size_bytes ELSE 0 END), 0), \
            COALESCE(SUM(CASE WHEN is_deleted = 1 THEN size_bytes ELSE 0 END), 0), \
            COALESCE(SUM(CASE WHEN is_deleted = 0 THEN 1 ELSE 0 END), 0) \
         FROM files WHERE owner_id = ? AND file_type = 'file'",
    )
    .bind(owner_id)
    .fetch_one(db)
    .await?;
    Ok(row)
}

/// 挂载点所在文件系统的 (总容量, 可用空间)，单位字节。
///
/// `statvfs` 在硬盘刚被拔出但尚未被硬盘监控任务感知时可能阻塞，故置于 `spawn_blocking`。
async fn statvfs_bytes(mount: &Path) -> AppResult<(i64, i64)> {
    let mount = mount.to_path_buf();
    tokio::task::spawn_blocking(move || statvfs_bytes_blocking(&mount))
        .await
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!("statvfs 任务失败: {e}")))?
}

fn statvfs_bytes_blocking(mount: &Path) -> AppResult<(i64, i64)> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(mount.as_os_str().as_bytes())
        .map_err(|e| crate::error::AppError::Other(anyhow::anyhow!("挂载路径含非法字符: {e}")))?;

    // SAFETY: 传入合法的 C 字符串指针与已初始化的 statvfs 结构。
    let stat = unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
            return Err(crate::error::AppError::Other(anyhow::anyhow!(
                "statvfs 调用失败: {}",
                std::io::Error::last_os_error()
            )));
        }
        stat
    };

    let frsize = stat.f_frsize as i64;
    let total = frsize.saturating_mul(stat.f_blocks as i64);
    let available = frsize.saturating_mul(stat.f_bavail as i64);
    Ok((total, available))
}
