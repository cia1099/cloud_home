CREATE TABLE drives (
    id           TEXT PRIMARY KEY,
    mount_path   TEXT NOT NULL UNIQUE,
    label        TEXT,
    is_active    INTEGER NOT NULL DEFAULT 1,
    detected_at  TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);
