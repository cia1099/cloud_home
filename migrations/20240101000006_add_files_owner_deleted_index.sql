CREATE INDEX IF NOT EXISTS idx_files_owner_deleted ON files(owner_id, is_deleted);
