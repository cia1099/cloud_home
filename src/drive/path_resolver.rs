//! 逻辑 file_id → 硬盘物理路径解析。
//!
//! 物理路径完全由服务端基于 UUID 生成，用户提供的 `name` 永不参与路径构造，
//! 以防路径遍历攻击。

use std::path::{Path, PathBuf};

/// SQLite 数据库文件名（存放在数据根目录下）。
pub const DB_FILE_NAME: &str = "cloud_home.db";

/// 数据根目录：`{mount_path}/{data_dir}`。
pub fn data_root(mount_path: &Path, data_dir: &str) -> PathBuf {
    mount_path.join(data_dir)
}

/// 数据库文件路径：`{data_root}/cloud_home.db`。
pub fn db_path(data_root: &Path) -> PathBuf {
    data_root.join(DB_FILE_NAME)
}

/// 某个文件的物理存储目录：`{data_root}/users/{user_id}/{uuid[0..2]}`。
pub fn file_dir(data_root: &Path, user_id: &str, file_id: &str) -> PathBuf {
    let shard = &file_id[..file_id.len().min(2)];
    data_root.join("users").join(user_id).join(shard)
}

/// 某个文件的完整物理路径：`{file_dir}/{file_id}`。
pub fn file_path(data_root: &Path, user_id: &str, file_id: &str) -> PathBuf {
    file_dir(data_root, user_id, file_id).join(file_id)
}
