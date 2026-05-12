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
├── ui/                   # Next.js 前端（稍后实现）
├── Cargo.toml
└── .env                  # 本地环境变量（git ignore）
```

## Key Decisions

- **Web 框架**: axum 0.8（tokio 官方生态，tower 中间件体系）
- **数据库**: SQLite via sqlx，存放在外接硬盘上（随盘携带）
- **认证**: JWT，存放在 HttpOnly Cookie 中；同时支持 `Authorization: Bearer` header
- **外接硬盘侦测**: macOS 扫描 `/Volumes/`，Linux 扫描 `/media/` 和 `/run/media/`，均使用 `notify` crate 实时监控
- **错误处理**: `thiserror` 定义 `AppError`，实现 `IntoResponse`
