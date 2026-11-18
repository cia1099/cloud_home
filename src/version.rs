//! 编译期版本信息。

/// 当前版本（来自 Cargo.toml）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 编译目标三元组（由 build.rs 注入）。
pub const TARGET: &str = env!("BUILD_TARGET");
