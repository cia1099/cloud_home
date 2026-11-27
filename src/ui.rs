//! 内嵌 Web UI：与 `/api/v1` 同源提供静态文件，未知前端路由回退到 `index.html`（SPA）。
//!
//! 发布构建把 `ui/dist` 编进二进制（见 `build.rs`），调试构建直接读磁盘上的 `ui/dist`，
//! 两者只是「按相对路径取字节」这一步不同，其余逻辑（缓存头、回退、自检）完全共用。
//! 更新二进制即更新 UI，二者版本不可能错位。

use std::borrow::Cow;

use anyhow::{Context, bail};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::{HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

#[cfg(not(debug_assertions))]
include!(concat!(env!("OUT_DIR"), "/ui_embed.rs"));

/// 发布构建：从内嵌表里按相对路径（如 `assets/index-x.js`）取文件。
#[cfg(not(debug_assertions))]
fn load(path: &str) -> Option<Cow<'static, [u8]>> {
    EMBEDDED
        .iter()
        .find(|(p, _)| *p == path)
        .map(|(_, bytes)| Cow::Borrowed(*bytes))
}

/// 调试构建：直接读磁盘上的 `ui/dist`，`cargo run` 配合 Vite 构建即可用；目录不存在则视为没有该文件。
#[cfg(debug_assertions)]
fn load(path: &str) -> Option<Cow<'static, [u8]>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ui/dist");
    std::fs::read(root.join(path)).ok().map(Cow::Owned)
}

/// 所有未匹配到 API / 文档路由的请求落到这里。
pub async fn fallback(method: Method, uri: Uri) -> Response {
    respond(&method, uri.path(), &load)
}

/// 给所有响应（API、文档、UI）加 `X-Robots-Tag`：这是私有产品，不允许被搜索引擎收录。
pub async fn noindex(mut resp: Response) -> Response {
    resp.headers_mut().insert(
        HeaderName::from_static("x-robots-tag"),
        HeaderValue::from_static("noindex, nofollow"),
    );
    resp
}

type Load<'a> = &'a dyn Fn(&str) -> Option<Cow<'static, [u8]>>;

fn respond(method: &Method, path: &str, load: Load) -> Response {
    // 未知的 /api/* 保持 JSON 404，绝不回退成 index.html。
    if path == "/api" || path.starts_with("/api/") {
        return AppError::NotFound.into_response();
    }
    if method != Method::GET && method != Method::HEAD {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }
    let rel = path.trim_start_matches('/');
    // 拒绝路径穿越；调试构建读磁盘，这一步必须在取文件之前。
    if rel.split('/').any(|s| s == ".." || s.contains('\\')) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let rel = if rel.is_empty() { "index.html" } else { rel };

    let (name, body) = match load(rel) {
        Some(body) => (rel, body),
        // 缺失的静态资源（/assets/* 或带扩展名的文件）必须是 404，不能被 index.html 掩盖成「成功」。
        None if is_asset_path(rel) => return StatusCode::NOT_FOUND.into_response(),
        None => match load("index.html") {
            Some(body) => ("index.html", body),
            None => return StatusCode::NOT_FOUND.into_response(),
        },
    };

    let cache = if name.starts_with("assets/") {
        // Vite 产物文件名带内容哈希，内容变则名字变，可永久缓存。
        "public, max-age=31536000, immutable"
    } else {
        // index.html 等入口文件必须每次校验，更新后才能立刻拿到新 UI。
        "no-cache"
    };
    let mime = mime_guess::from_path(name).first_or_octet_stream();
    let body = if method == Method::HEAD {
        axum::body::Body::empty()
    } else {
        axum::body::Body::from(body.into_owned())
    };
    let mut resp = Response::new(body);
    let h = resp.headers_mut();
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    h.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).expect("mime 为合法头值"),
    );
    resp
}

/// 是否像静态资源路径：`assets/` 目录下，或末段带扩展名（`/favicon.ico`、`/x.js`）。
/// 前端路由（`/files/photos`）不带扩展名，才会回退到 index.html。
fn is_asset_path(rel: &str) -> bool {
    rel.starts_with("assets/") || rel.rsplit('/').next().is_some_and(|f| f.contains('.'))
}

/// `--self-test` 用：校验本二进制携带的 UI 完整且版本一致。
pub fn self_test() -> anyhow::Result<()> {
    verify(&load, crate::version::VERSION)
}

fn verify(load: Load, version: &str) -> anyhow::Result<()> {
    let index = load("index.html").context("内嵌 UI 缺少 index.html")?;
    let index = String::from_utf8_lossy(&index);
    for asset in referenced_assets(&index) {
        if load(asset).is_none() {
            bail!("index.html 引用的资源不存在：{asset}");
        }
    }
    let meta = load("version.json").context("内嵌 UI 缺少 version.json")?;
    let meta: serde_json::Value = serde_json::from_slice(&meta).context("version.json 无法解析")?;
    let baked = meta["version"].as_str().unwrap_or_default();
    if baked != version {
        bail!("UI 版本 {baked:?} 与二进制版本 {version:?} 不一致");
    }
    Ok(())
}

