//! 安装步骤：下载校验、解包、自检、备份数据库、切换版本链接。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, bail};
use futures_lite::StreamExt;
use sha2::{Digest, Sha256};

use super::boot_guard::set_link;

/// 单个安装包的硬上限，防止清单里 size 被篡改成天文数字（清单已验签，这是纵深防御）。
const MAX_PACKAGE_BYTES: u64 = 512 * 1024 * 1024;
/// 下载时两个数据块之间允许的最长停顿。
const STALL_TIMEOUT: Duration = Duration::from_secs(60);
const SELF_TEST_TIMEOUT: Duration = Duration::from_secs(15);

/// 流式下载到 `dest`，边收边算 sha256；大小超限或哈希不符都会删掉文件并报错。
pub async fn download(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> anyhow::Result<()> {
    use tokio::io::AsyncWriteExt;

    let limit = expected_size.min(MAX_PACKAGE_BYTES);
    let resp = http.get(url).send().await?.error_for_status()?;
    if let Some(len) = resp.content_length()
        && len > limit
    {
        bail!("安装包大小 {len} 超过清单声明的 {limit}");
    }

    if let Some(dir) = dest.parent() {
        tokio::fs::create_dir_all(dir).await?;
    }
    let mut file = tokio::fs::File::create(dest).await?;
    let mut hasher = Sha256::new();
    let mut total: u64 = 0;
    let mut stream = Box::pin(resp.bytes_stream());

    let result: anyhow::Result<()> = async {
        loop {
            let chunk = match tokio::time::timeout(STALL_TIMEOUT, stream.next()).await {
                Err(_) => bail!("下载停滞超过 {} 秒", STALL_TIMEOUT.as_secs()),
                Ok(None) => break,
                Ok(Some(c)) => c?,
            };
            total += chunk.len() as u64;
            if total > limit {
                bail!("下载内容超过声明大小 {limit}");
            }
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
        }
        file.flush().await?;
        if total != expected_size {
            bail!("下载大小 {total} 与清单声明的 {expected_size} 不符");
        }
        let got = hex::encode(hasher.finalize());
        if !got.eq_ignore_ascii_case(expected_sha256) {
            bail!("sha256 不匹配：期望 {expected_sha256}，实际 {got}");
        }
        Ok(())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(dest).await;
    }
    result
}

/// 从 tar.gz 里只取 `cloud_home-<ver>-<target>/cloud_home` 一个文件到 `dest_dir/cloud_home`。
/// 其他条目一律忽略——不信任归档里的路径，不做通用解包，因而没有路径穿越面。
pub fn extract(tarball: &Path, version: &str, target: &str, dest_dir: &Path) -> anyhow::Result<()> {
    let wanted = PathBuf::from(format!("cloud_home-{version}-{target}")).join("cloud_home");
    let gz = flate2::read::GzDecoder::new(std::fs::File::open(tarball)?);
    let mut archive = tar::Archive::new(gz);

    std::fs::create_dir_all(dest_dir)?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() || entry.path()? != wanted {
            continue;
        }
        let out = dest_dir.join("cloud_home");
        let mut f = std::fs::File::create(&out)?;
        std::io::copy(&mut entry.by_ref().take(MAX_PACKAGE_BYTES * 4), &mut f)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(0o755))?;
        }
        return Ok(());
    }
    bail!("安装包里没有 {}", wanted.display())
}

/// 运行新二进制的 `--self-test`：确认它能在本机执行，且报告的版本与预期一致。
pub async fn self_test(binary: &Path, expected_version: &str) -> anyhow::Result<()> {
    let mut cmd = tokio::process::Command::new(binary);
    cmd.arg("--self-test").kill_on_drop(true);
    let out = tokio::time::timeout(SELF_TEST_TIMEOUT, cmd.output())
        .await
        .context("自检超时")?
        .context("无法执行新版本二进制")?;
    if !out.status.success() {
        bail!("自检失败，退出状态 {}", out.status);
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    if !stdout
        .trim()
        .starts_with(&format!("ok {expected_version} "))
    {
        bail!("自检输出与预期版本不符：{}", stdout.trim());
    }
    Ok(())
}

/// 用 SQLite 的 `VACUUM INTO` 做一致性在线备份，只保留最近 2 份。
pub async fn backup_db(
    pool: &crate::db::Db,
    data_root: &Path,
    from_version: &str,
) -> anyhow::Result<PathBuf> {
    let dir = data_root.join("backups");
    tokio::fs::create_dir_all(&dir).await?;
    let dest = dir.join(format!("db-before-update-from-{from_version}.sqlite"));
    let _ = tokio::fs::remove_file(&dest).await; // VACUUM INTO 要求目标不存在
    sqlx::query("VACUUM INTO ?")
        .bind(dest.display().to_string())
        .execute(pool)
        .await
        .context("数据库备份失败")?;
    prune_backups(&dir, 2);
    Ok(dest)
}

fn prune_backups(dir: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = rd
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("db-before-update-from-")
        })
        .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
        .collect();
    files.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
    for (_, p) in files.into_iter().skip(keep) {
        let _ = std::fs::remove_file(p);
    }
}

