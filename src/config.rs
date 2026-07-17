use std::path::PathBuf;

/// 全局配置，从 .env / 环境变量加载。
///
/// 注意：`DATABASE_URL` 不在此处，它由程序在运行时根据侦测到的硬盘路径构造。
#[derive(Debug, Clone)]
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
