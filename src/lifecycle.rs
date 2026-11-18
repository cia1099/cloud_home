//! 进程生命周期：在途传输计数 + 关停信号。
//!
//! 在线更新必须重启进程。重启前要知道「现在有没有人在上传/下载」（自动更新据此推迟），
//! 并且要能让常驻连接（SSE）主动收尾，否则优雅关停会被它们无限期拖住。

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use futures_lite::Stream;
use tokio_util::sync::CancellationToken;

/// 在途的大流量传输（上传、下载、打包）计数器，克隆开销低。
#[derive(Clone, Default)]
pub struct ActivityTracker {
    active: Arc<AtomicUsize>,
}

impl ActivityTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一次传输；返回的 guard 被丢弃时自动注销。
    pub fn begin(&self) -> ActivityGuard {
        self.active.fetch_add(1, Ordering::SeqCst);
        ActivityGuard {
            active: self.active.clone(),
        }
    }

    /// 当前在途传输数。
    pub fn active(&self) -> usize {
        self.active.load(Ordering::SeqCst)
    }

    /// 等到没有在途传输，或超过 `max_wait`；返回是否已空闲。
    pub async fn wait_idle(&self, max_wait: Duration) -> bool {
        let deadline = tokio::time::Instant::now() + max_wait;
        while self.active() > 0 {
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        true
    }
}

/// 传输存续期间持有；Drop 时计数 -1。
pub struct ActivityGuard {
    active: Arc<AtomicUsize>,
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        self.active.fetch_sub(1, Ordering::SeqCst);
    }
}

/// 把 guard 绑到响应流上：流被读完或连接中断（流被丢弃）时才注销，
/// 而不是 handler 返回时——流式响应在 handler 返回之后还会持续很久。
pub struct Tracked<S> {
    inner: S,
    _guard: ActivityGuard,
}

impl<S> Tracked<S> {
    pub fn new(inner: S, guard: ActivityGuard) -> Self {
        Self {
            inner,
            _guard: guard,
        }
    }
}

impl<S: Stream + Unpin> Stream for Tracked<S> {
    type Item = S::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

/// 关停信号：收到 SIGTERM / Ctrl-C，或内部 `token`（更新器触发）被取消时返回。
pub async fn shutdown_signal(token: CancellationToken) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("收到 Ctrl-C，开始优雅关停"),
        _ = terminate => tracing::info!("收到 SIGTERM，开始优雅关停"),
        _ = token.cancelled() => tracing::info!("收到内部关停请求（更新/重启），开始优雅关停"),
    }
    // 保证其余持有同一 token 的组件（如 SSE 流）也能感知到关停。
    token.cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_counts_and_releases() {
        let t = ActivityTracker::new();
        assert_eq!(t.active(), 0);
        let a = t.begin();
        let b = t.begin();
        assert_eq!(t.active(), 2);
        drop(a);
        assert_eq!(t.active(), 1);
        drop(b);
        assert_eq!(t.active(), 0);
    }

    #[tokio::test]
    async fn wait_idle_times_out_then_succeeds() {
        let t = ActivityTracker::new();
        let g = t.begin();
        assert!(!t.wait_idle(Duration::from_millis(50)).await);
        drop(g);
        assert!(t.wait_idle(Duration::from_millis(50)).await);
    }
}
