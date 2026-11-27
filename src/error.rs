use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, OptionalFromRequest, Request};
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

    /// 413：请求体超过 `MAX_UPLOAD_SIZE_MB`。
    #[error("请求体超过大小限制")]
    PayloadTooLarge,

    #[error("{0}")]
    BadRequest(String),

    /// 请求体 JSON 提取失败（语法、类型、Content-Type、过大），由 `ApiJson` 产生。
    #[error("{2}")]
    BadBody(StatusCode, &'static str, String),

    #[error("资源冲突: {0}")]
    Conflict(String),

    /// 409，带稳定的机器可读错误码（前端据此区分「有传输进行中，可强制」等情形）。
    #[error("{1}")]
    CodedConflict(&'static str, String),

    #[error("分享链接已过期")]
    Gone,

    #[error("外接硬盘不可用")]
    DriveUnavailable,

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
            AppError::NotFound
            | AppError::PayloadTooLarge
            | AppError::BadRequest(_)
            | AppError::BadBody(..) => {
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
            AppError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE"),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, "BAD_REQUEST"),
            AppError::BadBody(status, code, _) => (*status, code),
            AppError::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT"),
            AppError::CodedConflict(code, _) => (StatusCode::CONFLICT, code),
            AppError::Gone => (StatusCode::GONE, "GONE"),
            AppError::DriveUnavailable => (StatusCode::SERVICE_UNAVAILABLE, "DRIVE_UNAVAILABLE"),
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

/// 请求体 JSON 提取器：替换 `axum::Json` 作为入参，
/// 所有提取失败都走统一错误信封，而不是 axum 默认的 `text/plain`。
///
/// 错误码：语法错 `INVALID_JSON`(400)、类型/字段错 `INVALID_BODY`(422)、
/// 缺少或错误的 Content-Type `UNSUPPORTED_MEDIA_TYPE`(415)、过大 `PAYLOAD_TOO_LARGE`(413)。
pub struct ApiJson<T>(pub T);

impl From<JsonRejection> for AppError {
    fn from(rej: JsonRejection) -> Self {
        let (code, message) = match &rej {
            JsonRejection::JsonSyntaxError(_) => ("INVALID_JSON", "请求体不是合法的 JSON"),
            JsonRejection::JsonDataError(_) => ("INVALID_BODY", "请求体字段缺失或类型不正确"),
            JsonRejection::MissingJsonContentType(_) => (
                "UNSUPPORTED_MEDIA_TYPE",
                "Content-Type 必须是 application/json",
            ),
            _ if rej.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                return AppError::PayloadTooLarge;
            }
            _ => ("INVALID_BODY", "无法读取请求体"),
        };
        AppError::BadBody(rej.status(), code, message.to_string())
    }
}

impl<S, T> FromRequest<S> for ApiJson<T>
where
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, AppError> {
        Ok(Self(Json::<T>::from_request(req, state).await?.0))
    }
}

/// 请求体可选的端点用：没有 Content-Type 视为未提供，其余失败同样走信封。
impl<S, T> OptionalFromRequest<S> for ApiJson<T>
where
    Json<T>: OptionalFromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Option<Self>, AppError> {
        Ok(<Json<T> as OptionalFromRequest<S>>::from_request(req, state)
            .await?
            .map(|j| Self(j.0)))
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
            (AppError::PayloadTooLarge, 413, "PAYLOAD_TOO_LARGE"),
            (
                AppError::BadBody(StatusCode::BAD_REQUEST, "INVALID_JSON", "x".into()),
                400,
                "INVALID_JSON",
            ),
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
