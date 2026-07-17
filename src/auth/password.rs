//! 密码哈希与验证（Argon2id）。

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::{AppError, AppResult};

/// 使用 spec 规定的参数构造 Argon2id：memory_cost=65536, iterations=2, parallelism=1。
fn argon2() -> Argon2<'static> {
    let params = Params::new(65536, 2, 1, None).expect("Argon2 参数合法");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

/// 哈希明文密码，返回 PHC 字符串。
pub fn hash(password: &str) -> AppResult<String> {
    // 用 rand 生成 16 字节盐并 base64 编码，避免依赖 rand_core 的 OsRng 特性。
    let salt_bytes: [u8; 16] = rand::random();
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|e| AppError::Other(anyhow::anyhow!("生成盐失败: {e}")))?;
    let hash = argon2()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Other(anyhow::anyhow!("密码哈希失败: {e}")))?;
    Ok(hash.to_string())
}

/// 校验明文密码是否匹配存储的 PHC 哈希。
pub fn verify(password: &str, phc: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    argon2()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}
