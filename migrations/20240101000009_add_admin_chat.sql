-- 管理员聊天集成（Telegram 等）：配对身份、一次性配对码、按钮令牌、待发送队列。
-- 只追加，不改动任何已有表。

-- 已配对的管理员。sender_id 是聊天提供方的不可变数字 id（不是用户名）。
CREATE TABLE admin_chat_identities (
    id        TEXT PRIMARY KEY,
    provider  TEXT NOT NULL,
    sender_id TEXT NOT NULL,
    chat_id   TEXT NOT NULL,
    label     TEXT NOT NULL,
    paired_at TEXT NOT NULL,
    UNIQUE (provider, sender_id)
);

-- 一次性配对码：只存哈希，10 分钟有效，用过即作废。
CREATE TABLE chat_pairing_codes (
    code_hash  TEXT PRIMARY KEY,
    expires_at TEXT NOT NULL,
    used_at    TEXT
);

-- 按钮令牌：不透明随机串（只存哈希），绑定「哪位管理员 + 哪个账户 + 哪个动作」，单次有效、24 小时过期。
-- 管理员被取消配对或账户被删除时随之级联删除。
CREATE TABLE chat_action_tokens (
    token_hash  TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES admin_chat_identities(id) ON DELETE CASCADE,
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    action      TEXT NOT NULL CHECK (action IN ('approve', 'reject')),
    expires_at  TEXT NOT NULL,
    used_at     TEXT
);
CREATE INDEX idx_chat_action_tokens_user ON chat_action_tokens(user_id);

-- 待发送 / 已发送消息。聊天不可用时消息留在队列里按指数退避重试，不丢。
-- kind: notify（新申请通知，带按钮）| summary（突发合并摘要）| reminder（每日提醒）
-- state: queued | sent | failed | suppressed（被突发合并吞掉，仅用于计数，不投递）
CREATE TABLE chat_outbox (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    identity_id     TEXT NOT NULL REFERENCES admin_chat_identities(id) ON DELETE CASCADE,
    kind            TEXT NOT NULL,
    user_id         TEXT REFERENCES users(id) ON DELETE CASCADE,
    text            TEXT NOT NULL,
    -- JSON：[{"label": "...", "token": "..."}]；发送成功后清空。
    buttons         TEXT,
    state           TEXT NOT NULL DEFAULT 'queued',
    attempts        INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT NOT NULL,
    -- 发送成功后提供方返回的消息引用（JSON），用于事后编辑。
    sent_ref        TEXT,
    created_at      TEXT NOT NULL
);
CREATE INDEX idx_chat_outbox_due ON chat_outbox(state, next_attempt_at);
CREATE INDEX idx_chat_outbox_user ON chat_outbox(user_id, kind);
