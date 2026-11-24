//! SQLite 连接池初始化与迁移执行。

use std::path::Path;
use std::str::FromStr;

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub type Db = SqlitePool;

/// 根据数据库文件路径建立连接池，自动创建文件并开启外键约束。
pub async fn init_pool(db_path: &Path) -> anyhow::Result<Db> {
    let url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// 运行 `migrations/` 下的所有迁移。
pub async fn run_migrations(pool: &Db) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// 测试用：独立的内存数据库，已跑完全部迁移。
#[cfg(test)]
pub async fn test_pool() -> Db {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await
        .unwrap();
    run_migrations(&pool).await.unwrap();
    pool
}
