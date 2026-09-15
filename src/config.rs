use std::path::PathBuf;

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
