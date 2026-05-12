# Cloud Home — 后端实现规范

## Context

这是一个替代 Google Drive 的家庭云端存储服务，后端完全用 Rust 实现。目标：通过 RESTful API 管理外接硬盘上的文件，支持多账户隔离、回收站保留 7 天、分享链接访问。当前项目仅有 Hello World（`src/main.rs`），所有功能从零开始。

**关键决策：**
- SQLite 数据库文件存放在外接硬盘上（随盘携带，易于迁移）
- 账户注册开放：任何人均可通过 API 注册
- JWT 存放在 HttpOnly Cookie 中（浏览器自动携带，重开页面保持登录，防 XSS）；同时支持 `Authorization: Bearer` header 供 API 客户端使用

---

## 技术栈（Cargo.toml 依赖）

```toml
[dependencies]
# Web 框架
axum = { version = "0.8", features = ["multipart"] }
tower = { version = "0.5", features = ["full"] }
tower-http = { version = "0.6", features = ["cors", "trace", "limit"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["io"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 数据库
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio", "migrate", "chrono", "uuid"] }

# 认证
jsonwebtoken = "9"
argon2 = "0.5"

# UUID / 时间 / 随机数
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
rand = "0.9"
base64 = "0.22"

# 错误处理
thiserror = "2"
anyhow = "1"

# 环境变量
dotenvy = "0.15"

# 文件系统监控（跨平台：macOS FSEvents / Linux inotify，notify 自动选择后端）
notify = "7"
libc = "0.2"

# MIME 类型检测
mime_guess = "2"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

---

## 项目目录结构

```
cloud_home/
├── Cargo.toml
├── .env                              # 本地环境变量（git ignore）
├── .env.example                      # 模板（纳入 git）
├── migrations/
│   ├── 20240101000001_create_users.sql
│   ├── 20240101000002_create_drives.sql
│   ├── 20240101000003_create_files.sql
│   ├── 20240101000004_create_trash.sql
│   └── 20240101000005_create_shares.sql
└── src/
    ├── main.rs           # 入口：组装 AppState、启动服务和 trash_cleaner 任务
    ├── config.rs         # 从 .env 加载的配置结构体
    ├── error.rs          # AppError 枚举，实现 IntoResponse
    ├── state.rs          # AppState { db, config, drive_manager }
    ├── db/
    │   └── mod.rs        # SqlitePool 初始化，运行 sqlx migrate
    ├── models/
    │   ├── user.rs       # User, CreateUserDto, LoginDto
    │   ├── file.rs       # FileEntry, FileType enum
    │   ├── share.rs      # ShareLink
    │   └── trash.rs      # TrashEntry
    ├── auth/
    │   ├── jwt.rs        # JWT 签发与验证，Claims 结构体
    │   ├── password.rs   # Argon2id 哈希与验证
    │   └── extractor.rs  # axum AuthUser extractor（Cookie 优先，fallback Bearer header）
    ├── drive/
    │   ├── detector.rs   # 跨平台硬盘扫描（#[cfg(target_os)] macOS + Linux）
    │   ├── manager.rs    # DriveManager：Arc<RwLock<Option<ActiveDrive>>>
    │   └── path_resolver.rs  # 逻辑 file_id → 硬盘物理路径
    ├── handlers/
    │   ├── mod.rs        # Router 组装，中间件注册
    │   ├── auth.rs       # /auth/*
    │   ├── files.rs      # /files/*
    │   ├── folders.rs    # /folders
    │   ├── search.rs     # /search
    │   ├── trash.rs      # /trash/*
    │   ├── shares.rs     # /shares/*
    │   ├── public.rs     # /public/shares/:token（无 auth）
    │   └── drives.rs     # /drives
    ├── services/
    │   ├── file_service.rs   # 文件业务逻辑（上传/下载/移动）
    │   ├── trash_service.rs  # 软删除、恢复、清理
    │   └── share_service.rs  # token 生成与验证
    └── tasks/
        └── trash_cleaner.rs  # tokio::spawn 定时任务，每小时清理过期文件
```

---

## 数据库 Schema

### migrations/20240101000001_create_users.sql
```sql
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
```

### migrations/20240101000002_create_drives.sql
```sql
CREATE TABLE drives (
    id           TEXT PRIMARY KEY,
    mount_path   TEXT NOT NULL UNIQUE,
    label        TEXT,
    is_active    INTEGER NOT NULL DEFAULT 1,
    detected_at  TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);
```

### migrations/20240101000003_create_files.sql
```sql
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
```

### migrations/20240101000004_create_trash.sql
```sql
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
```

### migrations/20240101000005_create_shares.sql
```sql
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
```

---

## API 接口（完整列表）

Base URL: `http://localhost:8080/api/v1`

