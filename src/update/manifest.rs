//! 发布清单：验签、解析、判断是否有可装的新版本。

use std::collections::HashMap;

use anyhow::{Context, bail};
use semver::Version;
use serde::Deserialize;

/// 清单大小上限，防止被喂一个巨大响应。
pub const MAX_MANIFEST_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes_url: String,
    /// 低于此版本的设备不能直接升到本版，须先经过中间版本。
    #[serde(default)]
    pub min_from_version: Option<String>,
    pub targets: HashMap<String, TargetEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetEntry {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

/// 用公钥校验 `manifest.json` 的 minisign 签名。**必须先验签再解析。**
pub fn verify(public_key_b64: &str, manifest: &[u8], signature_text: &str) -> anyhow::Result<()> {
    let pk = minisign_verify::PublicKey::from_base64(public_key_b64)
        .map_err(|e| anyhow::anyhow!("内置公钥无效: {e}"))?;
    let sig = minisign_verify::Signature::decode(signature_text)
        .map_err(|e| anyhow::anyhow!("签名文件无法解析: {e}"))?;
    pk.verify(manifest, &sig, false)
        .map_err(|e| anyhow::anyhow!("清单签名校验失败: {e}"))
}

pub fn parse(bytes: &[u8]) -> anyhow::Result<Manifest> {
    serde_json::from_slice(bytes).context("清单 JSON 解析失败")
}

/// 结论：清单里有没有「严格新于当前」且适用于本机的版本。
#[derive(Debug, PartialEq, Eq)]
pub enum Evaluation {
    /// 已是最新（或清单版本不比当前新——绝不降级）。
    UpToDate,
    /// 清单没有本机目标的安装包。
    NoPackage,
    /// 当前版本太旧，不能直接升到该版本。
    TooOld {
        min_from: String,
    },
    Available,
}

pub fn evaluate(
    m: &Manifest,
    current: &str,
    target: &str,
    allow_prerelease: bool,
) -> anyhow::Result<Evaluation> {
    let new = Version::parse(&m.version).with_context(|| format!("清单版本无效: {}", m.version))?;
    let cur = Version::parse(current).with_context(|| format!("当前版本无效: {current}"))?;

    if !allow_prerelease && !new.pre.is_empty() {
        return Ok(Evaluation::UpToDate);
    }
    if new <= cur {
        return Ok(Evaluation::UpToDate);
    }
    if let Some(min) = m.min_from_version.as_deref() {
        let min_v = Version::parse(min).with_context(|| format!("min_from_version 无效: {min}"))?;
        if cur < min_v {
            return Ok(Evaluation::TooOld {
                min_from: min.to_string(),
            });
        }
    }
    if !m.targets.contains_key(target) {
        return Ok(Evaluation::NoPackage);
    }
    Ok(Evaluation::Available)
}

/// sha256 必须是 64 位十六进制，防止清单里的坏值导致永远校验不过或绕过比较。
pub fn check_sha256_format(s: &str) -> anyhow::Result<()> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("sha256 格式无效");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 测试专用密钥对（私钥已丢弃）对 TEST_MSG 的签名。
    const TEST_PK: &str = "RWSbH5i+4nec96Hjkdj1hh3Si7g27dcNCN5tiY0SmhsYGW5ADHvKUBQH";
    const TEST_MSG: &[u8] = b"{\"version\":\"1.2.3\"}\n";
    const TEST_SIG: &str = "untrusted comment: signature from minisign secret key\nRUSbH5i+4nec92zYkrUiDM+8Zvk9/24jh/7auyhUJQda+8pIxq9h9oxSSSAz/zrz4uO/HTKWzCujIdqr8KsigtPU9Ih7PB3TcAM=\ntrusted comment: timestamp:1794982066\tfile:msg.json\thashed\n/kTpj8H+8GBuZZ8u7NfG9/31h4l9ibXrc1G3RkYVrNNV7p9u2HRr6lOb/ZdTAIc4PFm4oMXc4IknnyJBS8QwCQ==\n";

    #[test]
    fn valid_signature_passes() {
        verify(TEST_PK, TEST_MSG, TEST_SIG).unwrap();
    }

    #[test]
    fn tampered_manifest_fails() {
        assert!(verify(TEST_PK, b"{\"version\":\"9.9.9\"}\n", TEST_SIG).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        // 正式公钥不是这份签名的签发者。
        assert!(verify(crate::update::PUBLIC_KEY, TEST_MSG, TEST_SIG).is_err());
    }

    #[test]
    fn install_scripts_embed_the_same_public_key() {
        // 安装脚本验签用的是它自己嵌入的公钥副本，必须与编进二进制的一致。
        for script in [
            include_str!("../../packaging/linux/install.sh"),
            include_str!("../../packaging/macos/install.sh"),
        ] {
            assert!(script.contains(crate::update::PUBLIC_KEY));
        }
    }

    #[test]
    fn garbage_signature_fails() {
        assert!(verify(TEST_PK, TEST_MSG, "not a signature").is_err());
    }

    fn manifest(version: &str, min_from: Option<&str>) -> Manifest {
        let mut targets = HashMap::new();
        targets.insert(
            "aarch64-apple-darwin".to_string(),
            TargetEntry {
                url: "https://example.invalid/x.tar.gz".into(),
                sha256: "0".repeat(64),
                size: 1,
            },
        );
        Manifest {
            version: version.into(),
            notes_url: String::new(),
            min_from_version: min_from.map(String::from),
            targets,
        }
    }

    const T: &str = "aarch64-apple-darwin";

    #[test]
    fn newer_is_available_same_or_older_is_not() {
        assert_eq!(
            evaluate(&manifest("0.4.0", None), "0.3.0", T, false).unwrap(),
            Evaluation::Available
        );
        assert_eq!(
            evaluate(&manifest("0.3.0", None), "0.3.0", T, false).unwrap(),
            Evaluation::UpToDate
        );
        // 绝不降级：即使清单被回滚到旧版本也不装。
        assert_eq!(
            evaluate(&manifest("0.2.0", None), "0.3.0", T, false).unwrap(),
            Evaluation::UpToDate
        );
    }

    #[test]
    fn prerelease_only_on_beta() {
        let m = manifest("0.4.0-rc.1", None);
        assert_eq!(
            evaluate(&m, "0.3.0", T, false).unwrap(),
            Evaluation::UpToDate
        );
        assert_eq!(
            evaluate(&m, "0.3.0", T, true).unwrap(),
            Evaluation::Available
        );
        // rc.2 > rc.1
        assert_eq!(
            evaluate(&manifest("0.4.0-rc.2", None), "0.4.0-rc.1", T, true).unwrap(),
            Evaluation::Available
        );
        // 正式版 > 同号 rc
        assert_eq!(
            evaluate(&manifest("0.4.0", None), "0.4.0-rc.2", T, false).unwrap(),
            Evaluation::Available
        );
    }

    #[test]
    fn min_from_and_missing_target() {
        assert_eq!(
            evaluate(&manifest("0.5.0", Some("0.4.0")), "0.3.0", T, false).unwrap(),
            Evaluation::TooOld {
                min_from: "0.4.0".into()
            }
        );
        assert_eq!(
            evaluate(
                &manifest("0.4.0", None),
                "0.3.0",
                "x86_64-unknown-linux-musl",
                false
            )
            .unwrap(),
            Evaluation::NoPackage
        );
    }

    #[test]
    fn sha_format() {
        assert!(check_sha256_format(&"a".repeat(64)).is_ok());
        assert!(check_sha256_format("abc").is_err());
        assert!(check_sha256_format(&"g".repeat(64)).is_err());
    }
}
