//! 启动守卫：新版本若反复起不来，自动回滚二进制和数据库。
//!
//! 安装器把 `pending = 新版本` 写进 `update/state.json` 再重启。新版本每次启动先在这里
//! 把 `attempts` 加 1；超过 [`MAX_ATTEMPTS`] 次仍没被「提交」就回滚。提交发生在服务稳定
//! 运行 [`COMMIT_AFTER`] 之后（见 [`commit_after_health`]）。

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// 新版本最多允许启动几次而不被提交。
pub const MAX_ATTEMPTS: u32 = 3;
/// 服务持续运行多久后视为健康，提交本次更新。
pub const COMMIT_AFTER: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateState {
    /// 已安装但尚未被提交的版本。
    #[serde(default)]
    pub pending: Option<String>,
    /// `pending` 已启动的次数。
    #[serde(default)]
    pub attempts: u32,
    /// 更新前的数据库备份（回滚时还原）。
    #[serde(default)]
    pub backup: Option<PathBuf>,
    /// 曾回滚过的版本——检查器不再重复安装它，避免「装→崩→回滚」死循环。
    #[serde(default)]
    pub last_failed: Option<String>,
}

impl UpdateState {
    fn file(root: &Path) -> PathBuf {
        root.join("update").join("state.json")
    }

