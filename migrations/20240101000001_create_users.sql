-- 账户基础：个人资料（users）、凭证与标识（user_identities）、验证码（otp_codes）。
--
-- users 只存个人资料：username 是可改、不唯一的显示名，登录一律靠
-- user_identities 里的 email / phone。凭证（密码哈希）同样是一条 identity，
-- 这样将来接入 Google / Apple 只是多插一行，不必动表结构。

CREATE TABLE users (
    id         TEXT PRIMARY KEY,
    username   TEXT NOT NULL,           -- 显示名：可变、不唯一
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    deleted_at TEXT                     -- 被运维删除的时刻，同时作为回收站批次标记
);
CREATE INDEX idx_users_username ON users(username);

CREATE TABLE user_identities (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    provider    TEXT NOT NULL,          -- 'password' | 'email' | 'phone' | 'google' | 'apple'
    identifier  TEXT NOT NULL,          -- email | E.164 手机号 | 第三方 subject
    secret      TEXT,                   -- 仅 'password' 存 argon2 哈希，其余为 NULL
    verified_at TEXT,                   -- 经 OTP 或第三方确认后写入
    managed_by  TEXT,                   -- NULL = 用户自有可编辑；'google'|'apple' = 第三方托管只读
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (provider, identifier),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_identities_user       ON user_identities(user_id);
CREATE INDEX idx_identities_identifier ON user_identities(identifier);

CREATE TABLE otp_codes (
    id          TEXT PRIMARY KEY,
    user_id     TEXT,                   -- 可空：为将来手机注册 / 登录预留
    purpose     TEXT NOT NULL,          -- 'identifier_old' | 'identifier_verify'
    destination TEXT NOT NULL,          -- 邮箱或 E.164 手机号
    code_hash   TEXT NOT NULL,          -- argon2 哈希，绝不明文存储验证码
    attempts    INTEGER NOT NULL DEFAULT 0,
    expires_at  TEXT NOT NULL,
    consumed_at TEXT,
    created_at  TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_otp_lookup ON otp_codes(destination, purpose, expires_at);
