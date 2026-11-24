# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed
- **Breaking:** new accounts now need administrator approval. With the new
  default `REGISTRATION_MODE=approval`, `POST /auth/register` answers
  `202 {"data":{"status":"pending_approval"}}` and issues **no token and no
  cookie**; the account stays inactive until approved. `REGISTRATION_MODE=open`
  keeps the old behaviour (`201` + token), `closed` answers
  `403 REGISTRATION_CLOSED`. The very first account is approved automatically
  (and gets `201` + token) only when the `users` table is completely empty and
  the caller is on loopback / a private network; the check and the insert run in
  one transaction. Logging in with the correct password on a pending or rejected
  account returns `403 ACCOUNT_PENDING` / `403 ACCOUNT_REJECTED` (a wrong
  password is still `401`). Existing accounts are migrated to `approved` and
  keep working. The web app is updated separately (R-062).

### Added
- Account approval via the private admin endpoints: `GET /admin/users` gains
  `?status=pending|approved|rejected|all` and returns `approval_status`,
  `approval_decided_*`, `approval_note`, `registered_ip` and `created_at`;
  new `POST /admin/users/approve` and `POST /admin/users/reject` (idempotent;
  reject-then-approve is allowed; rejecting an approved account is `409` naming
  the earlier decider). Audit events `user.register`, `user.approve`,
  `user.reject`, `user.expire` (the registration audit event `auth.register`
  is renamed `user.register`).
- Registration abuse limits: `REGISTER_RATE_PER_IP_PER_HOUR` (5) and
  `PENDING_MAX` (20) answer `429 REGISTRATION_BUSY`; a rejected identifier
  cannot register again for `REJECT_COOLDOWN_DAYS` (7, `409`); pending requests
  older than `PENDING_EXPIRE_DAYS` (14) are rejected automatically by an hourly
  background task. Restoring a soft-deleted account never reactivates one that
  is not approved.
- `POST /admin/trash/purge` empties a single account's trash immediately,
  reclaiming the disk space without waiting out the retention window. This is
  the only way to clear a removed account's files: deactivating an account
  causes its own token to be rejected, so its `DELETE /trash` can no longer be
  called. Works on active accounts too, and touches only the trash, never live
  files. Irreversible — restoring afterwards yields a working but empty
  account.
- Online self-update. A tag-triggered GitHub Actions workflow builds
  `aarch64`/`x86_64` Linux (musl) and Apple Silicon binaries and publishes a
  minisign-signed `manifest.json` with each release. The device checks for new
  releases (`UPDATE_MODE=manual|auto`, install window, stable/beta channel),
  verifies the signature against a key compiled into the binary, checks the
  package sha256, self-tests the new binary, backs up the database, swaps the
  `current` symlink and restarts. A release that fails to start 3 times is
  rolled back automatically, including the database, and is never retried.
  Only runs from an install directory, never under `cargo run`. See
  `spec/update.md`. (The `/admin/update` API is not implemented yet.)
- The binary supports `--version` and `--self-test`.
- Graceful shutdown on SIGTERM/Ctrl-C: in-flight requests get up to 60 s to
  finish and SSE streams close themselves, so a restart no longer cuts them off
  abruptly. A new SSE `update` event announces an available version.

### Changed
- `GET /health` now returns JSON (`status`, `version`, `target`) instead of the
  plain text `ok`.

## [0.3.0] - 2026-09-21

### Added
- Accounts are now built on a `user_identities` table: every credential and
  identifier (password, email, phone, and later Google/Apple) is a row, so
  adding a login provider is purely additive. Email and phone are
  interchangeable login identifiers and at least one is required.
- Editable profile under `/me`: display name, password, and adding, changing,
  removing or verifying an email/phone. Changing an identifier requires both
  the current password **and** a one-time code sent to the existing
  identifier, which prevents a stolen password alone from taking over an
  account. Proving a new value is decoupled from setting it — new identifiers
  land unverified and can be verified at any time.
- One-time code support (`otp_codes`): 6 digits, 5-minute expiry, 5 attempts
  per code, 3 codes per hour per destination, stored only as an Argon2 hash.
  Delivery is currently a development placeholder that writes the code to the
  audit log; a real SMTP/SMS sender replaces a single function.
- Operator endpoints `GET/POST /admin/users*` to soft-delete and restore an
  account, guarded by a shared `ADMIN_PASSWORD` plus a loopback/private-network
  check. Deleting moves the account's files to the trash under the existing
  retention window and deactivates the account; the user row is never hard
  deleted, and restoring brings back only what the removal trashed.
- Structured logging and an audit trail: pretty console output plus a
  size-rotated JSON file, per-request correlation IDs, and audit events across
  authentication, file/folder/trash/share mutations, and background tasks.

### Changed
- **Breaking:** `username` is now a mutable, non-unique display name and can
  no longer be used to log in — `POST /auth/login` accepts an email or phone
  number as its `identifier`.
- **Breaking:** user-shaped API responses now expose `email`, `phone`,
  `has_password`, and a per-identity `verified`/`managed_by`, instead of a
  single flat `email` field.
- **Breaking:** the `users` table no longer stores `email` or `password_hash`,
  and the JWT no longer carries a username. Because the project is still
  pre-release, the original migration was rewritten rather than shipping an
  in-place upgrade, so existing development databases must be recreated.
- Uploading a file or creating a folder whose name already exists no longer
  fails; the name gets an OS-style index instead (`photo.jpg` →
  `photo(1).jpg`). Renaming and moving still reject duplicates.

### Fixed
- A deactivated user's existing token kept working until it expired (up to 7
  days), which allowed a removed account to keep acting and even restore its
  own files from the trash. Authentication now re-checks that the account is
  active on every request.
- `RUST_LOG` in `.env` was ignored because logging was initialised before the
  configuration was loaded.
- `jwt_secret` and `admin_password` are redacted when the configuration is
  printed with `{:?}`.

## [0.2.0] - 2026-09-14

### Added
- In-browser audio/video playback support: `.m4a`/`.m4v` uploads now get corrected,
  browser-standard MIME types (`audio/mp4`/`video/mp4`) instead of `mime_guess`'s
  non-standard defaults, so `<audio>`/`<video>` elements can play them directly via
  the existing `GET /files/{id}/raw` endpoint (HTTP Range already supported).
- Live storage change notifications via Server-Sent Events (`GET /events/stream`),
  plus `GET /usage` and `GET /usage/breakdown` endpoints for account/volume usage.
- `tar.gz` as an alternative batch-download archive format, selectable per-request
  alongside the existing ZIP format.
- File preview endpoint with HTTP Range support (206 Partial Content) for
  progressive loading of large files.
- OpenAPI-generated interactive API documentation (Scalar UI) at `/docs`.
- Core file storage API: multi-account isolation, folders, upload/download,
  rename, move, and search.
- Trash with 7-day retention, restore, and empty endpoints.
- Public file share links.

### Changed
- Upgraded backend dependencies to their latest major versions.

### Docs
- Added install and run instructions to the README.

[Unreleased]: https://github.com/cia1099/cloud_home/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/cia1099/cloud_home/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/cia1099/cloud_home/releases/tag/v0.2.0