    /// 文件缺失视为空状态；损坏也按空处理（宁可少一次回滚，也不因坏文件起不来）。
    pub fn load(root: &Path) -> Self {
        std::fs::read(Self::file(root))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// 原子写：先写临时文件再 rename，断电不会留下半截 JSON。
    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let path = Self::file(root);
        std::fs::create_dir_all(path.parent().expect("has parent"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Proceed,
    /// 新版本已超过启动次数上限，需要回滚。
    Rollback {
        failed: String,
    },
}

/// 纯状态转移（不碰磁盘，便于测试）。`my_version` 是当前运行的版本。
pub fn decide(state: &mut UpdateState, my_version: &str) -> Decision {
    match state.pending.clone() {
        Some(p) if p == my_version => {
            state.attempts += 1;
            if state.attempts > MAX_ATTEMPTS {
                Decision::Rollback { failed: p }
            } else {
                Decision::Proceed
            }
        }
        Some(_) => {
            // pending 指向别的版本：说明已被回滚或人为换过版本，这条记录过期了。
            state.pending = None;
            state.attempts = 0;
            Decision::Proceed
        }
        None => Decision::Proceed,
    }
}

/// 原子地把 `root/<name>` 指向 `target`（相对 root 的路径）：先建临时链接再 rename。
#[cfg(unix)]
pub fn set_link(root: &Path, name: &str, target: &str) -> anyhow::Result<()> {
    let tmp = root.join(format!(".{name}.tmp"));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp)?;
    std::fs::rename(&tmp, root.join(name))?;
    Ok(())
}

#[cfg(not(unix))]
pub fn set_link(_root: &Path, _name: &str, _target: &str) -> anyhow::Result<()> {
    anyhow::bail!("在线更新仅支持 Unix 系统")
}

/// 回滚：`current` 指回 `previous`，还原数据库备份，记下失败版本。
pub fn rollback(root: &Path, db_path: &Path, state: &mut UpdateState) -> anyhow::Result<()> {
    let previous = std::fs::read_link(root.join("previous")).context("没有 previous 版本可回滚")?;
    let previous = previous
        .to_str()
        .context("previous 链接目标无效")?
        .to_string();

    if let Some(backup) = state.backup.as_ref().filter(|b| b.exists()) {
        let tmp = db_path.with_extension("restore.tmp");
        std::fs::copy(backup, &tmp).context("复制数据库备份失败")?;
        std::fs::rename(&tmp, db_path).context("还原数据库失败")?;
        // WAL/SHM 属于被替换前的库，留着会让 SQLite 把旧日志重放到备份上。
        for suffix in ["-wal", "-shm"] {
            let mut side = db_path.as_os_str().to_owned();
            side.push(suffix);
            let _ = std::fs::remove_file(PathBuf::from(side));
        }
    }

    set_link(root, "current", &previous)?;
    state.last_failed = state.pending.take();
    state.attempts = 0;
    state.save(root)?;
    Ok(())
}

/// `main` 里在数据库初始化**之前**调用。返回 `Ok(())` 表示继续启动；
/// 若触发了回滚则直接退出进程，由 systemd/launchd 拉起旧版本。
pub fn run(root: &Path, enabled: bool, my_version: &str, db_path: &Path) -> anyhow::Result<()> {
    if !enabled {
        return Ok(());
    }
    let mut state = UpdateState::load(root);
    let before = state.clone();
    let decision = decide(&mut state, my_version);

    match decision {
        Decision::Proceed => {
            if state != before {
                state.save(root)?;
            }
            if state.pending.is_some() {
                tracing::info!(
                    version = my_version,
                    attempts = state.attempts,
                    "新版本试运行中，等待稳定后提交"
                );
            }
            Ok(())
        }
        Decision::Rollback { failed } => {
            crate::audit!(error, "update.rollback", "started", failed_version = %failed, "新版本多次启动未能稳定，回滚");
            match rollback(root, db_path, &mut state) {
                Ok(()) => {
                    crate::audit!(warn, "update.rollback", "success", failed_version = %failed, "已回滚，退出以便守护进程拉起旧版本");
                    std::process::exit(1);
                }
                Err(e) => {
                    // 回滚本身失败：继续启动比反复崩溃更好——至少服务可能还能用。
                    crate::audit!(error, "update.rollback", "failure", error = %e, "回滚失败，继续尝试启动当前版本");
                    Ok(())
                }
            }
        }
    }
}

/// 服务稳定运行 [`COMMIT_AFTER`] 后清除 `pending`，更新正式生效。
pub async fn commit_after_health(root: PathBuf, my_version: String) {
    tokio::time::sleep(COMMIT_AFTER).await;
    let mut state = UpdateState::load(&root);
    if state.pending.as_deref() == Some(my_version.as_str()) {
        state.pending = None;
        state.attempts = 0;
        match state.save(&root) {
            Ok(()) => {
                crate::audit!(info, "update.commit", "success", version = %my_version, "更新已提交")
            }
            Err(e) => tracing::error!(error = %e, "提交更新状态失败"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending(v: &str, attempts: u32) -> UpdateState {
        UpdateState {
            pending: Some(v.into()),
            attempts,
            ..Default::default()
        }
    }

    #[test]
    fn first_boots_proceed_then_rollback_after_limit() {
        let mut s = pending("0.4.0", 0);
        for expected in 1..=MAX_ATTEMPTS {
            assert_eq!(decide(&mut s, "0.4.0"), Decision::Proceed);
            assert_eq!(s.attempts, expected);
        }
        assert_eq!(
            decide(&mut s, "0.4.0"),
            Decision::Rollback {
                failed: "0.4.0".into()
            }
        );
    }

    #[test]
    fn no_pending_is_noop() {
        let mut s = UpdateState::default();
        assert_eq!(decide(&mut s, "0.3.0"), Decision::Proceed);
        assert_eq!(s, UpdateState::default());
    }

    #[test]
    fn stale_pending_for_other_version_is_cleared() {
        let mut s = pending("0.4.0", 2);
        assert_eq!(decide(&mut s, "0.3.0"), Decision::Proceed);
        assert_eq!(s.pending, None);
        assert_eq!(s.attempts, 0);
    }

    fn tmp_root() -> PathBuf {
        let p = std::env::temp_dir().join(format!("ch-update-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(p.join("versions/0.3.0")).unwrap();
        std::fs::create_dir_all(p.join("versions/0.4.0")).unwrap();
        p
    }

    #[cfg(unix)]
    #[test]
    fn rollback_repoints_current_and_restores_db() {
        let root = tmp_root();
        set_link(&root, "current", "versions/0.4.0").unwrap();
        set_link(&root, "previous", "versions/0.3.0").unwrap();

        let db = root.join("cloud_home.sqlite");
        std::fs::write(&db, b"NEW-SCHEMA").unwrap();
        std::fs::write(root.join("cloud_home.sqlite-wal"), b"wal").unwrap();
        let backup = root.join("backup.sqlite");
        std::fs::write(&backup, b"OLD-SCHEMA").unwrap();

        let mut s = UpdateState {
            pending: Some("0.4.0".into()),
            attempts: 4,
            backup: Some(backup),
            last_failed: None,
        };
        rollback(&root, &db, &mut s).unwrap();

        assert_eq!(
            std::fs::read_link(root.join("current"))
                .unwrap()
                .to_str()
                .unwrap(),
            "versions/0.3.0"
        );
        assert_eq!(std::fs::read(&db).unwrap(), b"OLD-SCHEMA");
        assert!(!root.join("cloud_home.sqlite-wal").exists());
        assert_eq!(s.last_failed.as_deref(), Some("0.4.0"));
        assert_eq!(s.pending, None);
        // 状态已落盘
        assert_eq!(
            UpdateState::load(&root).last_failed.as_deref(),
            Some("0.4.0")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn rollback_without_previous_errors_and_changes_nothing() {
        let root = tmp_root();
        set_link(&root, "current", "versions/0.4.0").unwrap();
        let mut s = pending("0.4.0", 4);
        assert!(rollback(&root, &root.join("x.sqlite"), &mut s).is_err());
        assert_eq!(
            std::fs::read_link(root.join("current"))
                .unwrap()
                .to_str()
                .unwrap(),
            "versions/0.4.0"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn state_roundtrip_and_corrupt_file() {
        let root = tmp_root();
        let s = pending("1.0.0", 2);
        s.save(&root).unwrap();
        assert_eq!(UpdateState::load(&root), s);
        std::fs::write(root.join("update/state.json"), b"{not json").unwrap();
        assert_eq!(UpdateState::load(&root), UpdateState::default());
        let _ = std::fs::remove_dir_all(&root);
    }
}
