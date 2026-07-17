# Cloud Home

家庭云端存储服务，目标是替代 Google Drive。后端用 Rust（axum + SQLite）实现，通过 RESTful API 管理外接硬盘上的文件，支持多账户隔离、回收站（7 天保留期）、文件分享链接。

完整后端规范见 [`spec/backend.md`](spec/backend.md)。

## 功能特性

- 账户注册 / 登录（JWT，HttpOnly Cookie + `Authorization: Bearer` 双模式）
- 文件与资料夹的上传、下载、重命名、移动、列目录、搜索
- 回收站软删除，保留 7 天后由定时任务自动清理
- 分享链接（公开访问，可设过期时间与是否允许下载）
- 自动侦测外接硬盘挂载（macOS `/Volumes`、Linux `/media` 与 `/run/media`），数据库与文件随盘携带
- 账户隔离：用户只能看见自己的文件；未挂载硬盘时写操作返回 `503`

## 环境要求

- Rust 1.97+（`edition = "2024"`）— 建议用 [rustup](https://rustup.rs/) 安装
- 一个外接硬盘（生产用）；本地开发可用兜底目录，见下文

## 安装

```sh
git clone <repo-url> cloud_home
cd cloud_home

# 复制环境变量模板并按需修改
cp .env.example .env

# 生成一个安全的 JWT 密钥填入 .env 的 JWT_SECRET
openssl rand -base64 64
```

编辑 `.env`，至少设置 `JWT_SECRET`。其余配置项含义见 [`.env.example`](.env.example) 与 `spec/backend.md`。

> 注意：`DATABASE_URL` 无需配置，程序会在运行时根据侦测到的硬盘路径自动构造。

## 构建

```sh
cargo build            # 开发构建
cargo build --release  # 发布构建（产物在 target/release/cloud_home）
```

## 运行

### 生产（自动侦测外接硬盘）

插上外接硬盘后直接运行，程序会扫描挂载点并把数据写入 `{挂载点}/cloud_home_data/`：

```sh
cargo run
# 或使用发布产物
./target/release/cloud_home
```

服务默认监听 `http://127.0.0.1:8080`，API 基础路径为 `/api/v1`。

### 本地开发（无外接硬盘）

未侦测到外接硬盘时，程序依次回退到环境变量 `DEV_DRIVE_PATH`，再到当前目录下的 `./dev_drive`：

```sh
JWT_SECRET=dev_secret_change_me DEV_DRIVE_PATH=/tmp/ch_drive cargo run
```

## 快速验证

```sh
BASE=http://127.0.0.1:8080/api/v1

# 健康检查
curl $BASE/health

# 注册（返回 token，并设置 auth_token Cookie）
curl -X POST $BASE/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"password123"}'

# 用返回的 token 访问当前用户信息
curl $BASE/auth/me -H "Authorization: Bearer <token>"
```

完整 API 列表见 [`spec/backend.md`](spec/backend.md)。

## 开发命令

```sh
cargo fmt      # 格式化
cargo clippy   # Lint
cargo test     # 运行测试
```

## 前端

`ui/`（Next.js）稍后实现，当前阶段后端优先。
