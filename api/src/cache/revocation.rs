use crate::error::AppError;
use async_trait::async_trait;
use redis::aio::ConnectionManager;
use uuid::Uuid;

#[async_trait]
pub trait RevocationStore: Send + Sync {
    async fn is_session_revoked(&self, session_id: Uuid) -> Result<bool, AppError>;
    async fn is_user_revoked(&self, user_id: Uuid) -> Result<bool, AppError>;
    async fn is_revoked(&self, session_id: Uuid, user_id: Uuid) -> Result<bool, AppError>;
    async fn revoke_session(&self, session_id: Uuid, ttl_secs: i64) -> Result<(), AppError>;
    async fn revoke_user(&self, user_id: Uuid, ttl_secs: i64) -> Result<(), AppError>;
}

pub struct RedisRevocationStore {
    conn: ConnectionManager,
}
impl RedisRevocationStore {
    pub fn new(conn: ConnectionManager) -> Self {
        Self { conn }
    }
}

#[async_trait]
impl RevocationStore for RedisRevocationStore {
    async fn is_session_revoked(&self, session_id: Uuid) -> Result<bool, AppError> {
        is_revoked(&mut self.conn.clone(), &format!("revoked:sid:{session_id}")).await
    }

    async fn is_user_revoked(&self, user_id: Uuid) -> Result<bool, AppError> {
        is_revoked(&mut self.conn.clone(), &format!("revoked:uid:{user_id}")).await
    }

    async fn is_revoked(&self, session_id: Uuid, user_id: Uuid) -> Result<bool, AppError> {
        let sid_key = format!("revoked:sid:{session_id}");
        let uid_key = format!("revoked:uid:{user_id}");

        let count: i64 = redis::cmd("EXISTS")
            .arg(&sid_key)
            .arg(&uid_key)
            .query_async(&mut self.conn.clone())
            .await
            .map_err(|e| AppError::Internal(format!("kiểm tra Redis thất bại: {e}")))?;
        Ok(count > 0)
    }

    async fn revoke_session(&self, session_id: Uuid, ttl_secs: i64) -> Result<(), AppError> {
        set_revoked(
            &mut self.conn.clone(),
            &format!("revoked:sid:{session_id}"),
            ttl_secs,
        )
        .await
    }

    async fn revoke_user(&self, user_id: Uuid, ttl_secs: i64) -> Result<(), AppError> {
        set_revoked(
            &mut self.conn.clone(),
            &format!("revoked:uid:{user_id}"),
            ttl_secs,
        )
        .await
    }
}

async fn is_revoked(conn: &mut ConnectionManager, key: &str) -> Result<bool, AppError> {
    let exists: i64 = redis::cmd("EXISTS")
        .arg(key)
        .query_async(conn)
        .await
        .map_err(|e| AppError::Internal(format!("kiểm tra Redis thất bại: {e}")))?;
    Ok(exists > 0)
}

async fn set_revoked(
    conn: &mut ConnectionManager,
    key: &str,
    ttl_secs: i64,
) -> Result<(), AppError> {
    let _: () = redis::cmd("SET")
        .arg(key)
        .arg("1")
        .arg("EX")
        .arg(ttl_secs)
        .query_async(conn)
        .await
        .map_err(|e| AppError::Internal(format!("ghi vào Redis thất bại: {e}")))?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> RedisRevocationStore {
        let url = std::env::var("REDIS_URL").expect("thiếu REDIS_URL — đã `source api/.env` chưa?");
        let conn = crate::cache::connect(&url).await.unwrap();
        RedisRevocationStore::new(conn)
    }

    #[tokio::test]
    async fn session_starts_not_revoked() {
        let store = test_store().await;
        let session_id = Uuid::new_v4();
        assert!(!store.is_session_revoked(session_id).await.unwrap());
    }

    #[tokio::test]
    async fn revoke_session_marks_it_revoked() {
        let store = test_store().await;
        let session_id = Uuid::new_v4();

        store.revoke_session(session_id, 60).await.unwrap();

        assert!(store.is_session_revoked(session_id).await.unwrap());
        assert!(!store.is_user_revoked(Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    async fn revoke_user_is_independent_from_session() {
        let store = test_store().await;
        let user_id = Uuid::new_v4();

        store.revoke_user(user_id, 60).await.unwrap();

        assert!(store.is_user_revoked(user_id).await.unwrap());
        assert!(!store.is_session_revoked(Uuid::new_v4()).await.unwrap());
    }

    #[tokio::test]
    async fn revocation_expires_after_ttl() {
        let store = test_store().await;
        let session_id = Uuid::new_v4();

        store.revoke_session(session_id, 1).await.unwrap();
        assert!(store.is_session_revoked(session_id).await.unwrap());

        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

        assert!(!store.is_session_revoked(session_id).await.unwrap());
    }
    #[tokio::test]
    async fn is_revoked_checks_both_namespaces() {
        let store = test_store().await;
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        assert!(!store.is_revoked(session_id, user_id).await.unwrap());

        store.revoke_session(session_id, 60).await.unwrap();
        assert!(store.is_revoked(session_id, user_id).await.unwrap());

        let session_id2 = Uuid::new_v4();
        store.revoke_user(user_id, 60).await.unwrap();
        assert!(store.is_revoked(session_id2, user_id).await.unwrap());
    }
}
