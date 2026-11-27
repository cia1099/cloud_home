//! 编译期两件事：
//! 1. 把编译目标三元组（如 `aarch64-unknown-linux-musl`）烘焙进二进制，在线更新据此挑选安装包。
//! 2. 发布构建（未开 debug_assertions）把 `ui/dist` 嵌入二进制：缺失或版本与
//!    `CARGO_PKG_VERSION` 不一致则编译失败，保证不会发出过期/缺失的 UI。
//!    调试构建不嵌入（运行时直接读磁盘上的 `ui/dist`），`ui/dist` 不存在也不报错。

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=ui/dist");
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=BUILD_TARGET={target}");

    // 与 `cfg(debug_assertions)` 保持同一判据：src/ui.rs 据此选择「读磁盘」或「读内嵌」。
    if std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_none() {
        embed_ui();
    }
}

fn embed_ui() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("ui/dist");
    if !root.join("index.html").is_file() {
        fail(
            "ui/dist/index.html 不存在：发布构建前必须先构建 UI（cd ui && bun install --frozen-lockfile && bun run build）",
        );
    }
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let baked = std::fs::read_to_string(root.join("version.json"))
        .ok()
        .and_then(|s| baked_version(&s))
        .unwrap_or_else(|| fail("ui/dist/version.json 缺失或无法解析：UI 构建产物不完整"));
    if baked != version {
        fail(&format!(
            "UI 内置版本 {baked} 与 Cargo.toml 版本 {version} 不一致：请重新构建 UI（bun run build）"
        ));
    }

    let mut files = Vec::new();
    collect(&root, &root, &mut files);
    files.sort();

    let mut src = String::from("static EMBEDDED: &[(&str, &[u8])] = &[\n");
    for rel in &files {
        let abs = root.join(rel).display().to_string();
        writeln!(src, "    ({rel:?}, include_bytes!({abs:?})),").unwrap();
    }
    src.push_str("];\n");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("ui_embed.rs");
    std::fs::write(out, src).unwrap();
}

/// 递归收集 `dir` 下所有文件，路径相对 `root`、统一用 `/` 分隔。
fn collect(root: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(root, &path, out);
        } else {
            let rel = path.strip_prefix(root).unwrap();
            let parts: Vec<_> = rel.iter().map(|p| p.to_string_lossy()).collect();
            out.push(parts.join("/"));
        }
    }
}

/// 从 `{"version":"x"}` 取出版本号（构建脚本不引入 JSON 依赖，手工解析这一个固定字段）。
fn baked_version(s: &str) -> Option<String> {
    let rest = s.split("\"version\"").nth(1)?;
    let rest = rest.split_once(':')?.1.trim_start().strip_prefix('"')?;
    Some(rest.split('"').next()?.to_string())
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(1);
}
