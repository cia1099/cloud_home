use std::path::PathBuf;

/// 注册模式（`REGISTRATION_MODE`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    /// 注册即启用，签发令牌。
    Open,
    /// 注册后处于 pending，须管理员批准；不签发令牌。
    Approval,
    /// 关闭注册。
    Closed,
}

impl std::str::FromStr for RegistrationMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "approval" => Ok(Self::Approval),
            "closed" => Ok(Self::Closed),
            other => anyhow::bail!("REGISTRATION_MODE 只能是 open|approval|closed，收到 {other:?}"),
        }
    }
}

/// 注册策略与滥用限制。
#[derive(Debug, Clone, Copy)]
pub struct RegistrationPolicy {
    pub mode: RegistrationMode,
    /// 同一来源 IP 每小时最多提交的注册数（仅 approval 模式）。
    pub rate_per_ip_per_hour: i64,
    /// pending 账户总数上限，超出返回 429（仅 approval 模式）。
    pub pending_max: i64,
    /// 被拒绝的标识在此天数内不得重新注册。
    pub reject_cooldown_days: i64,
    /// pending 超过此天数自动拒绝。
    pub pending_expire_days: i64,
}

/// 全局配置，从 .env / 环境变量加载。
///
/// 注意：`DATABASE_URL` 不在此处，它由程序在运行时根据侦测到的硬盘路径构造。
///
/// 刻意不 `#[derive(Debug)]`——`jwt_secret` 是明文密钥，手写 Debug 将其脱敏，
/// 杜绝整体 `{:?}` 打印时泄露。
#[derive(Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    /// macOS 专用的挂载扫描根目录（Linux 自动扫描 /media 与 /run/media）。
    pub volumes_watch_path: PathBuf,
    /// 硬盘上存放数据的目录名。
    pub drive_data_dir: String,
    pub jwt_secret: String,
    pub jwt_expires_in_hours: i64,
    pub max_upload_size_mb: u64,
    pub trash_retention_days: i64,
    pub trash_cleanup_interval_hours: u64,
    pub cors_allowed_origins: Vec<String>,
    pub public_base_url: String,
    /// 日志文件目录（本地磁盘，非外接硬盘——硬盘卸载正是最需要记录的时刻）。
    pub log_dir: PathBuf,
    /// 控制台日志格式：`pretty`（默认，人类可读）或 `json`。文件恒为 JSON。
    pub log_format: String,
    /// 是否写日志文件。
    pub log_file_enabled: bool,
    /// 单个日志文件大小上限（MB），超过即滚动。
    pub log_max_size_mb: u64,
    /// 是否信任反向代理的 `X-Forwarded-For` / `X-Real-IP`（默认 false，防伪造）。
    pub trust_proxy_headers: bool,
    /// 运维端点（删除 / 还原账户）的共享口令。未设置即关闭这些端点。
    pub admin_password: Option<String>,
    /// 在线更新：manual（人点安装）| auto（时间窗内空闲时自动装）。
    pub update_mode: crate::update::settings::Mode,
    /// 自动安装的时间窗（本地时间）`HH:MM-HH:MM`。
    pub update_window: String,
    /// stable（仅正式版）| beta（含 rc 预发布）。
    pub update_channel: crate::update::settings::Channel,
    pub update_check_interval_hours: u64,
    /// 安装根目录（含 versions/、current 符号链接）。`current` 不是符号链接时更新器禁用。
    pub update_root: PathBuf,
    /// 覆盖清单所在 URL 前缀——仅供测试用本地镜像，生产不要设置。
    pub update_base_url: Option<String>,
    /// 注册模式与限流。
    pub registration: RegistrationPolicy,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("server_host", &self.server_host)
            .field("server_port", &self.server_port)
            .field("volumes_watch_path", &self.volumes_watch_path)
            .field("drive_data_dir", &self.drive_data_dir)
            .field("jwt_secret", &"<redacted>")
            .field("jwt_expires_in_hours", &self.jwt_expires_in_hours)
            .field("max_upload_size_mb", &self.max_upload_size_mb)
            .field("trash_retention_days", &self.trash_retention_days)
            .field("trash_cleanup_interval_hours", &self.trash_cleanup_interval_hours)
            .field("cors_allowed_origins", &self.cors_allowed_origins)
            .field("public_base_url", &self.public_base_url)
            .field("log_dir", &self.log_dir)
            .field("log_format", &self.log_format)
            .field("log_file_enabled", &self.log_file_enabled)
            .field("log_max_size_mb", &self.log_max_size_mb)
            .field("trust_proxy_headers", &self.trust_proxy_headers)
            .field(
                "admin_password",
                &self.admin_password.as_ref().map(|_| "<redacted>"),
            )
            .field("update_mode", &self.update_mode)
            .field("update_window", &self.update_window)
            .field("update_channel", &self.update_channel)
            .field("update_check_interval_hours", &self.update_check_interval_hours)
            .field("update_root", &self.update_root)
            .field("update_base_url", &self.update_base_url)
            .field("registration", &self.registration)
            .finish()
    }
}

