//! 在线更新：检查 GitHub Release → 验签下载 → 自检 → 切换版本 → 重启，失败自动回滚。
//! 总体设计见 `spec/update.md`。

pub mod boot_guard;
pub mod checker;
pub mod installer;
pub mod manifest;
pub mod settings;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use anyhow::Context;
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::db::Db;
use crate::drive::DriveManager;
use crate::events::ChangeNotifier;
use crate::lifecycle::ActivityTracker;
use manifest::{Evaluation, Manifest};
use settings::{Channel, Settings};

/// 发布签名公钥（minisign）。对应私钥只存在于 GitHub Actions 的 `MINISIGN_SECRET_KEY`
/// 与离线备份里；设备只信任编进二进制的这把公钥。**更换密钥需要先发一版同时认新旧两把的过渡版本。**
pub const PUBLIC_KEY: &str = "RWR4IcEpkUB52UT0pqy7cHKuT1IKMRljRctf8rcOYT332Z29K23oGbp3";
pub const REPO: &str = "cia1099/cloud_home";
const LATEST_BASE_URL: &str = "https://github.com/cia1099/cloud_home/releases/latest/download";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Checking,
    Downloading,
    Installing,
    /// 新版本已就位，正在关停以便守护进程拉起它。
    Restarting,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct Available {
    pub version: String,
    pub notes_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub phase: Phase,
    pub current_version: &'static str,
    pub available: Option<Available>,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("在线更新未启用（不是由安装目录运行）")]
    Disabled,
    #[error("没有可安装的新版本")]
    NoUpdate,
    #[error("有 {0} 个传输正在进行")]
    Busy(usize),
    #[error("更新正在进行中")]
    InProgress,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

struct Inner {
    cfg: Arc<Config>,
    root: PathBuf,
    http: reqwest::Client,
    settings: RwLock<Settings>,
    status: RwLock<Snapshot>,
    /// 最近一次验签通过的清单（安装用它，而不是重新拉取，保证装的就是展示过的那个）。
    manifest: RwLock<Option<Manifest>>,
    last_check: Mutex<Option<Instant>>,
    install_lock: tokio::sync::Mutex<()>,
    db: Db,
    drive_manager: DriveManager,
    activity: ActivityTracker,
    shutdown: CancellationToken,
    events: ChangeNotifier,
}

#[derive(Clone)]
pub struct Updater {
    inner: Arc<Inner>,
}

impl Updater {
    pub fn new(
        cfg: Arc<Config>,
        db: Db,
        drive_manager: DriveManager,
        activity: ActivityTracker,
        shutdown: CancellationToken,
        events: ChangeNotifier,
    ) -> anyhow::Result<Self> {
        // reqwest 用 rustls-no-provider：这里显式装 ring。已装过则忽略。
        let _ = rustls::crypto::ring::default_provider().install_default();

        let http = reqwest::Client::builder()
            .user_agent(format!("cloud_home-updater/{}", crate::version::VERSION))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .context("创建 HTTP 客户端失败")?;

        let root = cfg.update_root.clone();
        let defaults = Settings {
            mode: cfg.update_mode,
            window: cfg.update_window.clone(),
            channel: cfg.update_channel,
            check_interval_hours: cfg.update_check_interval_hours,
        };
        let settings = Settings::load(&root, defaults);

        Ok(Self {
            inner: Arc::new(Inner {
                cfg,
                root,
                http,
                settings: RwLock::new(settings),
                status: RwLock::new(Snapshot {
                    phase: Phase::Idle,
                    current_version: crate::version::VERSION,
                    available: None,
                    last_checked_at: None,
                    last_error: None,
                }),
                manifest: RwLock::new(None),
                last_check: Mutex::new(None),
                install_lock: tokio::sync::Mutex::new(()),
                db,
                drive_manager,
                activity,
                shutdown,
                events,
            }),
        })
    }

    /// 只有从安装目录（`root/current` 是符号链接）运行时才启用，
    /// 因此开发机上的 `cargo run` 永远不会自己更新自己。
    pub fn enabled(&self) -> bool {
        is_managed(&self.inner.root)
    }

    pub fn root(&self) -> &Path {
        &self.inner.root
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.status.read().unwrap().clone()
    }

    pub fn settings(&self) -> Settings {
        self.inner.settings.read().unwrap().clone()
    }

    /// 更新并持久化设置（校验失败则不改动）。
    pub fn set_settings(&self, new: Settings) -> anyhow::Result<()> {
        new.save(&self.inner.root)?;
        *self.inner.settings.write().unwrap() = new;
        Ok(())
    }

    fn set_phase(&self, phase: Phase) {
        self.inner.status.write().unwrap().phase = phase;
    }

    fn busy(&self) -> bool {
        matches!(
            self.snapshot().phase,
            Phase::Checking | Phase::Downloading | Phase::Installing | Phase::Restarting
        )
    }

