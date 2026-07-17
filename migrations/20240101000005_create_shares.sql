CREATE TABLE shares (
    id           TEXT PRIMARY KEY,
    file_id      TEXT NOT NULL,
    owner_id     TEXT NOT NULL,
    token        TEXT NOT NULL UNIQUE,
    can_download INTEGER NOT NULL DEFAULT 1,
    expires_at   TEXT,
    access_count INTEGER NOT NULL DEFAULT 0,
    created_at   TEXT NOT NULL,
    is_active    INTEGER NOT NULL DEFAULT 1,

    FOREIGN KEY (file_id)  REFERENCES files(id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id) REFERENCES users(id) ON DELETE CASCADE
);
CREATE INDEX idx_shares_token ON shares(token);
CREATE INDEX idx_shares_owner ON shares(owner_id);
