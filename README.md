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

### 一体机安装（发布版，无需 Rust）

脚本会取最新发布的清单并用内置 minisign 公钥验签（验不过即中止），再下载匹配本机架构的安装包、校验 sha256，
布置 `/opt/cloud_home`（`versions/<ver>/` + `current` 符号链接，在线更新随即可用），并注册开机自启、退出即重启的服务。
首次安装会生成 `/opt/cloud_home/.env`（随机 `JWT_SECRET`），按需修改后 `systemctl restart cloud_home`（macOS 见下）。

```sh
# Linux（需要 curl、minisign、systemd；创建 cloudhome 用户与 systemd 服务）
sudo bash packaging/linux/install.sh

# macOS（Apple 芯片；brew install minisign；服务以当前登录用户运行，launchd LaunchDaemon）
sudo bash packaging/macos/install.sh
# 修改 .env 后：sudo launchctl kickstart -k system/com.cloudhome.server
```

## 构建

```sh
# 先构建 UI（需要 bun）；发布构建会把 ui/dist 嵌入二进制
(cd ui && bun install --frozen-lockfile && bun run build)

cargo build            # 开发构建（运行时读磁盘上的 ui/dist；没有 ui/dist 也能编译）
cargo build --release  # 发布构建（产物在 target/release/cloud_home）
```

发布构建要求 `ui/dist/index.html` 存在，且 UI 内置版本等于 `Cargo.toml` 的 `version`，否则编译失败。

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

# 注册。默认 REGISTRATION_MODE=approval：
#   - 全新数据库里，来自本机/内网的第一个账户自动批准，返回 token 并设置 auth_token Cookie；
#   - 之后的账户返回 202 {"data":{"status":"pending_approval"}}，没有 token，须管理员批准。
#   REGISTRATION_MODE=open 则注册即启用并返回 token。
curl -X POST $BASE/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"alice","email":"alice@example.com","password":"password123"}'

# 管理员批准（需设置 ADMIN_PASSWORD，且从本机/内网调用）
curl "$BASE/admin/users?status=pending" -H 'X-Admin-Password: <ADMIN_PASSWORD>'
curl -X POST $BASE/admin/users/approve -H 'X-Admin-Password: <ADMIN_PASSWORD>' \
  -H 'Content-Type: application/json' -d '{"identifier":"bob@example.com"}'

# 用返回的 token 访问当前用户信息
curl $BASE/auth/me -H "Authorization: Bearer <token>"
```

完整 API 列表见 [`spec/backend.md`](spec/backend.md)。

## 在聊天里批准注册（可选）

默认 `CHAT_PROVIDER=none`，审批只走上面的 `/admin/*` 端点。设置 `CHAT_PROVIDER=telegram` 与 `CHAT_BOT_TOKEN`
（向 @BotFather 申请的机器人令牌，属于密钥，只放 `.env`）后，新的注册申请会推送给已配对的管理员，
消息上有 **批准 / 拒绝** 按钮。机器人用长轮询，只发起出站 HTTPS 连接，不需要开放任何入站端口。

```sh
# 1. 运维签发一次性配对码（需 ADMIN_PASSWORD，本机/内网；8 位，10 分钟，单次有效）
curl -X POST $BASE/admin/chat/pairing -H 'X-Admin-Password: <ADMIN_PASSWORD>'
# 2. 管理员在 Telegram 里私聊机器人：/pair <code>
# 3. 之后：/pending 查看待批准申请，点按钮批准或拒绝；/whoami、/help
#    运维：/users、/status、/restore <标识>；/delete <标识> 与 /purge <标识>（清空回收站，不可逆）
#    会先回一个确认码，再发 /confirm <码> 才执行（2 分钟内有效，/cancel 放弃）
curl $BASE/admin/chat -H 'X-Admin-Password: <ADMIN_PASSWORD>'            # 提供方、连接状态、已配对管理员
curl -X DELETE $BASE/admin/chat/admins/<id> -H 'X-Admin-Password: <ADMIN_PASSWORD>'   # 取消配对，立即生效
```

只有已配对的管理员能操作；其他人给机器人发任何消息都得不到回应（仅写入审计日志）。
聊天服务不可用时注册照常进入待批准状态，消息留在队列里等恢复后补发，不会隐式批准任何人。
**发给聊天提供方的内容**：申请人的显示名、打码后的邮箱/手机号（`CHAT_MASK_IDENTIFIERS=false` 才发全文）、申请时间、
来源 IP，以及 批准/拒绝 按钮；绝不包含密码、令牌、文件名或文件内容。完整说明与令牌轮换见
[`spec/admin_chat.md`](spec/admin_chat.md) 第 10 节。

## 开发命令

```sh
cargo fmt      # 格式化
cargo clippy   # Lint
cargo test     # 运行测试
```

### Test
token
```
eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJjMzdlN2Q2Ni0xMTljLTRhY2UtOTc1Yi03N2M4ZDQ4MDhlMjEiLCJpYXQiOjE3ODg4NTQzNTEsImV4cCI6MTc4OTQ1OTE1MX0.eHtVho6hV5IbxrSEc7pqdparKaku0KwWpSzUkJ2P8GE

parent_id=c37e7d66-119c-4ace-975b-77c8d4808e21
folder_id="c63a0f00-46ae-49ef-9a2b-f2fb9869c3ae"
```
* upload
```sh
curl /api/v1/files/upload \
  --request POST \
  --header 'Content-Type: multipart/form-data' \
  --cookie 'auth_token=eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJjMzdlN2Q2Ni0xMTljLTRhY2UtOTc1Yi03N2M4ZDQ4MDhlMjEiLCJpYXQiOjE3ODg4NTQzNTEsImV4cCI6MTc4OTQ1OTE1MX0.eHtVho6hV5IbxrSEc7pqdparKaku0KwWpSzUkJ2P8GE' \
  --form 'file=@filename'
```


## 前端

`ui/`（Vite + React + TypeScript）构建后嵌入 `cloud_home` 二进制，与 API 同源提供：打开 `http://<host>:<port>/` 即是 Web UI，
`/api/v1`、`/docs`、`/openapi.json` 不受影响，未知的 `/api/*` 仍返回 JSON 404。开发时用 `bun run dev`（Vite 代理 `/api` 到后端），
或 `bun run build` 后直接 `cargo run`（调试构建读磁盘上的 `ui/dist`）。
  