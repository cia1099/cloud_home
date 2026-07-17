CREATE TABLE files (
    id              TEXT PRIMARY KEY,
    owner_id        TEXT NOT NULL,
    parent_id       TEXT,
    drive_id        TEXT NOT NULL,
    name            TEXT NOT NULL,
    file_type       TEXT NOT NULL,       -- 'file' | 'folder'
    size_bytes      INTEGER NOT NULL DEFAULT 0,
    mime_type       TEXT,
    physical_path   TEXT,                -- 仅 file 有值；资料夹为 NULL
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    is_deleted      INTEGER NOT NULL DEFAULT 0,

    FOREIGN KEY (owner_id)  REFERENCES users(id)  ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES files(id)  ON DELETE SET NULL,
    FOREIGN KEY (drive_id)  REFERENCES drives(id)
);
CREATE UNIQUE INDEX idx_files_unique_name
    ON files(owner_id, parent_id, name) WHERE is_deleted = 0;
CREATE INDEX idx_files_owner   ON files(owner_id);
CREATE INDEX idx_files_parent  ON files(parent_id);
CREATE INDEX idx_files_deleted ON files(is_deleted);
