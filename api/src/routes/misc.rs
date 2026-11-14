use crate::{error::AppError, state::AppState};
use axum::{Router, routing::get};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/boom/not-found", get(boom_not_found))
        .route("/boom/internal", get(boom_internal))
        .route("/slow", get(slow))
}

async fn root() -> &'static str {
    "Hello, order-api!"
}

async fn boom_not_found() -> Result<&'static str, AppError> {
    Err(AppError::NotFound(
        "đơn hàng #123 không tồn tại".to_string(),
    ))
}

async fn boom_internal() -> Result<&'static str, AppError> {
    Err(AppError::Internal(
        "mô phỏng lỗi kết nối database".to_string(),
    ))
}

async fn slow() -> &'static str {
    tracing::info!("bắt đầu xử lý request chậm...");
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    tracing::info!("đã xử lý xong request chậm");
    "Xong việc chậm!"
}
