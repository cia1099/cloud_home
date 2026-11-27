-- 站内定向分享（R-071）：一条分享若有收件人行，就是「分享给指定账户」，
-- 只能由这些账户凭登录会话访问，公开 token 端点对它一律 404；
-- 没有收件人行的分享仍是原来的公开链接。
CREATE TABLE share_recipients (
    share_id   TEXT NOT NULL,
    user_id    TEXT NOT NULL,
    created_at TEXT NOT NULL,
    -- 收件人首次打开的时间；NULL 表示还没看过（前端显示未读点）。
    seen_at    TEXT,

    PRIMARY KEY (share_id, user_id),
    FOREIGN KEY (share_id) REFERENCES shares(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id)  REFERENCES users(id)  ON DELETE CASCADE
);
CREATE INDEX idx_share_recipients_user ON share_recipients(user_id);