**认证方式（双模式，服务端同时支持）：**
- **Cookie 模式（前端用）**：登录后服务端 `Set-Cookie: auth_token=<jwt>; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=604800`，浏览器自动携带，重开页面保持登录状态
- **Bearer 模式（API 客户端用）**：`Authorization: Bearer <jwt_token>`

`auth/extractor.rs` 读取优先级：先找 Cookie `auth_token`，找不到再找 `Authorization` header。

错误格式: `{ "error": { "code": "ERROR_CODE", "message": "..." } }`

### 认证

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| POST | /auth/register | 无 | 注册账户，设置 auth_token Cookie，同时返回 token 字段 |
| POST | /auth/login    | 无 | 登录，设置 auth_token Cookie，同时返回 token + expires_at |
| POST | /auth/logout   | JWT | 清除 auth_token Cookie（`Set-Cookie: auth_token=; Max-Age=0`） |
| GET  | /auth/me       | JWT | 获取当前用户信息 |

### 硬盘状态

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| GET  | /drives | JWT | 列出侦测到的外接硬盘，返回 active_drive |

### 文件操作

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| GET    | /files                      | JWT | 列出目录内容（`?parent_id=`，省略为根目录） |
| POST   | /files/upload               | JWT | 上传文件（multipart/form-data，支持 `parent_id` 和 `name`） |
| GET    | /files/:id                  | JWT | 获取文件元数据 |
| GET    | /files/:id/download         | JWT | 流式下载文件 |
| PATCH  | /files/:id                  | JWT | 重命名（body: `{ "name": "..." }`） |
| POST   | /files/:id/move             | JWT | 移动（body: `{ "target_parent_id": "..." }`） |
| DELETE | /files/:id                  | JWT | 移入回收站（软删除，若是资料夹则递归） |

### 资料夹

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| POST | /folders | JWT | 建立资料夹（body: `{ "name": "...", "parent_id": null }`） |

### 搜索

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| GET | /search | JWT | 按关键字搜索当前用户的文件和资料夹名称 |

**Query 参数：**

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `q` | string | 是 | 搜索关键字（最少 1 字符，最多 100 字符） |
| `type` | string | 否 | `file` \| `folder`，不填返回两者 |
| `page` | int | 否 | 默认 1 |
| `per_page` | int | 否 | 默认 50，最大 200 |

**实现方式：** SQLite `LIKE` 模糊匹配：

```sql
SELECT * FROM files
WHERE owner_id = ?
  AND is_deleted = 0
  AND name LIKE '%' || ? || '%'
  AND (file_type = ? OR ? IS NULL)
ORDER BY name ASC
LIMIT ? OFFSET ?
```

**成功响应 `200 OK`：**
```json
{
  "data": {
    "query": "photo",
    "items": [
      {
        "id": "...",
        "name": "photo.jpg",
        "file_type": "file",
        "size_bytes": 2048576,
        "mime_type": "image/jpeg",
        "parent_id": "...",
        "parent_path": "Documents/2025",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
      }
    ],
    "pagination": { "page": 1, "per_page": 50, "total": 3 }
  }
}
```

> `parent_path` 由服务端用 CTE 递归查询拼出，让前端可直接显示文件所在位置。

### 回收站

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| GET    | /trash              | JWT | 列出回收站内容（含 expires_at） |
| POST   | /trash/:id/restore  | JWT | 恢复文件（原目录已删则恢复到根目录） |
| DELETE | /trash/:id          | JWT | 立即永久删除单个文件 |
| DELETE | /trash              | JWT | 清空回收站，返回删除数量和释放空间 |

### 文件分享

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| POST   | /shares          | JWT | 创建分享链接（body: `{ "file_id", "can_download", "expires_at" }`） |
| GET    | /shares          | JWT | 列出自己创建的所有分享链接 |
| DELETE | /shares/:id      | JWT | 撤销分享链接（设 is_active=0） |

### 公开访问（无需认证）

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| GET | /public/shares/:token           | 无 | 查看分享文件信息（含 `shared_by` 用户名） |
| GET | /public/shares/:token/download  | 无 | 流式下载分享文件，每次访问 +1 access_count |

错误码：`404 NOT_FOUND`、`410 Gone`（已过期）、`403 FORBIDDEN`（can_download=false）

---

## 外接硬盘侦测方案（跨平台）

**文件：** `src/drive/detector.rs`

开发环境为 macOS，生产部署为 Ubuntu 24。使用 `#[cfg(target_os)]` 条件编译实现两套扫描逻辑，对外暴露统一接口 `DriveDetector::scan()` 和 `DriveDetector::watch_path()`。

### macOS 策略

