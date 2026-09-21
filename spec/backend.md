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
tower-http = { version = "0.7", features = ["cors", "trace", "limit", "request-id", "catch-panic"] }

# 异步运行时
tokio = { version = "1", features = ["full"] }
tokio-util = { version = "0.7", features = ["io"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 数据库
sqlx = { version = "0.9", features = ["sqlite", "runtime-tokio", "migrate", "chrono", "uuid"] }

# 认证
jsonwebtoken = "10"
argon2 = "0.5"

# UUID / 时间 / 随机数
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
rand = "0.10"
base64 = "0.22"

# 错误处理
thiserror = "2"
anyhow = "1"

# 环境变量
dotenvy = "0.15"

# 文件系统监控（跨平台：macOS FSEvents / Linux inotify，notify 自动选择后端）
notify = "8"
libc = "0.2"

# MIME 类型检测
mime_guess = "2"

# 日志
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
# 按大小滚动日志文件（tracing-appender 内建滚动仅支持按时间）
file-rotate = "0.8"
```

---

## 项目目录结构

```
cloud_home/
├── Cargo.toml
├── .env                              # 本地环境变量（git ignore）
├── .env.example                      # 模板（纳入 git）
├── migrations/
│   ├── 20240101000001_create_users.sql   # users + user_identities + otp_codes
│   ├── 20240101000002_create_drives.sql
│   ├── 20240101000003_create_files.sql
│   ├── 20240101000004_create_trash.sql
│   └── 20240101000005_create_shares.sql
└── src/
    ├── main.rs           # 入口：组装 AppState、启动服务和 trash_cleaner 任务
    ├── config.rs         # 从 .env 加载的配置结构体
    ├── error.rs          # AppError 枚举，实现 IntoResponse
    ├── logging.rs        # 日志初始化（双路输出）、audit! 宏、git 风格改名记法
    ├── state.rs          # AppState { db, config, drive_manager }
    ├── db/
    │   └── mod.rs        # SqlitePool 初始化，运行 sqlx migrate
    ├── models/
    │   ├── user.rs       # User, UserIdentity, 各类 DTO 与 UserResponse
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
    │   ├── me.rs         # /me/*（显示名、密码、标识增改删与验证）
    │   ├── admin.rs      # /admin/users*（共享口令 + 内网限制，AdminGuard 提取器）
    │   ├── files.rs      # /files/*
    │   ├── folders.rs    # /folders
    │   ├── search.rs     # /search
    │   ├── trash.rs      # /trash/*
    │   ├── shares.rs     # /shares/*
    │   ├── public.rs     # /public/shares/:token（无 auth）
    │   └── drives.rs     # /drives
    ├── services/
    │   ├── file_service.rs   # 文件业务逻辑（上传/下载/移动）
    │   ├── user_service.rs   # 用户与身份（凭证/标识）、账户软删除与还原
    │   ├── otp_service.rs    # 验证码签发/投递/校验（投递目前为日志占位）
    │   ├── trash_service.rs  # 软删除、恢复、清理
    │   └── share_service.rs  # token 生成与验证
    └── tasks/
        └── trash_cleaner.rs  # tokio::spawn 定时任务，每小时清理过期文件
```

---

## 数据库 Schema

### migrations/20240101000001_create_users.sql

账户基础三张表。`users` 只存个人资料，凭证与标识都在 `user_identities`：

```sql
CREATE TABLE users (
    id         TEXT PRIMARY KEY,
    username   TEXT NOT NULL,           -- 显示名：可变、不唯一
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    is_active  INTEGER NOT NULL DEFAULT 1,
    deleted_at TEXT                     -- 账户被运维删除的时刻，同时是回收站批次标记
);
CREATE INDEX idx_users_username ON users(username);

CREATE TABLE user_identities (
    id          TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL,
    provider    TEXT NOT NULL,          -- 'password' | 'email' | 'phone' | 'google' | 'apple'
    identifier  TEXT NOT NULL,          -- email | E.164 手机号 | 第三方 subject
    secret      TEXT,                   -- 仅 'password' 存 argon2 哈希
    verified_at TEXT,                   -- 经 OTP 或第三方确认后写入
    managed_by  TEXT,                   -- NULL = 用户自有可编辑；'google'|'apple' = 第三方托管只读
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,
    UNIQUE (provider, identifier),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE otp_codes (
    id          TEXT PRIMARY KEY,
    user_id     TEXT,                   -- 可空：为将来手机注册 / 登录预留
    purpose     TEXT NOT NULL,          -- 'identifier_old' | 'identifier_verify'
    destination TEXT NOT NULL,
    code_hash   TEXT NOT NULL,          -- argon2 哈希，绝不明文存储
    attempts    INTEGER NOT NULL DEFAULT 0,
    expires_at  TEXT NOT NULL,
    consumed_at TEXT,
    created_at  TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

几个必须记住的取舍：

- **`username` 刻意没有 `UNIQUE`**：它只是显示名，随时可改。顺带一提，
  SQLite 的列级 `UNIQUE` 由隐式索引支撑、无法单独 DROP——一旦加上，日后想去掉
  就只能整表重建。这里从一开始就不加。
- **`password` 与 `email` 是两行**：密码属于账户，邮箱是可更换的标识。拆开之后
  换邮箱不会碰到密码哈希。`password` 行的 `identifier` 只作留痕，查找一律走
  `email` / `phone` 行。
- **`managed_by` 按「每条标识」记录归属，而非按账户**。同一账户可两者兼有：
  先用邮箱注册、之后绑定 Google 的用户，其邮箱仍归自己编辑。若按账户判定
  「有 google 标识就全部只读」，会把用户自己建的标识错误冻结。
- **`users.deleted_at` 是回收站批次标记**：运维删除账户时，该时刻同时写入
  `users.deleted_at` 与本次产生的每一条 `trash.deleted_at`。还原时据此只还原
  「随删除一起进来的」项目，用户先前自己删掉的东西保持在回收站里。

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
| POST | /auth/register | 无 | 注册账户（`email` 与 `phone` 至少给一个），设置 auth_token Cookie，同时返回 token 字段 |
| POST | /auth/login    | 无 | 登录，`identifier` 可为邮箱或手机号，设置 auth_token Cookie，同时返回 token + expires_at |
| POST | /auth/logout   | JWT | 清除 auth_token Cookie（`Set-Cookie: auth_token=; Max-Age=0`） |
| GET  | /auth/me       | JWT | 获取当前用户信息（含各标识的 verified / managed_by 与 has_password） |

**用户名不唯一**，它只是可随时修改的显示名；登录一律靠邮箱或手机号。
JWT 的 Claims 里**不带用户名**——`AuthUser` 提取时本就要查库确认账户仍启用，
顺带取回当前用户名即可，改名能立刻反映到日志里，不存在过时副本。

**注册不做归属校验**（开发期取舍）：可以填入任意邮箱 / 手机号，落库时
`verified_at` 为空。这带来两个后果，都是有意接受的：

- 未验证的标识同样占用 `UNIQUE(provider, identifier)` 名额，理论上可抢占他人地址。
  家庭局域网场景可接受；真要对公网开放时应先补上注册验证——`verified_at`
  与 `purpose` 词表都已就位，属于纯增量改动，无需迁移。
- 填错的标识**改不回来**：改标识需要旧通道验证码，而验证码发不到一个不存在的地址。
  逃生出口是运维删除该账户后重新注册（见下方运维端点）。

### 个人资料

| 方法 | 路径 | 认证 | 说明 |
|------|------|------|------|
| PATCH  | /me                            | JWT | 修改显示名（非登录标识，无需二次验证） |
| POST   | /me/password                   | JWT | 修改密码（需当前密码） |
| POST   | /me/identifier/challenge       | JWT | 向「现有标识」发送验证码，用于随后的增改删 |
| PUT    | /me/identifier                 | JWT | 新增或修改邮箱 / 手机号 |
| DELETE | /me/identifier                 | JWT | 删除邮箱 / 手机号（至少须保留一个） |
| POST   | /me/identifier/verify/request  | JWT | 向自己已登记的标识发送验证码 |
| POST   | /me/identifier/verify          | JWT | 提交验证码，置 `verified_at` |

**标识变更的两个因素，缺一不可**（这条规则不直观，容易在日后被「简化」掉，故记在此）：

1. 当前密码；
2. 发往**现有标识**的验证码。

第 2 条才是关键：只验密码的话，密码一旦泄露，攻击者就能把邮箱换成自己的，
从而永久接管账户；验证旧通道能证明请求者确实持有这个账户。手机与邮箱同等对待——
旧手机作为持有证明并不比旧邮箱弱。它同时堵住了迂回路径：否则可以先凭密码删掉邮箱、
再添加自己的，两步达成同样的接管（所以**删除**也要求旧通道验证码）。

**「证明新值」与「设置新值」是解耦的**：新标识一律以未验证状态落库，用户随时可用
上面的 verify 端点把它标记为已验证。验证码按 `purpose` 隔离，
`identifier_old` 的码不能拿去当 `identifier_verify` 用。

OTP 限制：6 位数字、5 分钟有效、单码最多 5 次尝试、同一目标每小时最多 3 条。
**投递目前是开发期占位**：没有 SMTP / SMS 通道，验证码直接写进审计日志
（`otp.issued`），接入真实发送方时只需替换 `otp_service::deliver` 一个函数。

### 第三方登录（Google / Apple）的约定

OAuth 登录本身（客户端凭证、令牌交换、回调路由）尚未实现，此处先固定契约与支撑它的 schema：

- 第三方托管的标识（`managed_by` 非空）在本地**只读**：`/me/identifier` 一律返回 403，
  提示去对应平台修改；纯第三方账户也不需要密码，`/me/password` 同样 403。
- **每次 OAuth 登录都按第三方响应同步资料**：`username` 每次覆盖，
  `managed_by = <该 provider>` 的标识行随之更新；`managed_by IS NULL` 的行绝不碰。
  （副作用：在个人资料里改的显示名，只能保留到该用户下次第三方登录为止。）
- **账户关联**：若第三方返回的、已验证的邮箱能对上某个账户的 `email` 标识，
  就把 `google`/`apple` 标识挂到**那个账户**上，而不是新建一个——否则用户会多出一个
  空账户，看起来文件全没了。
- **第三方邮箱必须是已验证的**。若对方未声明邮箱已验证（或 Apple 返回私密转发地址），
  **明确拒绝登录并说明原因**，不要静默改为新建账户、也不要隐瞒理由；
  拿未经证实的地址做关联，正是上面用旧通道验证码堵掉的那条接管路径。
- **同步撞名**：第三方邮箱已属于另一个账户时，跳过该字段、记一条 `audit!` 警告、
  放行登录——不该因为一个资料字段把人挡在自己的文件之外。

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

### 重名处理策略

同目录下、未删除项目之间的 `name` 有唯一性约束（`idx_files_unique_name`）。冲突时的行为依操作而异：

| 操作 | 冲突时行为 |
|------|-----------|
| 上传（`POST /files/upload`）、建立资料夹（`POST /folders`） | **自动改名**：仿照操作系统复制文件的做法，在扩展名前插入 `(n)`（n 从 1 起递增直到无冲突），如 `photo.jpg` → `photo(1).jpg`；资料夹直接附加于名称末尾，如 `Documents` → `Documents(1)`。永不失败。 |
| 重命名（`PATCH /files/:id`）、移动（`POST /files/:id/move`） | **拒绝**，返回 `409 Conflict`。这是用户的显式操作，静默改名会造成困惑，交由前端提示用户另取名称。 |
| 回收站还原（`POST /trash/:id/restore`） | **自动改名**，追加 `(已还原 <时间戳>)` 后缀，如 `report.pdf (已还原 20260101120000)`；同样永不失败。 |

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
8. **令牌不等于账户仍可用**：`AuthUser` 提取时在验签之后**必须再查一次库**确认
   `is_active = 1`。签名有效只说明令牌没被篡改；账户被运维删除后，其旧令牌在过期前
   （默认 7 天）依然能通过验签——不查库的话，被删用户可以继续操作，甚至把自己的文件
   从回收站里还原出来。这次查询顺带取回当前用户名，写入请求 span。

### 运维端点（删除 / 还原账户）

| 方法 | 路径 | 说明 |
|------|------|------|
| GET  | /admin/users         | 列出所有账户（含已停用）及其标识 |
| POST | /admin/users/delete  | 软删除账户 |
| POST | /admin/users/restore | 还原账户 |

**这不是权限系统**：没有管理员角色，只有一个共享口令加一层来源限制。三道关卡按序执行，
且都在读取请求体之前（`AdminGuard` 提取器）：

1. 未配置 `ADMIN_PASSWORD` → `404`（功能没开，就当这条路由不存在）；
2. 调用方 IP 非回环 / 内网 → `403`，并记 `reason="remote_caller"`；
3. `X-Admin-Password` 缺失或不符 → `401`（定长比较，避免按字节提前返回泄露前缀）。

口令与请求头的值**绝不写入日志**，`Config` 的手写 `Debug` 也对其脱敏。

> 来源限制的局限要正视：反向代理在前时，对端 IP 就是网关自己的回环地址，内网判定会
> 形同虚设，除非开启 `TRUST_PROXY_HEADERS` 并信任 `X-Forwarded-For`。它是给局域网机器
> 加的一层防护，不能替代口令本身。

**定位账户**用请求体里的 `identifier`（匹配**任意** provider：邮箱、手机号，
或 Google/Apple subject——纯第三方账户既无邮箱也无手机号，必须够得着），
放在 body 而非 URL，免得出现在上游访问日志里。由于 `UNIQUE` 是
`(provider, identifier)` 的组合，同一字符串可能在不同 provider 下属于不同账户，
命中多个时返回 `409`，要求用可选的 `provider` 消歧——绝不猜测删哪一个。

**删除的语义**：该用户所有未删除文件一次性进回收站（共用同一 `deleted_at` 作为批次标记），
账户置 `is_active = 0`。**用户行永不物理删除**，回收站清理任务到期后只清文件，
因此保留期过后还原会得到一个可用但空的账户。分享链接刻意不动——
`share_service::resolve_public` 本就要求文件 `is_deleted = 0`，
所以链接在文件进回收站的瞬间自然失效，还原后又自动恢复，且每条链接原本的
`is_active` 不受影响。

---

## 日志与审计

**文件：** `src/logging.rs`（初始化、`audit!` 宏、`git_rename_notation` 辅助函数）。

### 双路输出

| Sink | 格式 | 用途 |
|------|------|------|
| 控制台（stdout） | `LOG_FORMAT`（默认 `pretty`，可选 `json`） | 本地 `cargo run` 时人眼阅读 |
| 文件（`LOG_DIR/cloud_home.log`） | 恒为 JSON | 供云端日志采集/监控代理（Filebeat/Promtail/Datadog Agent 等）读取 |

文件按**大小**滚动（`file-rotate` crate；`tracing-appender` 内建滚动仅支持按时间，不适用）：超过 `LOG_MAX_SIZE_MB`（默认 1）即把当前文件整体改名为 `cloud_home.log.1`，重新开始写入空文件；只保留 1 个备份，总占用约 2×`LOG_MAX_SIZE_MB`。保留 1 个备份而非直接原地截断，是为了让日志采集方有机会读完被滚动出去的旧文件而不丢行——原地截断在采集方读取滞后时会造成静默丢失。

日志目录默认 `./logs`（本地磁盘，非外接硬盘）——外接硬盘的消失正是最需要被记录的事件之一，若日志本身写在盘上，卸载瞬间的日志就可能连带丢失。

### target 与过滤

三个自定义 target：

| target | 内容 |
|--------|------|
| `audit` | 谁做了什么（登录/上传/删除…）——审计主线索 |
| `http` | 访问日志，每个请求一行（`event="http.request"`，含 request_id、status、latency_ms） |
| `cloud_home::*` | 普通诊断日志 |

无 `RUST_LOG` 时默认过滤：`cloud_home=info,http=info,audit=info,tower_http=warn`。访问日志由本项目自己在 `http` target 发出，不再依赖 `tower_http` 内建 span 的 debug 级别输出。

### 请求关联

`SetRequestIdLayer` 在无 `x-request-id` 请求头时生成 UUID（`MakeRequestUuid`，复用既有 `uuid` 依赖），`PropagateRequestIdLayer` 写回响应头，上游反向代理传入的 id 会被原样保留。`TraceLayer::make_span_with` 建立 `http` span，携带：

- `request_id`
- `method`
- `route`：**路由模板**（如 `/api/v1/public/shares/{token}`），**绝不用原始路径**——分享 token 等敏感值出现在路径中，写入外发日志文件等同泄露
- `client_ip`：`TRUST_PROXY_HEADERS=true` 时取 `X-Forwarded-For`/`X-Real-IP` 首跳，否则取 TCP 对端地址（`ConnectInfo`，默认 `false`，防伪造）
- `user_id`、`username`：初始为空，由 `AuthUser` 提取器在验证 JWT 成功后用 `Span::current().record(...)` 回填

JWT 的 `Claims` 因此新增 `username` 字段（`#[serde(default)]` 兼容改动前签发的旧 token，7 天内自然过期淘汰），使得同一请求内的所有日志行——包括 `error.rs` 中原本拿不到用户上下文的一条——都自动带上可读身份，无需每个调用点手动传递。

> **JSON 形状提示：** span 字段嵌套在 `span` 对象下，不会展平进事件本身。示例：
> ```json
> {"target":"audit","event":"file.upload","result":"success","file_id":"…",
>  "span":{"request_id":"…","user_id":"…","username":"…","route":"…","name":"http"}}
> ```
> 下游查询用 `span.request_id` / `span.user_id`。审计调用点在身份是事件核心时（登录事件本身，以及任何可能脱离请求 span 发出的行，如后台任务）仍显式再传一份 `user_id`/`username`。

### 字段约定

- `event`：`领域.动作`（如 `file.upload`、`auth.login`），点号分隔，段内小写下划线。
- `result`：`"success"` | `"failure"` | `"started"`（流式响应：已授权并开始，非「字节已全部送达」，见下）。
- 失败行附加 `reason`，取自封闭词表：`unknown_user`、`bad_password`、`token_missing`、`token_invalid`、`duplicate_username_or_email`、`not_owner`、`share_expired`、`download_disabled`、`drive_unavailable`。
- **可读优先于晦涩**：有 id 就尽量同时给人类名字——`file_name` 伴 `file_id`，`username` 伴 `user_id`。
- **改名/移动用 git 风格记法**：`change` 字段用 `git diff --stat` 式的重命名记法渲染（`git_rename_notation`），折叠公共前后缀，差异中段用花括号——如 `Documents/2025/{draft.txt => final.txt}`、`{Documents/2025 => Archive}/photo.jpg`。`old_path`/`new_path` 离散字段同时记录，供机器查询；`change` 是给人看的渲染结果。
- 字段名统一：`user_id`、`username`、`file_id`、`file_name`、`file_path`、`old_path`/`new_path`、`change`、`file_type`、`size_bytes`、`parent_id`、`old_parent_id`/`new_parent_id`、`count`、`freed_bytes`、`share_id`、`share_token_prefix`（仅前 8 字符）、`owner_id`。
- 级别：成功 → info；用户导致的失败（401/403/409/410）→ warn；服务端失败 → error；常规 404/400 → debug（避免淹没审计信号）。
- **绝不记录**：密码、`password_hash`、`jwt_secret`、完整 JWT、完整分享 token、`/public/shares/*` 的原始路径、搜索关键字原文。

### 事件目录

| event | 位置 | 关键字段 |
|---|---|---|
| `auth.register` success | `handlers/auth.rs` | `user_id`, `username` |
| `auth.register` failure | 同上（Conflict 分支） | `reason="duplicate_username_or_email"`, `username` |
| `auth.login` failure（用户不存在） | 同上 | `reason="unknown_user"`, `identifier` |
| `auth.login` failure（密码错误） | 同上 | `reason="bad_password"`, `user_id`, `username` |
| `auth.login` success | 同上 | `user_id`, `username` |
| `auth.logout` | 同上 | `user_id` |
| `auth.token_rejected` | `auth/extractor.rs`（`token_missing`）、`auth/jwt.rs`（`token_invalid`） | warn, `reason` |
| `file.upload` | `handlers/files.rs` | `file_id`, `file_name`, `file_path`, `size_bytes` |
| `file.download` | 同上 | `file_id`, `file_name`, `file_path`, `size_bytes` |
| `file.raw` | 同上 | `file_id`, `file_name`, `size_bytes`——**刻意不解析路径**（媒体拖动每次 seek 都是新请求，避免每次都触发一次路径回溯查询） |
| `file.rename` | 同上 | `file_id`, `change`, `old_path`, `new_path` |
| `file.move` | 同上 | `file_id`, `change`, `old_path`, `new_path` |
| `file.delete`（软删除） | 同上 | `file_id`, `file_name`, `file_path`, `size_bytes` |
| `file.download_archive` | 同上 | `count`, `format`；`result="started"`（打包在独立 task 中进行，handler 不会收到完成信号） |
| `folder.create` | `handlers/folders.rs` | `file_id`, `file_name`, `file_path` |
| `trash.restore` | `handlers/trash.rs` | `file_id`, `file_name`, `file_path` |
| `trash.delete`（永久删除） | 同上 | `trash_id`, `freed_bytes` |
| `trash.empty` | 同上 | `count`, `freed_bytes` |
| `share.create` | `handlers/shares.rs` | `share_id`, `file_id`, `share_token_prefix`, `expires_at`, `can_download` |
| `share.revoke` | 同上 | `share_id` |
| `public.share_view` | `handlers/public.rs`（访问者匿名） | `share_token_prefix`, `owner_id`, `file_id` + span 的 `client_ip` |
| `public.share_download` | 同上 | 同上 + `size_bytes`；失败 `reason="download_disabled"` |
| `drive.attached` / `drive.detached` | `tasks/drive_monitor.rs` | `mount` |
| `task.trash_cleanup` | `tasks/trash_cleaner.rs` | `count` |
| `process.panic` | panic hook（`logging.rs`） | 自动带当前请求 span（若有） |

**流式响应完成度说明：** `download`/`raw`/`public.share_download` 返回的是 `ReaderStream`；`download_archive` 把打包工作 `tokio::spawn` 到独立 task 后立即返回响应头。「handler 返回 Ok」的含义是「传输已授权并开始」，不是「字节已全部送达」——响应体是流式的，handler 返回时并不知道客户端是否真正读完。单文件流按 `"success"` 记（审计意义在于「已被授权拿到这些字节」），批量打包按 `"started"` 记。字节级送达确认不在本设计范围内。

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
RUST_LOG=cloud_home=info,http=info,audit=info,tower_http=warn
CORS_ALLOWED_ORIGINS=http://localhost:3000
PUBLIC_BASE_URL=http://localhost:8080

# 日志文件目录（本地磁盘，非外接硬盘）
LOG_DIR=./logs
# 控制台格式：pretty（默认，人类可读）| json。文件恒为 json。
LOG_FORMAT=pretty
LOG_FILE_ENABLED=true
# 单个日志文件大小上限（MB），超过即滚动，保留 1 个备份（cloud_home.log.1）
LOG_MAX_SIZE_MB=1
# 是否信任反向代理的 X-Forwarded-For / X-Real-IP（生产上 nginx/Caddy 之后再开启）
TRUST_PROXY_HEADERS=false
# 运维端点（/admin/users）的共享口令；留空 = 这些端点一律 404（功能关闭）。
# 另有一层「调用方须来自回环或内网」的限制，注意其在反向代理之后会失效。
ADMIN_PASSWORD=
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
