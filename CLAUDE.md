# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust project using Cargo. It's currently a minimal setup with a single binary target in `src/main.rs`.

## Common Commands

- **Build**: `cargo build`
- **Run**: `cargo run`
- **Run with Release optimizations**: `cargo build --release` then `./target/release/cloud_home`
- **Lint/Check**: `cargo clippy`
- **Format**: `cargo fmt`
- **Test**: `cargo test`

## Project Structure

- `src/main.rs` - Entry point with the binary
- `Cargo.toml` - Project manifest (currently minimal with no external dependencies)
- `target/` - Build artifacts (ignored in git)
