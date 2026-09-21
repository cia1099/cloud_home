//! 结构化日志与审计初始化。
//!
//! 双路输出：控制台（默认人类可读的 pretty，便于 `cargo run` 时阅读）+
//! 滚动日志文件（始终 JSON，便于云端日志采集）。文件按大小滚动（默认 1 MB），
//! 保留 1 个备份（`cloud_home.log` + `cloud_home.log.1`）。
//!
//! 三个 target：
//! - `audit`：谁做了什么（登录/上传/删除…），审计线索；
//! - `http`：访问日志（每个请求一行，含 request_id、状态、耗时）；
//! - `cloud_home::*`：普通诊断日志。

use std::net::IpAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, MatchedPath, Request};
use axum::http::{Extensions, HeaderMap};
use axum::response::Response;
use file_rotate::compression::Compression;
use file_rotate::suffix::AppendCount;
use file_rotate::{ContentLimit, FileRotate};
use tracing::{Span, field};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry, fmt};

use crate::config::Config;

/// 无 `RUST_LOG` 时的默认过滤：应用与自定义 target 取 info，tower_http 降到 warn
/// （访问日志由我们自己在 `http` target 发出，不再依赖 tower_http 的 debug span）。
const DEFAULT_FILTER: &str = "cloud_home=info,http=info,audit=info,tower_http=warn";

/// 统一的审计日志入口：固定 `target: "audit"`，强制携带 `event` 与 `result`，
/// 避免各调用点自行拼字段导致的字段名漂移。
///
/// 用法：`audit!(info, "file.upload", "success", user_id = %u, file_id = %id, "上传文件");`
#[macro_export]
macro_rules! audit {
    ($lvl:ident, $event:expr, $result:expr, $($rest:tt)*) => {
        ::tracing::$lvl!(target: "audit", event = $event, result = $result, $($rest)*)
    };
}

fn make_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_FILTER))
}

/// 初始化日志系统，返回必须在整个进程生命周期内持有的 `WorkerGuard`
/// （一旦丢弃，非阻塞写线程会停止刷新，文件日志将丢失）。
#[must_use = "丢弃 WorkerGuard 会导致文件日志停止刷新"]
pub fn init(cfg: &Config) -> Vec<WorkerGuard> {
    let mut guards = Vec::new();
    let mut layers: Vec<Box<dyn Layer<Registry> + Send + Sync>> = Vec::new();

    // 控制台层：pretty（默认）或 json。
    let console = if cfg.log_format == "json" {
        fmt::layer()
            .json()
            .flatten_event(true)
            .with_writer(std::io::stdout)
            .with_filter(make_filter())
            .boxed()
    } else {
        fmt::layer()
            .with_writer(std::io::stdout)
            .with_filter(make_filter())
            .boxed()
    };
    layers.push(console);

    // 文件层：始终 JSON，按大小滚动。
    if cfg.log_file_enabled {
        let bytes = (cfg.log_max_size_mb.max(1) as usize) * 1024 * 1024;
        let rotate = FileRotate::new(
            cfg.log_dir.join("cloud_home.log"),
            AppendCount::new(1),
            ContentLimit::Bytes(bytes),
            Compression::None,
            None,
        );
        let (writer, guard) = tracing_appender::non_blocking(rotate);
        guards.push(guard);
        layers.push(
            fmt::layer()
                .json()
                .flatten_event(true)
                .with_current_span(true)
                .with_span_list(false)
                .with_ansi(false)
                .with_writer(writer)
                .with_filter(make_filter())
                .boxed(),
        );
    }

    tracing_subscriber::registry().with(layers).init();

    install_panic_hook();
    guards
}

/// 把 panic 信息送进日志系统（默认 hook 只写 stderr，不会进文件 sink）。
/// 同时覆盖 handler 与后台任务的 panic；handler 的 panic 另由 `CatchPanicLayer`
/// 捕获并转成 500，使服务不中断。
fn install_panic_hook() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!(
            target: "audit",
            event = "process.panic",
            result = "failure",
            panic = %info,
            "进程 panic"
        );
        prev(info);
    }));
}

/// `TraceLayer::make_span_with` 回调：为每个请求建立 `http` span，
/// 携带 request_id / method / route（路由模板，非原始路径）/ client_ip，
/// 并预留 user_id、username 两个字段由认证提取器回填。
pub fn make_span<B>(req: &Request<B>, trust_proxy: bool) -> Span {
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    // 用匹配到的路由模板，绝不用 `req.uri().path()`——
    // `/public/shares/{token}` 的原始路径含分享密钥，会被写进外发的日志文件。
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("unknown");
    let ip = client_ip(req.headers(), req.extensions(), trust_proxy)
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    tracing::info_span!(
        "http",
        request_id = %request_id,
        method = %req.method(),
        route = %route,
        client_ip = %ip,
        user_id = field::Empty,
        username = field::Empty,
    )
}

