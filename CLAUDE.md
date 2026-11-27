# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Cloud Home** — 家庭云端存储服务，目标是替代 Google Drive。后端用 Rust 实现，通过 RESTful API 管理外接硬盘上的文件，支持多账户隔离、回收站（7 天保留期）、文件分享链接。

完整后端规范见 [`spec/backend.md`](spec/backend.md)，包含：技术栈、目录结构、数据库 Schema、所有 API 接口定义、外接硬盘侦测方案（macOS + Linux）、安全设计、.env 配置。

## Common Commands

- **Build**: `cargo build`
- **Run**: `cargo run`
- **Run with Release optimizations**: `cargo build --release` then `./target/release/cloud_home`
- **Lint/Check**: `cargo clippy`
- **Format**: `cargo fmt`
- **Test**: `cargo test`

## Project Structure

```
cloud_home/
├── spec/backend.md       # 后端完整规范（API、Schema、架构决策）
├── src/                  # Rust 后端源码
├── migrations/           # sqlx 数据库迁移文件
├── ui/                   # Vite + React + TypeScript 前端（单页应用，无 SSR / 无 SEO）
├── Cargo.toml
└── .env                  # 本地环境变量（git ignore）
```

## Key Decisions

- **Web 框架**: axum 0.8（tokio 官方生态，tower 中间件体系）
- **数据库**: SQLite via sqlx，存放在外接硬盘上（随盘携带）
- **认证**: JWT，存放在 HttpOnly Cookie 中；同时支持 `Authorization: Bearer` header
- **外接硬盘侦测**: macOS 扫描 `/Volumes/`，Linux 扫描 `/media/` 和 `/run/media/`，均使用 `notify` crate 实时监控
- **一体机**: 单个 `cloud_home` 二进制同源提供 API 与 Web UI（UI 编译进二进制，R-026）；UI 只给家人朋友用，不公开、无 SEO。UI 属于产品版本的一部分，每次 OTA 更新连同后端一起替换、一起回滚。
- **错误处理**: `thiserror` 定义 `AppError`，实现 `IntoResponse`

## 代码准则：只保留正在使用的代码

目标：不累积遗留代码。仓库里只有「现在在用」的那一份实现；回退靠 git（历史、tag、发布），不靠代码里的残留。

1. **需求变了就地重写，不并存。** 改端点、函数、类型、配置项、表结构的用法时，直接改写原处。禁止：`foo_v1`/`foo_v2`、`foo_old`/`foo_new`、并行的 `/v1` 与 `/v2` 路径、同时读取新旧字段或新旧环境变量、用开关或 feature flag 保留旧逻辑、「以防回滚」的备份实现、兼容垫片（shim）。
2. **零闲置代码。** 没有调用者的函数、方法、类型、模块、路由、环境变量/配置项、依赖（crate / npm 包）、文件、测试夹具，一律删除。
   - 不用 `#[allow(dead_code)]`、`#[allow(unused)]`、`_` 前缀变量、`#[cfg(any())]` 来掩盖未使用的东西；不留注释掉的代码；不写「以后可能用得上」的预留代码。
   - 新代码与它的**第一个调用者**在同一次提交里出现；没有调用者就不写。
3. **改动即清理。** 同一次提交里删掉因这次改动而失效的一切：旧调用方、旧测试、旧文档（`spec/`、`README.md`、`.env.example`、OpenAPI 注解、`CHANGELOG.md`）、旧依赖。重命名或删除后，全仓库搜索残留引用（后端、前端、脚本、文档）。
4. **破坏性变更不留兼容层。** 直接改成新行为，所有调用方（含 `ui/`）在同一次改动里更新，并在 `CHANGELOG.md` 标 **Breaking**。后端与 UI 同包发布、一起 OTA 更新，所以不需要为旧客户端保留旧接口。
5. **唯一例外：已应用的数据库迁移。** `migrations/` 是只追加的账本（设备上的数据库已按它升级过），不改写、不删除；需要删列/表就新增一个迁移。但读写旧列/旧表的代码必须在同一次提交里删除。
6. **提交前自查。** `cargo clippy --all-targets` 没有 `dead_code` / `unused` 类警告；前端 `bun run lint` 与类型检查通过且无未使用的导出/依赖；`grep -rniE '_v[0-9]\b|legacy|deprecated|_old\b|todo.*remove'` 没有新增命中。

## Agent team

Requirements live in [`SPEC.md`](SPEC.md) (single source of truth). Work is run by a manager (`scripts/run_manager.sh`)
that delegates to the subagents in `.claude/agents/`: `backend-engineer`, `frontend-engineer`, `designer`, `tester`.
Progress is tracked in `plan/STATUS.md` and `reports/`. Agents commit locally on `agent/<R-id>-<slug>` branches, never push or tag.