/// 取出 HTML 里引用的 `/assets/...` 资源，返回相对路径（不含开头的 `/`）。
fn referenced_assets(html: &str) -> Vec<&str> {
    html.match_indices("/assets/")
        .map(|(i, _)| {
            let rest = &html[i + 1..];
            let end = rest
                .find(|c: char| matches!(c, '"' | '\'' | ')' | '<' | '>') || c.is_whitespace())
                .unwrap_or(rest.len());
            &rest[..end]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    fn files() -> impl Fn(&str) -> Option<Cow<'static, [u8]>> {
        |p: &str| match p {
            "index.html" => Some(Cow::Borrowed(
                br#"<script src="/assets/a-1.js"></script><link href="/assets/a-1.css">"#
                    .as_slice(),
            )),
            "assets/a-1.js" => Some(Cow::Borrowed(b"js".as_slice())),
            "assets/a-1.css" => Some(Cow::Borrowed(b"css".as_slice())),
            "favicon.svg" => Some(Cow::Borrowed(b"<svg/>".as_slice())),
            "version.json" => Some(Cow::Borrowed(br#"{"version":"1.2.3"}"#.as_slice())),
            _ => None,
        }
    }

    fn get(path: &str) -> Response {
        respond(&Method::GET, path, &files())
    }

    fn header<'a>(r: &'a Response, name: &str) -> Option<&'a str> {
        r.headers().get(name).and_then(|v| v.to_str().ok())
    }

    #[tokio::test]
    async fn root_and_deep_links_serve_index_no_cache() {
        for p in ["/", "/files/photos", "/s/abc123/"] {
            let r = get(p);
            assert_eq!(r.status(), StatusCode::OK, "{p}");
            assert_eq!(header(&r, "cache-control"), Some("no-cache"));
            assert!(header(&r, "content-type").unwrap().starts_with("text/html"));
            let body = to_bytes(r.into_body(), usize::MAX).await.unwrap();
            assert!(body.starts_with(b"<script"));
        }
    }

    #[test]
    fn hashed_assets_are_immutable() {
        let r = get("/assets/a-1.js");
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(
            header(&r, "cache-control"),
            Some("public, max-age=31536000, immutable")
        );
        assert!(header(&r, "content-type").unwrap().contains("javascript"));
    }

    #[test]
    fn root_files_are_revalidated() {
        let r = get("/favicon.svg");
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(header(&r, "cache-control"), Some("no-cache"));
    }

    #[test]
    fn missing_asset_is_404_not_index() {
        assert_eq!(get("/assets/gone-9.js").status(), StatusCode::NOT_FOUND);
        assert_eq!(get("/robots.txt").status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn unknown_api_is_json_404() {
        for p in ["/api", "/api/x", "/api/v1/nope"] {
            let r = get(p);
            assert_eq!(r.status(), StatusCode::NOT_FOUND, "{p}");
            assert!(header(&r, "content-type").unwrap().contains("json"));
            let body = to_bytes(r.into_body(), usize::MAX).await.unwrap();
            assert!(String::from_utf8_lossy(&body).contains("NOT_FOUND"));
        }
    }

    #[test]
    fn traversal_and_methods_are_refused() {
        assert_eq!(get("/assets/../Cargo.toml").status(), StatusCode::NOT_FOUND);
        let r = respond(&Method::POST, "/files", &files());
        assert_eq!(r.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn head_has_headers_without_body() {
        let r = respond(&Method::HEAD, "/", &files());
        assert_eq!(r.status(), StatusCode::OK);
        assert!(
            to_bytes(r.into_body(), usize::MAX)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn noindex_marks_every_response() {
        let r = noindex(get("/")).await;
        assert_eq!(header(&r, "x-robots-tag"), Some("noindex, nofollow"));
    }

    #[test]
    fn verify_accepts_complete_ui() {
        verify(&files(), "1.2.3").unwrap();
    }

    #[test]
    fn verify_rejects_version_mismatch() {
        assert!(verify(&files(), "1.2.4").is_err());
    }

    #[test]
    fn verify_rejects_missing_index_asset_or_version() {
        let no_index = |p: &str| if p == "index.html" { None } else { files()(p) };
        assert!(verify(&no_index, "1.2.3").is_err());
        let no_asset = |p: &str| {
            if p == "assets/a-1.css" {
                None
            } else {
                files()(p)
            }
        };
        assert!(verify(&no_asset, "1.2.3").is_err());
        let no_version = |p: &str| {
            if p == "version.json" {
                None
            } else {
                files()(p)
            }
        };
        assert!(verify(&no_version, "1.2.3").is_err());
    }

    #[test]
    fn referenced_assets_extracts_paths() {
        let html = r#"<script src="/assets/a.js"></script><link href='/assets/b.css'>"#;
        assert_eq!(referenced_assets(html), vec!["assets/a.js", "assets/b.css"]);
    }
}
