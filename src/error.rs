use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// 全局错误类型，统一映射为 HTTP 响应。
///
/// 线上响应格式：`{ "error": { "code": "...", "message": "..." } }`。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("认证失败")]
    Unauthorized,

    #[error("无权访问该资源")]
    Forbidden,

    /// 403，但带上可读原因（例如标识由 Google/Apple 托管、本地不可编辑）。
    #[error("{0}")]
    ForbiddenReason(String),

    /// 密码正确但账户尚待管理员批准。
    #[error("账户正在等待管理员批准")]
    AccountPending,

    /// 密码正确但账户申请已被拒绝。
    #[error("账户申请已被拒绝")]
    AccountRejected,

    #[error("当前不开放注册")]
    RegistrationClosed,

    /// 429：待批准队列已满，或同一来源注册过于频繁。
    #[error("{0}")]
    RegistrationBusy(String),

    #[error("资源不存在")]
    NotFound,

    #[error("{0}")]
    BadRequest(String),

    #[error("资源冲突: {0}")]
    Conflict(String),

    /// 409，带稳定的机器可读错误码（前端据此区分「有传输进行中，可强制」等情形）。
    #[error("{1}")]
    CodedConflict(&'static str, String),

    #[error("分享链接已过期")]
    Gone,

    #[error("外接硬盘不可用")]
    DriveUnavailable,

    #[error("负载过大")]
    #[allow(dead_code)]
    PayloadTooLarge,

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl AppError {
    /// 按变体分级记录日志。
    ///
    /// `Unauthorized`/`Forbidden` 刻意不在此记录——这里没有 `AuthUser`，
    /// 记下来也是匿名、无上下文的一行；它们已在真正产生处（`auth/extractor.rs`、
    /// `auth/jwt.rs`、`handlers/public.rs` 的 403 分支）带着真实身份记录过了，
    /// 此处再记一遍只会造成重复。
    fn log(&self) {
        match self {
            AppError::Database(_) | AppError::Io(_) | AppError::Other(_) => {
                // Debug（`?`）而非 Display（`%`）：Other(anyhow::Error) 的 Debug
                // 会打印完整 "Caused by:" 链，Display 只有最外层一句。
                tracing::error!(error = ?self, "内部错误");
            }
            AppError::DriveUnavailable => {
                tracing::warn!(reason = "drive_unavailable", "外接硬盘不可用");
            }
            AppError::Conflict(_)
            | AppError::CodedConflict(..)
            | AppError::Gone
            | AppError::AccountPending
            | AppError::AccountRejected
            | AppError::RegistrationClosed
            | AppError::RegistrationBusy(_) => {
                tracing::debug!(error = %self, "请求冲突/已过期");
            }
            AppError::NotFound | AppError::BadRequest(_) | AppError::PayloadTooLarge => {
                tracing::debug!(error = %self, "请求被拒绝");
            }
            AppError::Unauthorized | AppError::Forbidden | AppError::ForbiddenReason(_) => {}
        }
    }

    fn parts(&self) -> (StatusCode, &'static str) {
        match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED"),
            AppError::Forbidden | AppError::ForbiddenReason(_) => {
                (StatusCode::FORBIDDEN, "FORBIDDEN")
            }
            AppError::AccountPending => (StatusCode::FORBIDDEN, "ACCOUNT_PENDING"),
            AppError::AccountRejected => (StatusCode::FORBIDDEN, "ACCOUNT_REJECTED"),
            AppError::RegistrationClosed => (StatusCode::FORBIDDEN, "REGISTRATION_CLOSED"),
            AppError::RegistrationBusy(_) => {
                (StatusCode::TOO_MANY_REQUESTS, "REGISTRATION_BUSY")
            }
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::CodedConflict(code, _) => (StatusCode::CONFLICT, code),
            AppError::Gone => (StatusCode::GONE, "GONE"),
            AppError::DriveUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "DRIVE_UNAVAILABLE"),
            AppError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            AppError::Database(_) | AppError::Io(_) | AppError::Other(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR")
            }
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = self.parts();
        self.log();

        let message = match &self {
            AppError::Database(_) | AppError::Io(_) | AppError::Other(_) => {
                "服务器内部错误".to_string()
            }
            other => other.to_string(),
        };

        let body = Json(ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        });

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approval_errors_have_stable_codes() {
        for (err, status, code) in [
            (AppError::AccountPending, 403, "ACCOUNT_PENDING"),
            (AppError::AccountRejected, 403, "ACCOUNT_REJECTED"),
            (AppError::RegistrationClosed, 403, "REGISTRATION_CLOSED"),
            (AppError::RegistrationBusy("x".into()), 429, "REGISTRATION_BUSY"),
            (AppError::CodedConflict("UPDATE_X", "x".into()), 409, "UPDATE_X"),
        ] {
            let (s, c) = err.parts();
            assert_eq!((s.as_u16(), c), (status, code));
        }
    }
}

/// 错误响应体中 `error` 字段的内容。
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

/// 统一错误响应体：`{ "error": { "code": "...", "message": "..." } }`。
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}
