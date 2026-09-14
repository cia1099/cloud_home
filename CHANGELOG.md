# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/cia1099/cloud_home/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/cia1099/cloud_home/releases/tag/v0.2.0