    /// 距离上次检查是否已超过设置的间隔（含 0~10% 抖动，错开设备同时请求 GitHub）。
    pub(crate) fn check_due(&self) -> bool {
        let interval = Duration::from_secs(self.settings().check_interval_hours * 3600);
        let jitter = interval.mul_f64(rand::random::<f64>() * 0.1);
        match *self.inner.last_check.lock().unwrap() {
            None => true,
            Some(t) => t.elapsed() >= interval + jitter,
        }
    }

    pub fn activity(&self) -> &ActivityTracker {
        &self.inner.activity
    }

    /// 解析清单所在目录的 URL：测试覆盖 > beta 通道（取最新的含预发布 Release）> 稳定版 `latest`。
    async fn base_url(&self, channel: Channel) -> anyhow::Result<String> {
        if let Some(u) = &self.inner.cfg.update_base_url {
            return Ok(u.trim_end_matches('/').to_string());
        }
        if channel == Channel::Stable {
            return Ok(LATEST_BASE_URL.to_string());
        }
        let body = self
            .http_get(&format!(
                "https://api.github.com/repos/{REPO}/releases?per_page=10"
            ))
            .await?;
        #[derive(serde::Deserialize)]
        struct Rel {
            tag_name: String,
            #[serde(default)]
            draft: bool,
        }
        let rels: Vec<Rel> =
            serde_json::from_slice(&body).context("GitHub Release 列表解析失败")?;
        let tag = rels
            .into_iter()
            .find(|r| !r.draft)
            .context("没有可用的 Release")?
            .tag_name;
        Ok(format!("https://github.com/{REPO}/releases/download/{tag}"))
    }