/// 解析客户端 IP：仅在信任反向代理时读取 `X-Forwarded-For` / `X-Real-IP`，
/// 否则用 TCP 对端地址（`ConnectInfo`）。默认不信任，防伪造。
///
/// 取 headers/extensions 而非整个 `Request`，好让请求提取器（`Parts`）
/// 也能复用同一份判定——「客户端 IP 是什么」只此一处定义，不会各写各的。
pub(crate) fn client_ip(
    headers: &HeaderMap,
    extensions: &Extensions,
    trust_proxy: bool,
) -> Option<IpAddr> {
    if trust_proxy {
        if let Some(first) = headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            && let Ok(ip) = first.trim().parse::<IpAddr>()
        {
            return Some(ip);
        }
        if let Some(ip) = headers
            .get("x-real-ip")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<IpAddr>().ok())
        {
            return Some(ip);
        }
    }
    extensions
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
}

/// `TraceLayer::on_response` 回调：每个请求完成时发一条访问日志。
/// 5xx 记为 warn，其余 info。（`on_failure` 置空，避免与此重复。）
pub fn on_response<B>(res: &Response<B>, latency: Duration, _span: &Span) {
    let status = res.status().as_u16();
    let latency_ms = latency.as_millis();
    if res.status().is_server_error() {
        tracing::warn!(target: "http", event = "http.request", status, latency_ms, "请求完成");
    } else {
        tracing::info!(target: "http", event = "http.request", status, latency_ms, "请求完成");
    }
}

/// 把一次重命名/移动渲染成 git `diff --stat` 风格：折叠公共前后缀，
/// 差异中段用 `{old => new}` 花括号表示。
///
/// 例：
/// - 同目录重命名 `Documents/2025/{draft.txt => final.txt}`
/// - 纯移动 `{Documents/2025 => Archive}/photo.jpg`
/// - 完全不同 `a/b/x.txt => c/y.txt`
/// - 未变化则原样返回路径。
pub fn git_rename_notation(old: &str, new: &str) -> String {
    if old == new {
        return old.to_string();
    }

    let old_segs: Vec<&str> = old.split('/').collect();
    let new_segs: Vec<&str> = new.split('/').collect();

    // 公共前缀段数。
    let mut prefix = 0;
    while prefix < old_segs.len()
        && prefix < new_segs.len()
        && old_segs[prefix] == new_segs[prefix]
    {
        prefix += 1;
    }

    // 公共后缀段数（不与前缀重叠）。
    let mut suffix = 0;
    while suffix < old_segs.len() - prefix
        && suffix < new_segs.len() - prefix
        && old_segs[old_segs.len() - 1 - suffix] == new_segs[new_segs.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let old_mid = old_segs[prefix..old_segs.len() - suffix].join("/");
    let new_mid = new_segs[prefix..new_segs.len() - suffix].join("/");

    let prefix_str = old_segs[..prefix].join("/");
    let suffix_str = old_segs[old_segs.len() - suffix..].join("/");

    let mut out = String::new();
    if !prefix_str.is_empty() {
        out.push_str(&prefix_str);
        out.push('/');
    }
    out.push('{');
    out.push_str(&old_mid);
    out.push_str(" => ");
    out.push_str(&new_mid);
    out.push('}');
    if !suffix_str.is_empty() {
        out.push('/');
        out.push_str(&suffix_str);
    }
    out
}

// 注：`on_failure` 在 router 中传 `()`（空实现），5xx 统一由 `on_response` 记录，
// 避免同一请求出现两条失败日志。

#[cfg(test)]
mod tests {
    use super::git_rename_notation;

    #[test]
    fn same_dir_rename() {
        assert_eq!(
            git_rename_notation("Documents/2025/draft.txt", "Documents/2025/final.txt"),
            "Documents/2025/{draft.txt => final.txt}"
        );
    }

    #[test]
    fn pure_move_same_name() {
        assert_eq!(
            git_rename_notation("Documents/2025/photo.jpg", "Archive/photo.jpg"),
            "{Documents/2025 => Archive}/photo.jpg"
        );
    }

    #[test]
    fn unrelated_paths() {
        assert_eq!(
            git_rename_notation("a/b/x.txt", "c/y.txt"),
            "{a/b/x.txt => c/y.txt}"
        );
    }

    #[test]
    fn root_level_rename() {
        assert_eq!(git_rename_notation("x.txt", "y.txt"), "{x.txt => y.txt}");
    }

    #[test]
    fn move_from_nested_to_root() {
        assert_eq!(
            git_rename_notation("A/B/x.txt", "x.txt"),
            "{A/B => }/x.txt"
        );
    }

    #[test]
    fn identical_returns_path() {
        assert_eq!(git_rename_notation("a/b.txt", "a/b.txt"), "a/b.txt");
    }
}
