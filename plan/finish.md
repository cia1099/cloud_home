# Cloud Home — 已完成模块总览

> 截至 2026-11-18，基于 `CHANGELOG.md`、git 历史与 `src/handlers/mod.rs` 中实际注册的路由整理。
> Base URL：`/api/v1`。「JWT」= Cookie `auth_token` 或 `Authorization: Bearer`。

**当前状态：后端 API 已全部实现（v0.3.0 + Unreleased）。** 前端 `ui/` 尚未提交（git 中为 untracked）。

## 里程碑时间线

| 日期 | 提交 | 内容 |
|------|------|------|
| 2026-07-17 | `d6c2072` | 核心后端：多账户、文件/资料夹、回收站、分享、搜索、硬盘侦测 |
| 2026-07-17 | `f9e95e8` / `14c200a` | 依赖升级到最新大版本；README 安装/运行说明 |
| 2026-09-08 | `6e0b3e5` | OpenAPI 交互文档（Scalar，`/docs`） |
| 2026-09-09 | `e9864ac` / `95b136b` | 文件预览（Range）、批量下载（ZIP，后加 tar.gz） |
| 2026-09-11 | `ded3367` | SSE 实时事件、用量统计 |
| 2026-09-14 | `2a993c4` / `42c824c` | 音视频浏览器内播放；发布 **v0.2.0** |
| 2026-09-15 | `1b695f2` | 结构化日志 + 审计 |
| 2026-09-15 | `905b488` | 上传 / 建资料夹重名自动加序号 |
| 2026-09-21 | `b3cea10` | 凭证迁入 `user_identities`、可编辑个人资料、OTP、运维端点；**v0.3.0** |
| 2026-09-21 | `b2395b6` | `POST /admin/trash/purge`（Unreleased，尚未打版本） |

---

## 逐端点完成情况

### 系统
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /health | 无 | ✅ | |
| GET | /docs | 无 | ✅ | Scalar OpenAPI UI（v0.2.0） |

### 认证 `auth`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| POST | /auth/register | 无 | ✅ | email / phone 至少一个；设置 Cookie；注册不验证归属（有意取舍） |
| POST | /auth/login | 无 | ✅ | v0.3.0 起 `identifier` 为邮箱或手机号，**不再支持用户名** |
| POST | /auth/logout | JWT | ✅ | 清除 Cookie |
| GET | /auth/me | JWT | ✅ | 返回 email/phone/has_password 及各标识 verified/managed_by |

### 个人资料 `me`（v0.3.0）
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| PATCH | /me | JWT | ✅ | 改显示名（非唯一） |
| POST | /me/password | JWT | ✅ | 改密码 |
| POST | /me/identifier/challenge | JWT | ✅ | 向现有标识发送一次性验证码 |
| PUT | /me/identifier | JWT | ✅ | 新增/更换邮箱或手机；需当前密码 + 验证码 |
| DELETE | /me/identifier | JWT | ✅ | 移除标识（至少保留一个） |
| POST | /me/identifier/verify/request | JWT | ✅ | 请求验证新标识的验证码 |
| POST | /me/identifier/verify | JWT | ✅ | 提交验证码，标记 verified |

⚠️ 验证码**投递仍是开发占位**（只写入审计日志），真实 SMTP/SMS 发送器尚未接入。

### 硬盘 `drives`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /drives | JWT | ✅ | 列出侦测到的外接盘与 active_drive；后台 `drive_monitor` 实时监控（macOS `/Volumes`，Linux `/media`、`/run/media`） |

### 文件 `files`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /files | JWT | ✅ | 列目录，`?parent_id=` |
| POST | /files/upload | JWT | ✅ | multipart；重名自动 `name(1).ext` |
| GET | /files/{id} | JWT | ✅ | 元数据 |
| GET | /files/{id}/download | JWT | ✅ | 流式下载，Range（206） |
| GET | /files/{id}/raw | JWT | ✅ | 内联预览，Range；`.m4a/.m4v` MIME 已校正，可直接给 `<audio>/<video>` |
| GET | /files/{id}/thumbnail | JWT | ✅ | 图片缩略图（JPEG，磁盘缓存；尺寸限 64/128/256/512/1024） |
| PATCH | /files/{id} | JWT | ✅ | 重命名；重名返回 409 |
| POST | /files/{id}/move | JWT | ✅ | 移动；重名返回 409 |
| DELETE | /files/{id} | JWT | ✅ | 软删除进回收站（资料夹递归） |
| POST | /files/download | JWT | ✅ | 批量打包下载，流式 ZIP 或 tar.gz（`format`），可含资料夹 |

