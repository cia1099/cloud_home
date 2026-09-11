//! 实时存储事件端点（SSE）：单条连接同时承载 `usage`（占用汇总，合并防抖后推送）
//! 与 `files`（单次文件/资料夹变更，逐条立即转发）两类事件。

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures_lite::stream::Stream;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::{Instant, MissedTickBehavior, interval_at};

use crate::auth::AuthUser;
use crate::events::ChangeEvent;
use crate::models::event::FileChangeEvent;
use crate::services::usage_service;
use crate::state::AppState;

/// GET /events/stream  账户存储变更的实时流（`text/event-stream`）。
///
/// 浏览器 `EventSource` 无法自定义请求头，因此本端点依赖 Cookie 鉴权
/// （`auth_token`，`Path=/; SameSite=Lax`，同源请求会自动携带）；跨域场景需
/// `new EventSource(url, { withCredentials: true })` 并确保后端 CORS 允许该来源。
/// 连接建立后立即推送一帧 `usage`；此后 `files` 事件逐条实时转发，`usage` 合并
/// 防抖后推送，另有 30 秒周期性重算兜底（应对硬盘拔插等带外变化）。
#[utoipa::path(
    get,
    path = "/events/stream",
    tag = "usage",
    responses(
        (status = 200, description = "SSE 事件流；`event: usage` 携带 UsageResponse，`event: files` 携带 FileChangeEvent",
         content_type = "text/event-stream", body = String),
        (status = 401, description = "认证失败", body = crate::error::ErrorResponse),
    ),
    security(("cookie_auth" = []), ("bearer_auth" = []))
)]
pub async fn stream(
    State(state): State<AppState>,
    user: AuthUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let owner_id = user.user_id;

    let stream = async_stream::stream! {
        let mut rx = state.events.subscribe();
        let mut usage_dirty = false;

        // `interval()` 的第一次 `.tick()` 总是立即完成——用 `interval_at` 把首次触发
        // 推迟到一个完整周期之后，避免连接建立瞬间就误触发一次多余的重算。
        let debounce_period = Duration::from_millis(300);
        let mut debounce = interval_at(Instant::now() + debounce_period, debounce_period);
        debounce.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let resync_period = Duration::from_secs(30);
        let mut resync = interval_at(Instant::now() + resync_period, resync_period);
        resync.set_missed_tick_behavior(MissedTickBehavior::Delay);

        // 连接即推首帧，客户端无需等待变更即可拿到当前值。
        let mut last = usage_service::snapshot(&state.db, &state.drive_manager, &owner_id)
            .await
            .ok();
        if let Some(snap) = &last
            && let Ok(ev) = Event::default().event("usage").json_data(snap)
        {
            yield Ok(ev);
        }

        loop {
            tokio::select! {
                res = rx.recv() => match res {
                    Ok(ChangeEvent::Changed(change)) if change.owner_id == owner_id => {
                        usage_dirty = true;
                        if let Ok(ev) = Event::default().event("files").json_data(&change.payload) {
                            yield Ok(ev);
                        }
                    }
                    Ok(ChangeEvent::Changed(_)) => {
                        // 其他账户的变更，与本连接无关。
                    }
                    Ok(ChangeEvent::RefreshAll) => {
                        usage_dirty = true;
                        if let Ok(ev) = Event::default().event("files").json_data(FileChangeEvent::refresh_all()) {
                            yield Ok(ev);
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        // 落后于广播通道，范围未知，退化为一次全量刷新提示。
                        usage_dirty = true;
                        if let Ok(ev) = Event::default().event("files").json_data(FileChangeEvent::refresh_all()) {
                            yield Ok(ev);
                        }
                    }
                    Err(RecvError::Closed) => break,
                },
                _ = debounce.tick(), if usage_dirty => {
                    usage_dirty = false;
                    if let Ok(snap) = usage_service::snapshot(&state.db, &state.drive_manager, &owner_id).await
                        && last.as_ref() != Some(&snap)
                    {
                        if let Ok(ev) = Event::default().event("usage").json_data(&snap) {
                            yield Ok(ev);
                        }
                        last = Some(snap);
                    }
                }
                _ = resync.tick() => {
                    usage_dirty = true;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