    async fn http_get(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self
            .inner
            .http
            .get(url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .error_for_status()?;
        if resp
            .content_length()
            .is_some_and(|l| l > manifest::MAX_MANIFEST_BYTES as u64)
        {
            anyhow::bail!("响应过大");
        }
        let bytes = resp.bytes().await?;
        if bytes.len() > manifest::MAX_MANIFEST_BYTES {
            anyhow::bail!("响应过大");
        }
        Ok(bytes.to_vec())
    }

    /// 检查更新：拉清单 → 验签 → 解析 → 比较。结果写入 [`Snapshot`]。
    pub async fn check(&self) -> Result<Option<Available>, UpdateError> {
        if !self.enabled() {
            return Err(UpdateError::Disabled);
        }
        if self.busy() {
            return Err(UpdateError::InProgress);
        }
        self.set_phase(Phase::Checking);
        *self.inner.last_check.lock().unwrap() = Some(Instant::now());

        let result = self.check_inner().await;
        let mut st = self.inner.status.write().unwrap();
        st.phase = Phase::Idle;
        st.last_checked_at = Some(crate::util::now_rfc3339());
        match &result {
            Ok(avail) => {
                st.last_error = None;
                st.available = avail.clone();
            }
            Err(e) => st.last_error = Some(format!("{e:#}")),
        }
        drop(st);
        result.map_err(UpdateError::Other)
    }

    async fn check_inner(&self) -> anyhow::Result<Option<Available>> {
        let settings = self.settings();
        let base = self.base_url(settings.channel).await?;
        let body = self.http_get(&format!("{base}/manifest.json")).await?;
        let sig = self
            .http_get(&format!("{base}/manifest.json.minisig"))
            .await?;
        // 先验签，再解析：未经验证的字节绝不进入 JSON 解析器之后的任何逻辑。
        manifest::verify(PUBLIC_KEY, &body, &String::from_utf8_lossy(&sig))?;
        let m = manifest::parse(&body)?;

        let eval = manifest::evaluate(
            &m,
            crate::version::VERSION,
            crate::version::TARGET,
            settings.channel == Channel::Beta,
        )?;
        let failed = boot_guard::UpdateState::load(&self.inner.root).last_failed;
        let avail = match eval {
            Evaluation::Available if failed.as_deref() == Some(m.version.as_str()) => {
                tracing::warn!(version = %m.version, "该版本曾回滚，跳过");
                None
            }
            Evaluation::Available => {
                let entry = &m.targets[crate::version::TARGET];
                manifest::check_sha256_format(&entry.sha256)?;
                Some(Available {
                    version: m.version.clone(),
                    notes_url: m.notes_url.clone(),
                    size: entry.size,
                })
            }
            Evaluation::TooOld { min_from } => {
                anyhow::bail!(
                    "当前版本过旧，需先升级到 {min_from} 以上才能安装 {}",
                    m.version
                )
            }
            Evaluation::UpToDate | Evaluation::NoPackage => None,
        };

        let is_new = avail.as_ref().map(|a| a.version.clone())
            != self.snapshot().available.map(|a| a.version);
        if let Some(a) = &avail {
            if is_new {
                crate::audit!(info, "update.check", "available", version = %a.version, "发现新版本");
                self.inner.events.notify_update(&a.version);
            }
            *self.inner.manifest.write().unwrap() = Some(m);
        } else {
            *self.inner.manifest.write().unwrap() = None;
        }
        Ok(avail)
    }

    /// 安装已发现的新版本。成功后触发优雅关停，由守护进程拉起新版本。
    /// `force = false` 时若有传输进行中则拒绝。
    pub async fn install(&self, force: bool) -> Result<(), UpdateError> {
        if !self.enabled() {
            return Err(UpdateError::Disabled);
        }
        let _lock = self
            .inner
            .install_lock
            .try_lock()
            .map_err(|_| UpdateError::InProgress)?;
        if self.busy() {
            return Err(UpdateError::InProgress);
        }
        let m = self
            .inner
            .manifest
            .read()
            .unwrap()
            .clone()
            .ok_or(UpdateError::NoUpdate)?;
        let active = self.inner.activity.active();
        if !force && active > 0 {
            return Err(UpdateError::Busy(active));
        }

        crate::audit!(info, "update.install", "started", version = %m.version, force, "开始安装更新");
        match self.install_inner(&m).await {
            Ok(()) => {
                self.set_phase(Phase::Restarting);
                crate::audit!(info, "update.install", "success", version = %m.version, "更新已就位，重启中");
                self.inner.shutdown.cancel();
                Ok(())
            }
            Err(e) => {
                crate::audit!(error, "update.install", "failure", version = %m.version, error = %format!("{e:#}"), "安装更新失败");
                let mut st = self.inner.status.write().unwrap();
                st.phase = Phase::Failed;
                st.last_error = Some(format!("{e:#}"));
                Err(UpdateError::Other(e))
            }
        }
    }

    async fn install_inner(&self, m: &Manifest) -> anyhow::Result<()> {
        let target = crate::version::TARGET;
        let entry = m.targets.get(target).context("清单没有本机目标的安装包")?;
        manifest::check_sha256_format(&entry.sha256)?;
        // 清单已验签；仍要求 https，防止被降级到明文传输（测试覆盖 base_url 时放行）。
        if !entry.url.starts_with("https://") && self.inner.cfg.update_base_url.is_none() {
            anyhow::bail!("安装包 URL 必须是 https");
        }

        let root = &self.inner.root;
        let staging = root.join("update").join("staging");
        let tarball = staging.join(format!("cloud_home-{}-{target}.tar.gz", m.version));

        self.set_phase(Phase::Downloading);
        installer::download(
            &self.inner.http,
            &entry.url,
            &tarball,
            entry.size,
            &entry.sha256,
        )
        .await?;

        self.set_phase(Phase::Installing);
        let partial = root.join("versions").join(format!("{}.partial", m.version));
        let final_dir = root.join("versions").join(&m.version);
        let _ = tokio::fs::remove_dir_all(&partial).await;
        {
            let (t, v, p) = (tarball.clone(), m.version.clone(), partial.clone());
            tokio::task::spawn_blocking(move || installer::extract(&t, &v, target, &p)).await??;
        }
        installer::self_test(&partial.join("cloud_home"), &m.version).await?;
        let _ = tokio::fs::remove_dir_all(&final_dir).await;
        tokio::fs::rename(&partial, &final_dir).await?;
        let _ = tokio::fs::remove_file(&tarball).await;

        // 切换前先备份数据库：新版本可能迁移 schema，回滚的旧二进制需要旧 schema。
        let data_root = self.inner.drive_manager.require_data_root().await?;
        let backup =
            installer::backup_db(&self.inner.db, &data_root, crate::version::VERSION).await?;

        let prev_failed = boot_guard::UpdateState::load(root).last_failed;
        let state = boot_guard::UpdateState {
            pending: Some(m.version.clone()),
            attempts: 0,
            backup: Some(backup),
            last_failed: prev_failed,
        };
        state.save(root)?;
        if let Err(e) = installer::activate(root, &m.version) {
            // 切换失败：撤销 pending，免得下次启动误判成「新版本启动失败」。
            let mut s = state;
            s.pending = None;
            let _ = s.save(root);
            return Err(e);
        }
        Ok(())
    }
}

/// `root/current` 是符号链接 → 由安装器部署、受更新管理。
pub fn is_managed(root: &Path) -> bool {
    std::fs::symlink_metadata(root.join("current"))
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// 启动后台组件：定时检查任务 + 新版本稳定后的提交。未受管时什么也不做。
pub fn start(updater: &Updater) {
    if !updater.enabled() {
        tracing::info!(root = %updater.root().display(), "在线更新未启用（非安装目录运行）");
        return;
    }
    tokio::spawn(boot_guard::commit_after_health(
        updater.root().to_path_buf(),
        crate::version::VERSION.to_string(),
    ));
    checker::spawn(updater.clone());
}