### 资料夹 `folders`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| POST | /folders | JWT | ✅ | 重名自动加序号 |

### 搜索 `search`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /search | JWT | ✅ | 分页；结果带 `parent_path`（CTE 递归拼出） |

### 回收站 `trash`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /trash | JWT | ✅ | 含 expires_at |
| POST | /trash/{id}/restore | JWT | ✅ | 原目录已删则回根目录；重名追加 `(已还原 时间戳)` |
| DELETE | /trash/{id} | JWT | ✅ | 立即永久删除单项 |
| DELETE | /trash | JWT | ✅ | 清空，返回数量与释放空间 |
| — | （后台任务） | — | ✅ | `trash_cleaner` 定时清理，7 天保留期 |

### 分享 `shares`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| POST | /shares | JWT | ✅ | `file_id`、`can_download`、`expires_at` |
| GET | /shares | JWT | ✅ | 列出自己的分享 |
| DELETE | /shares/{id} | JWT | ✅ | 撤销（`is_active=0`） |
| GET | /public/shares/{token} | 无 | ✅ | 分享信息（含 `shared_by`）；404 / 410 |
| GET | /public/shares/{token}/download | 无 | ✅ | 流式下载，`access_count` +1；can_download=false → 403 |

### 用量与实时事件 `usage`
| 方法 | 路径 | 认证 | 状态 | 备注 |
|------|------|------|------|------|
| GET | /usage | JWT | ✅ | 账户 / 磁盘用量 |
| GET | /usage/breakdown | JWT | ✅ | 按类别分解 |
| GET | /events/stream | JWT（Cookie） | ✅ | SSE：`usage` 与 `files` 事件；30 秒兜底重算 |

### 运维 `admin`（共享口令 `ADMIN_PASSWORD` + 回环/内网来源限制）
| 方法 | 路径 | 状态 | 备注 |
|------|------|------|------|
| GET | /admin/users | ✅ | 列出所有账户（含已停用）及标识 |
| POST | /admin/users/delete | ✅ | 软删除：文件进回收站并停用账户，用户行不物理删除 |
| POST | /admin/users/restore | ✅ | 还原，只带回该次删除所移入回收站的内容 |
| POST | /admin/trash/purge | ✅ | 立即清空某账户回收站，**不可逆**（Unreleased） |

---

## 横切能力

- ✅ 多账户隔离（每个账户独立目录与 `get_owned` 归属校验）
- ✅ 认证：JWT，Cookie + Bearer 双模式；每次请求复查账户是否启用（v0.3.0 修复）
- ✅ `user_identities` 表：新增登录方式（Google/Apple）为纯增量改动
- ✅ OTP：6 位、5 分钟有效、每码 5 次尝试、每目的地每小时 3 码、Argon2 哈希存储
- ✅ 结构化日志（控制台 pretty + 按大小滚动 JSON 文件）、请求关联 ID、审计事件
- ✅ SQLite 存于外接盘，随盘携带；配置脱敏（`jwt_secret`、`admin_password`）
- ✅ OpenAPI 文档由 `utoipa` 从路由自动生成

## 尚未完成 / 待办

- ⬜ **OTP 真实投递**：替换占位函数为 SMTP / SMS 发送
- ⬜ **第三方登录（Google / Apple）**：表结构与约定已就位，端点未实现
- ⬜ **注册验证**：目前可填任意邮箱/手机；对公网开放前需补
- ⬜ **前端 `ui/`**（Next.js）：未提交
- ⬜ **自动化测试**：仓库没有 `tests/` 目录，源码内仅少量测试
- ⬜ **发版**：`Unreleased` 中的 `/admin/trash/purge` 尚未打 tag（下一版本，如 v0.3.1 / v0.4.0）
- ⬜ 未跟踪文件：`spec/plan_batch_download_and_image_preview.md`、`wiki.md`、`.claude/` 尚未处理