/// 把 `previous` 指向当前版本、`current` 指向新版本（各自原子替换），然后清理旧版本目录。
pub fn activate(root: &Path, new_version: &str) -> anyhow::Result<()> {
    if let Ok(cur) = std::fs::read_link(root.join("current")) {
        let cur = cur.to_str().context("current 链接目标无效")?.to_string();
        set_link(root, "previous", &cur)?;
    }
    set_link(root, "current", &format!("versions/{new_version}"))?;
    prune_versions(root);
    Ok(())
}

/// 只保留 current 与 previous 指向的版本目录。
fn prune_versions(root: &Path) {
    let keep: Vec<_> = ["current", "previous"]
        .iter()
        .filter_map(|l| std::fs::read_link(root.join(l)).ok())
        .filter_map(|p| p.file_name().map(|n| n.to_owned()))
        .collect();
    let Ok(rd) = std::fs::read_dir(root.join("versions")) else {
        return;
    };
    for e in rd.filter_map(|e| e.ok()) {
        if !keep.contains(&e.file_name()) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let p = std::env::temp_dir().join(format!("ch-inst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_tarball(path: &Path, entries: &[(&str, &[u8])]) {
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(path).unwrap(),
            flate2::Compression::fast(),
        );
        let mut b = tar::Builder::new(gz);
        for (name, data) in entries {
            let mut h = tar::Header::new_gnu();
            h.set_size(data.len() as u64);
            h.set_mode(0o644);
            // 直接写原始名字：Builder::set_path 会拒绝 `..`，而恶意归档恰恰可以带它。
            h.as_old_mut().name[..name.len()].copy_from_slice(name.as_bytes());
            h.set_cksum();
            b.append(&h, *data).unwrap();
        }
        b.into_inner().unwrap().finish().unwrap();
    }

    #[test]
    fn extract_picks_only_the_binary() {
        let d = tmp();
        let tb = d.join("p.tar.gz");
        make_tarball(
            &tb,
            &[
                ("cloud_home-1.0.0-t/README", b"nope"),
                ("cloud_home-1.0.0-t/cloud_home", b"BINARY"),
            ],
        );
        let out = d.join("out");
        extract(&tb, "1.0.0", "t", &out).unwrap();
        assert_eq!(std::fs::read(out.join("cloud_home")).unwrap(), b"BINARY");
        assert_eq!(std::fs::read_dir(&out).unwrap().count(), 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(out.join("cloud_home"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o111,
                0o111
            );
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn extract_ignores_traversal_and_wrong_layout() {
        let d = tmp();
        let tb = d.join("p.tar.gz");
        make_tarball(&tb, &[("../evil", b"x"), ("other/cloud_home", b"x")]);
        assert!(extract(&tb, "1.0.0", "t", &d.join("out")).is_err());
        assert!(!d.join("evil").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[cfg(unix)]
    #[test]
    fn activate_swaps_links_and_prunes() {
        let root = tmp();
        for v in ["0.1.0", "0.2.0", "0.3.0"] {
            std::fs::create_dir_all(root.join("versions").join(v)).unwrap();
        }
        set_link(&root, "current", "versions/0.2.0").unwrap();
        set_link(&root, "previous", "versions/0.1.0").unwrap();

        activate(&root, "0.3.0").unwrap();

        let rl = |n: &str| {
            std::fs::read_link(root.join(n))
                .unwrap()
                .to_str()
                .unwrap()
                .to_string()
        };
        assert_eq!(rl("current"), "versions/0.3.0");
        assert_eq!(rl("previous"), "versions/0.2.0");
        assert!(!root.join("versions/0.1.0").exists());
        assert!(root.join("versions/0.2.0").exists() && root.join("versions/0.3.0").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn self_test_checks_version_and_status() {
        use std::os::unix::fs::PermissionsExt;
        let d = tmp();
        let ok = d.join("ok.sh");
        std::fs::write(&ok, "#!/bin/sh\necho ok 1.2.3 some-target\n").unwrap();
        let bad = d.join("bad.sh");
        std::fs::write(&bad, "#!/bin/sh\nexit 3\n").unwrap();
        for p in [&ok, &bad] {
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        self_test(&ok, "1.2.3").await.unwrap();
        assert!(self_test(&ok, "9.9.9").await.is_err());
        assert!(self_test(&bad, "1.2.3").await.is_err());
        assert!(self_test(&d.join("missing"), "1.2.3").await.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn prune_backups_keeps_two_newest() {
        let d = tmp();
        for i in 0..4 {
            std::fs::write(
                d.join(format!("db-before-update-from-0.{i}.0.sqlite")),
                b"x",
            )
            .unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        std::fs::write(d.join("unrelated.txt"), b"x").unwrap();
        prune_backups(&d, 2);
        let mut names: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "db-before-update-from-0.2.0.sqlite",
                "db-before-update-from-0.3.0.sqlite",
                "unrelated.txt"
            ]
        );
        let _ = std::fs::remove_dir_all(&d);
    }
}
