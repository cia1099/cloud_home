-- 账户审批：新注册的账户先处于 pending，由管理员批准后才能登录。
--
-- 已有账户一律视为 approved（DEFAULT），已签发的会话不受影响。
-- pending / rejected 账户同时保持 is_active = 0——任何只认 is_active 的旧代码路径
-- 仍会拒绝它们（fail closed）。
ALTER TABLE users ADD COLUMN approval_status TEXT NOT NULL DEFAULT 'approved'
    CHECK (approval_status IN ('pending', 'approved', 'rejected'));
ALTER TABLE users ADD COLUMN approval_decided_at TEXT;
-- 决定者：'open' | 'bootstrap' | 'admin-endpoint' | 'expiry' | 'chat:<provider>:<sender-id>'
ALTER TABLE users ADD COLUMN approval_decided_by TEXT;
ALTER TABLE users ADD COLUMN approval_note TEXT;
-- 注册来源 IP：给管理员判断申请真伪，也用于按 IP 限流。属于个人数据，不外发。
ALTER TABLE users ADD COLUMN registered_ip TEXT;

CREATE INDEX idx_users_approval_status ON users(approval_status);
CREATE INDEX idx_users_registered_ip ON users(registered_ip, created_at);
