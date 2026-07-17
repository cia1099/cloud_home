//! 通用工具函数。

use base64::Engine;
use chrono::Utc;

/// 当前时间的 RFC3339 字符串。
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

/// 生成 32 字节 CSPRNG 随机数并以 base64url（无填充）编码，用于分享 token。
pub fn generate_share_token() -> String {
    let bytes: [u8; 32] = rand::random();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
