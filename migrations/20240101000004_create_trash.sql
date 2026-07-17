CREATE TABLE trash (
    id                 TEXT PRIMARY KEY,
    file_id            TEXT NOT NULL UNIQUE,
    owner_id           TEXT NOT NULL,
    original_parent_id TEXT,
    original_name      TEXT NOT NULL,
    deleted_at         TEXT NOT NULL,
    expires_at         TEXT NOT NULL,    -- deleted_at + 7 天

    FOREIGN KEY (file_id)  REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_trash_owner      ON trash(owner_id);
CREATE INDEX idx_trash_expires_at ON trash(expires_at);
