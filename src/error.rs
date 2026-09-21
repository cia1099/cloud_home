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

    #[error("资源不存在")]
    NotFound,

    #[error("{0}")]
    BadRequest(String),

    #[error("资源冲突: {0}")]
    Conflict(String),

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
            AppError::Conflict(_) | AppError::Gone => {
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
            AppError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
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
