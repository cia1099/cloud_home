-- 聊天告警里「安装更新」按钮的令牌（R-065）：不透明随机串（只存哈希），
-- 绑定「哪位管理员 + 哪个版本」，单次有效、24 小时过期；管理员被取消配对时级联删除。
CREATE TABLE chat_update_tokens (
    token_hash  TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES admin_chat_identities(id) ON DELETE CASCADE,
    version     TEXT NOT NULL,
    expires_at  TEXT NOT NULL,
    used_at     TEXT
);
