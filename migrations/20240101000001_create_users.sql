CREATE TABLE users (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    is_active     INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_users_email    ON users(email);
CREATE INDEX idx_users_username ON users(username);
