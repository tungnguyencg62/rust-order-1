use redis::aio::ConnectionManager;

use crate::error::AppError;

pub async fn connect(url: &str) -> Result<ConnectionManager, AppError> {
    let client = redis::Client::open(url)
        .map_err(|e| AppError::Internal(format!("URL Redis không hợp lệ: {e}")))?;

    client
        .get_connection_manager()
        .await
        .map_err(|e| AppError::Internal(format!("kết nối Redis thất bại: {e}")))
}

pub async fn ping(conn: &mut ConnectionManager) -> Result<(), AppError> {
    let _: String = redis::cmd("PING")
        .query_async(conn)
        .await
        .map_err(|e| AppError::Internal(format!("Ping Redis thất bại: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_and_ping_succeeds() {
        let url = std::env::var("REDIS_URL").expect("thiếu REDIS_URL — đã `source api/.env` chưa?");
        let mut conn = connect(&url).await.unwrap();
        ping(&mut conn).await.unwrap();
    }

    #[tokio::test]
    async fn connect_fails_with_wrong_password() {
        let result = connect("redis://:mat-khau-sai@127.0.0.1:6379/").await;
        assert!(result.is_err());
    }
}