impl Config {
    /// 从环境变量加载配置，缺省值与 spec/backend.md 保持一致。
    pub fn from_env() -> anyhow::Result<Self> {
        // .env 缺失不视为错误（生产可用真实环境变量）。
        let _ = dotenvy::dotenv();

        Ok(Self {
            server_host: env_or("SERVER_HOST", "127.0.0.1"),
            server_port: env_parse("SERVER_PORT", 8080)?,
            volumes_watch_path: PathBuf::from(env_or("VOLUMES_WATCH_PATH", "/Volumes")),
            drive_data_dir: env_or("DRIVE_DATA_DIR", "cloud_home_data"),
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET 必须设置"))?,
            jwt_expires_in_hours: env_parse("JWT_EXPIRES_IN_HOURS", 168)?,
            max_upload_size_mb: env_parse("MAX_UPLOAD_SIZE_MB", 4096)?,
            trash_retention_days: env_parse("TRASH_RETENTION_DAYS", 7)?,
            trash_cleanup_interval_hours: env_parse("TRASH_CLEANUP_INTERVAL_HOURS", 1)?,
            cors_allowed_origins: env_or("CORS_ALLOWED_ORIGINS", "http://localhost:3000")
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            public_base_url: env_or("PUBLIC_BASE_URL", "http://localhost:8080"),
            log_dir: PathBuf::from(env_or("LOG_DIR", "./logs")),
            log_format: env_or("LOG_FORMAT", "pretty"),
            log_file_enabled: env_parse("LOG_FILE_ENABLED", true)?,
            log_max_size_mb: env_parse("LOG_MAX_SIZE_MB", 1)?,
            trust_proxy_headers: env_parse("TRUST_PROXY_HEADERS", false)?,
            admin_password: std::env::var("ADMIN_PASSWORD")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            update_mode: env_or("UPDATE_MODE", "manual").parse()?,
            update_window: {
                let w = env_or("UPDATE_WINDOW", "03:00-05:00");
                crate::update::settings::Window::parse(&w)
                    .map_err(|e| anyhow::anyhow!("UPDATE_WINDOW 无效: {e}"))?;
                w
            },
            update_channel: env_or("UPDATE_CHANNEL", "stable").parse()?,
            update_check_interval_hours: env_parse("UPDATE_CHECK_INTERVAL_HOURS", 6)?,
            update_root: PathBuf::from(env_or("UPDATE_ROOT", "/opt/cloud_home")),
            update_base_url: std::env::var("UPDATE_BASE_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            registration: RegistrationPolicy {
                mode: env_or("REGISTRATION_MODE", "approval").parse()?,
                rate_per_ip_per_hour: env_parse("REGISTER_RATE_PER_IP_PER_HOUR", 5)?,
                pending_max: env_parse("PENDING_MAX", 20)?,
                reject_cooldown_days: env_parse("REJECT_COOLDOWN_DAYS", 7)?,
                pending_expire_days: env_parse("PENDING_EXPIRE_DAYS", 14)?,
            },
        })
    }

    /// 上传大小上限（字节）。
    pub fn max_upload_size_bytes(&self) -> usize {
        (self.max_upload_size_mb * 1024 * 1024) as usize
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_parse<T>(key: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(v) => v
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("环境变量 {key} 解析失败: {e}")),
        Err(_) => Ok(default),
    }
}