| 项目 | 说明 |
|------|------|
| 扫描根目录 | `/Volumes/` |
| 挂载验证 | `libc::statfs` 检查 `f_flags & MNT_LOCAL` |
| 排除系统卷 | 名称含 `Macintosh HD`、`Recovery`、`Preboot`、`VM`、`Update` |
| 实时监控 | `notify::RecommendedWatcher`（FSEvents）监控 `/Volumes/` |

### Linux（Ubuntu 24）策略

| 项目 | 说明 |
|------|------|
| 扫描根目录 | `/media/` 和 `/run/media/`（udisks2 自动挂载位置） |
| 挂载验证 | 解析 `/proc/mounts`，过滤外部存储文件系统类型 |
| 允许的文件系统类型 | `vfat`、`ntfs`、`exfat`、`ext4`、`btrfs`、`xfs` |
| 实时监控 | `notify::RecommendedWatcher`（inotify）监控 `/media/` 和 `/run/media/` |

### 共同行为

- `DriveManager` 持有 `Arc<RwLock<Option<PathBuf>>>`，卸载时设为 None
- 硬盘不可用时所有写操作返回 `503 DRIVE_UNAVAILABLE`
- 多个外接硬盘同时挂载时，优先使用含 `.cloud_home_marker` 的硬盘

### 硬盘目录结构

```
{mount_path}/
└── cloud_home_data/
    ├── cloud_home.db          # SQLite 数据库（随盘携带）
    ├── users/
    │   └── {user_id}/
    │       └── {uuid[0..2]}/  # 按 UUID 前两位分区
    │           └── {file_id}
    └── .cloud_home_marker
```

物理路径：`{mount_path}/cloud_home_data/users/{user_id}/{uuid[0..2]}/{file_id}`
用户提供的 `name` 只存数据库，不用于构造物理路径（防路径遍历攻击）。

---

## 回收站自动清理

**文件：** `src/tasks/trash_cleaner.rs`，在 `main.rs` 中 `tokio::spawn` 独立运行。

```
每小时执行一次：
  查询 expires_at <= now() 的所有 trash 记录
  for each:
    删除物理文件（失败只 warn，不中断其他文件的清理）
    DELETE FROM files WHERE id = ?  -- CASCADE 删除 trash 记录
```

---

## 安全设计

1. **账户隔离**：所有文件查询强制带 `AND owner_id = ?`（从 JWT Claims 取），防 IDOR
2. **路径遍历防护**：物理路径由服务端基于 UUID 生成，用户输入的 `name` 只存 DB
3. **密码**：Argon2id，memory_cost=65536，iterations=2，parallelism=1
4. **JWT Cookie**：`HttpOnly`（防 XSS）、`SameSite=Lax`（防 CSRF）、`Secure`（生产 HTTPS）、`Max-Age=604800`
5. **CORS**：必须 `allow_credentials(true)` + 明确 `allowed_origins`（不能用 `*`）；前端请求加 `credentials: 'include'`
6. **分享 token**：32 字节 CSPRNG + base64url，256-bit 熵
7. **上传限制**：`tower_http::limit::RequestBodyLimitLayer` 在 handler 前拦截

---

## 关键配置（.env）

```env
SERVER_HOST=127.0.0.1
SERVER_PORT=8080
# macOS: /Volumes | Linux: 程序自动扫描 /media/ 和 /run/media/，此变量仅 macOS 用
VOLUMES_WATCH_PATH=/Volumes
DRIVE_DATA_DIR=cloud_home_data
JWT_SECRET=<openssl rand -base64 64>
JWT_EXPIRES_IN_HOURS=168
MAX_UPLOAD_SIZE_MB=4096
TRASH_RETENTION_DAYS=7
TRASH_CLEANUP_INTERVAL_HOURS=1
RUST_LOG=cloud_home=info,tower_http=debug
CORS_ALLOWED_ORIGINS=http://localhost:3000
PUBLIC_BASE_URL=http://localhost:8080
```

> `DATABASE_URL` 由程序在运行时根据侦测到的硬盘路径自动构造，无需在 .env 中写死。

---

## 实现顺序

1. `Cargo.toml` — 添加所有依赖
2. `config.rs` + `error.rs` — 全局基础
3. `drive/` 模块 — 硬盘侦测，确定 DB 路径
4. `db/` + `migrations/` — 数据库连接池，运行迁移
5. `state.rs` + `main.rs` 骨架 — 组装 AppState，启动 axum
6. `auth/` 模块 + `/auth/*` 端点
7. `handlers/files.rs` + `services/file_service.rs` — 文件 CRUD
8. `handlers/folders.rs` — 资料夹
9. `handlers/search.rs` — 搜索
10. `handlers/trash.rs` + `services/trash_service.rs` — 回收站
11. `handlers/shares.rs` + `handlers/public.rs` + `services/share_service.rs` — 分享链接
12. `tasks/trash_cleaner.rs` — 定时清理
13. `handlers/drives.rs` — 硬盘状态查询
