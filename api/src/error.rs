use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("lỗi cấu hình: {0}")]
    Config(String),

    #[error("không tìm thấy: {0}")]
    NotFound(String),

    #[error("lỗi máy chủ nội bộ: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, public_message) = match &self {
            AppError::Config(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "lỗi cấu hình nội bộ".to_string(),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "đã có lỗi xảy ra, vui lòng thử lại".to_string(),
            ),
        };
        tracing::error!(error = %self, status = %status, "request thất bại");
        let body = axum::Json(serde_json::json!({ "error": public_message }));
        (status, body).into_response()
    }
}
