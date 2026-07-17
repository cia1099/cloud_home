//! 跨平台外接硬盘侦测。
//!
//! macOS 扫描 `/Volumes/`，Linux 扫描 `/media/` 与 `/run/media/`。
//! 对外暴露统一接口 [`scan`] 和 [`watch_roots`]。

use std::path::{Path, PathBuf};

/// 硬盘上用于标记 Cloud Home 数据盘的文件名。
pub const DRIVE_MARKER: &str = ".cloud_home_marker";

/// 扫描当前系统上所有可用的外接硬盘挂载点。
///
/// 若多个硬盘同时挂载，含有 [`DRIVE_MARKER`] 的硬盘会排在最前。
pub fn scan(volumes_watch_path: &Path, data_dir: &str) -> Vec<PathBuf> {
    let mut drives = scan_platform(volumes_watch_path);

    // 优先使用已初始化过（含 marker 或数据目录）的硬盘。
    drives.sort_by_key(|p| {
        let has_marker = p.join(data_dir).join(DRIVE_MARKER).exists()
            || p.join(DRIVE_MARKER).exists();
        // false(0) 排在 true(1) 前，因此对 has_marker 取反。
        !has_marker
    });

    drives
}

/// 需要实时监控（挂载/卸载）的根目录列表。
pub fn watch_roots(volumes_watch_path: &Path) -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![volumes_watch_path.to_path_buf()]
    }
    #[cfg(target_os = "linux")]
    {
        let _ = volumes_watch_path;
        [PathBuf::from("/media"), PathBuf::from("/run/media")]
            .into_iter()
            .filter(|p| p.exists())
            .collect()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = volumes_watch_path;
        Vec::new()
    }
}

#[cfg(target_os = "macos")]
fn scan_platform(volumes_watch_path: &Path) -> Vec<PathBuf> {
    const EXCLUDED: [&str; 5] = ["Macintosh HD", "Recovery", "Preboot", "VM", "Update"];

    let Ok(entries) = std::fs::read_dir(volumes_watch_path) else {
        return Vec::new();
    };

    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !EXCLUDED.iter().any(|ex| name.contains(ex))
        })
        .filter(|p| is_local_mount(p))
        .collect()
}

#[cfg(target_os = "macos")]
fn is_local_mount(path: &Path) -> bool {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(c_path) = CString::new(path.as_os_str().as_bytes()) else {
        return false;
    };
    // SAFETY: 传入合法的 C 字符串指针与已初始化的 statfs 结构。
    unsafe {
        let mut stat: libc::statfs = std::mem::zeroed();
        if libc::statfs(c_path.as_ptr(), &mut stat) != 0 {
            return false;
        }
        (stat.f_flags & libc::MNT_LOCAL as u32) != 0
    }
}

#[cfg(target_os = "linux")]
fn scan_platform(_volumes_watch_path: &Path) -> Vec<PathBuf> {
    const ROOTS: [&str; 2] = ["/media", "/run/media"];
    let allowed = mounted_external_paths();

    let mut drives = Vec::new();
    for root in ROOTS {
        // 布局为 /media/<user>/<label> 或 /media/<label>，向下扫描两层。
        collect_mount_candidates(Path::new(root), 2, &allowed, &mut drives);
    }
    drives
}

/// 解析 `/proc/mounts`，返回外部存储文件系统类型对应的挂载点集合。
#[cfg(target_os = "linux")]
fn mounted_external_paths() -> std::collections::HashSet<PathBuf> {
    const ALLOWED_FS: [&str; 6] = ["vfat", "ntfs", "exfat", "ext4", "btrfs", "xfs"];

    let mut set = std::collections::HashSet::new();
    let Ok(content) = std::fs::read_to_string("/proc/mounts") else {
        return set;
    };
    for line in content.lines() {
        let mut cols = line.split_whitespace();
        let _device = cols.next();
        let Some(mount_point) = cols.next() else {
            continue;
        };
        let Some(fs_type) = cols.next() else {
            continue;
        };
        if ALLOWED_FS.contains(&fs_type) {
            // /proc/mounts 用八进制转义空格为 \040。
            set.insert(PathBuf::from(mount_point.replace("\\040", " ")));
        }
    }
    set
}

#[cfg(target_os = "linux")]
fn collect_mount_candidates(
    dir: &Path,
    depth: usize,
    allowed: &std::collections::HashSet<PathBuf>,
    out: &mut Vec<PathBuf>,
) {
    if allowed.contains(dir) {
        out.push(dir.to_path_buf());
        return;
    }
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            collect_mount_candidates(&path, depth - 1, allowed, out);
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn scan_platform(_volumes_watch_path: &Path) -> Vec<PathBuf> {
    Vec::new()
}
