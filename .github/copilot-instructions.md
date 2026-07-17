# Cloud Home — Copilot Instructions

**Cloud Home** is a self-hosted, Google-Drive-style file storage service for external hard drives. The backend is Rust; a Next.js frontend (`ui/`) comes later. It supports multi-account isolation, a 7-day trash retention, and public share links.

The authoritative spec is [`spec/backend.md`](../spec/backend.md) — read it before implementing any backend feature. It defines the tech stack, module layout, DB schema, every API endpoint, the cross-platform drive-detection scheme, and security requirements. Most docs and code comments are written in Chinese; keep new docs/comments consistent with surrounding language.

## Project status

The backend is greenfield: `src/main.rs` is still `Hello, world!` and `Cargo.toml` has no dependencies yet. When starting implementation, add dependencies and build modules following the layout and **实现顺序 (implementation order)** at the end of `spec/backend.md`.

## Commands

- **Build**: `cargo build`
- **Run**: `cargo run`
- **Release**: `cargo build --release` then `./target/release/cloud_home`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`
- **Test all**: `cargo test`
- **Single test**: `cargo test <test_name>` (add `-- --nocapture` to see output; `cargo test <module>::` to run a module)

## Architecture (big picture)

- **axum 0.8 + tower/tower-http** web layer; **tokio** async runtime throughout.
- **`AppState { db, config, drive_manager }`** (`state.rs`) is shared across handlers.
- **Drive detection** (`drive/`) is the linchpin: the SQLite DB and all file bytes live on a *detected external drive*, not in the repo. `DriveManager` holds `Arc<RwLock<Option<PathBuf>>>`; when no drive is mounted, write operations must return `503 DRIVE_UNAVAILABLE`. Detection uses `#[cfg(target_os)]` — macOS scans `/Volumes/`, Linux scans `/media/` and `/run/media/` — behind a unified `DriveDetector` interface, with `notify` for live mount/unmount monitoring.
- **`DATABASE_URL` is constructed at runtime** from the detected drive path — do not hardcode it in `.env`.
- **Layering**: `handlers/` (HTTP) → `services/` (business logic) → `models/` + `db/`. Keep business logic in `services/`, not handlers.
- **Trash is soft-delete**: `DELETE /files/:id` sets `is_deleted` and adds a `trash` row with `expires_at = deleted_at + 7 days`. A `tasks/trash_cleaner.rs` `tokio::spawn` loop runs hourly to purge expired physical files (log-and-continue on individual failures).

## Key conventions

- **Account isolation is mandatory**: every file/folder query must include `AND owner_id = ?` sourced from JWT claims (prevents IDOR). Never trust an `owner_id` from the request body.
- **Path-traversal defense**: physical paths are derived server-side from UUIDs (`{mount}/cloud_home_data/users/{user_id}/{uuid[0..2]}/{file_id}`). The user-supplied `name` is stored in the DB only and never used to build a filesystem path.
- **Auth is dual-mode**: an `AuthUser` extractor reads the `auth_token` HttpOnly cookie first, then falls back to the `Authorization: Bearer` header. Login/register set the cookie *and* return a token.
- **Passwords**: Argon2id (memory_cost=65536, iterations=2, parallelism=1).
- **Errors**: a single `AppError` enum (`error.rs`) using `thiserror`, implementing axum's `IntoResponse`. Wire responses use `{ "error": { "code": "...", "message": "..." } }`.
- **Share tokens**: 32-byte CSPRNG, base64url encoded.
- **CORS**: must set `allow_credentials(true)` with explicit origins (never `*`).
- **Timestamps** are stored as TEXT (RFC3339) and booleans as INTEGER, per the SQLite schema.
- **API base path**: `/api/v1`.

## Language-specific guidance

Persona/style guides live in `memory/backend/CLAUDE.md` (Rust: idiomatic async, tokio channels, `thiserror`/`anyhow`, module separation) and `memory/frontend/CLAUDE.md` (React/Next.js + TailwindCSS, no semicolons, `handle`-prefixed handlers, Conventional Commits). Follow the relevant one when writing code in each area. Commit messages follow Conventional Commits (`<type>[scope]: <description>`, imperative mood, no trailing period).
