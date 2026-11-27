use std::time::Duration;

use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use tokio::time::timeout;

use crate::{
    auth::{Claims, verify},
    error::AppError,
    state::AppState,
};

const REVOCATION_CHECK_TIMEOUT: Duration = Duration::from_millis(100);

pub struct AuthUser {
    pub claims: Claims,
    pub token: String,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or_else(|| AppError::Unauthorized("thiếu header Authorization".to_string()))?
            .to_str()
            .map_err(|_| AppError::Unauthorized("header Authorization không hợp lệ".to_string()))?;

        let token = header_value.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("header Authorization phải có dạng 'Bearer <token>'".to_string())
        })?;

        let claims = verify(token, &state.config.jwt_secret)?;

        let revoked = match timeout(
            REVOCATION_CHECK_TIMEOUT,
            state.revocation.is_revoked(claims.session_id, claims.sub),
        )
        .await
        {
            Ok(Ok(revoked)) => revoked,
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "kiểm tra thu hồi thất bại — fail-open, coi như chưa thu hồi");
                false
            }
            Err(_) => {
                tracing::warn!(
                    "kiểm tra thu hồi quá thời gian chờ — fail-open, coi như chưa thu hồi"
                );
                false
            }
        };
        if revoked {
            return Err(AppError::Unauthorized(
                "phiên đăng nhập đã bị thu hồi hoặc tài khoản đã bị khóa".to_string(),
            ));
        }
        Ok(AuthUser {
            claims,
            token: token.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        parts_with_auth_header, signup_and_login, test_state, test_state_with_revocation,
        unique_test_email,
    };
    use axum::http::Request;
    use sqlx::PgPool;
    use std::sync::Arc;
    use uuid::Uuid;

    fn parts_with_header(value: Option<&str>) -> Parts {
        let mut builder = Request::builder().uri("/");
        if let Some(v) = value {
            builder = builder.header(header::AUTHORIZATION, v);
        }
        builder.body(()).unwrap().into_parts().0
    }

    #[sqlx::test]
    async fn accepts_valid_bearer_token(pool: PgPool) {
        let state = test_state(pool).await;
        let email = unique_test_email("test-extractor");
        let token = signup_and_login(&email, "TestPassword123!").await;
        let mut parts = parts_with_header(Some(&format!("Bearer {token}")));

        let user = AuthUser::from_request_parts(&mut parts, &state)
            .await
            .unwrap();

        assert_eq!(user.claims.email.as_deref(), Some(email.as_str()));
    }

    #[sqlx::test]
    async fn rejects_missing_header(pool: PgPool) {
        let state = test_state(pool).await;
        let mut parts = parts_with_header(None);

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn rejects_header_without_bearer_prefix(pool: PgPool) {
        let state = test_state(pool).await;
        let mut parts = parts_with_header(Some("Basic dXNlcjpwYXNz"));

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn rejects_token_with_revoked_session(pool: PgPool) {
        let state = test_state(pool).await;
        let email = unique_test_email("test-revoked-session");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let claims = crate::auth::verify(&token, &state.config.jwt_secret).unwrap();
        state
            .revocation
            .revoke_session(claims.session_id, 60)
            .await
            .unwrap();

        let mut parts = parts_with_auth_header(Some(&format!("Bearer {token}")));
        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn rejects_token_with_revoked_user(pool: PgPool) {
        let state = test_state(pool).await;
        let email = unique_test_email("test-revoked-user");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let claims = crate::auth::verify(&token, &state.config.jwt_secret).unwrap();
        state.revocation.revoke_user(claims.sub, 60).await.unwrap();

        let mut parts = parts_with_auth_header(Some(&format!("Bearer {token}")));
        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }

    struct AlwaysErrorsRevocationStore;

    #[async_trait::async_trait]
    impl crate::cache::RevocationStore for AlwaysErrorsRevocationStore {
        async fn is_session_revoked(&self, _session_id: Uuid) -> Result<bool, AppError> {
            Err(AppError::Internal("giả lập Redis lỗi".to_string()))
        }
        async fn is_user_revoked(&self, _user_id: Uuid) -> Result<bool, AppError> {
            Err(AppError::Internal("giả lập Redis lỗi".to_string()))
        }
        async fn is_revoked(&self, _session_id: Uuid, _user_id: Uuid) -> Result<bool, AppError> {
            Err(AppError::Internal("giả lập Redis lỗi".to_string()))
        }
        async fn revoke_session(&self, _session_id: Uuid, _ttl_secs: i64) -> Result<(), AppError> {
            Ok(())
        }
        async fn revoke_user(&self, _user_id: Uuid, _ttl_secs: i64) -> Result<(), AppError> {
            Ok(())
        }
    }

    struct AlwaysTimesOutRevocationStore;

    #[async_trait::async_trait]
    impl crate::cache::RevocationStore for AlwaysTimesOutRevocationStore {
        async fn is_session_revoked(&self, _session_id: Uuid) -> Result<bool, AppError> {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            Ok(false)
        }
        async fn is_user_revoked(&self, _user_id: Uuid) -> Result<bool, AppError> {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            Ok(false)
        }
        async fn is_revoked(&self, _session_id: Uuid, _user_id: Uuid) -> Result<bool, AppError> {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            Ok(false)
        }
        async fn revoke_session(&self, _session_id: Uuid, _ttl_secs: i64) -> Result<(), AppError> {
            Ok(())
        }
        async fn revoke_user(&self, _user_id: Uuid, _ttl_secs: i64) -> Result<(), AppError> {
            Ok(())
        }
    }

    #[sqlx::test]
    async fn fail_open_when_revocation_check_errors(pool: PgPool) {
        let state = test_state_with_revocation(pool, Arc::new(AlwaysErrorsRevocationStore));
        let email = unique_test_email("test-failopen-error");
        let token = signup_and_login(&email, "TestPassword123!").await;
        let mut parts = parts_with_auth_header(Some(&format!("Bearer {token}")));

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_ok());
    }

    #[sqlx::test]
    async fn fail_open_when_revocation_check_times_out(pool: PgPool) {
        let state = test_state_with_revocation(pool, Arc::new(AlwaysTimesOutRevocationStore));
        let email = unique_test_email("test-failopen-timeout");
        let token = signup_and_login(&email, "TestPassword123!").await;
        let mut parts = parts_with_auth_header(Some(&format!("Bearer {token}")));

        let started = std::time::Instant::now();
        let result = AuthUser::from_request_parts(&mut parts, &state).await;
        let elapsed = started.elapsed();

        assert!(result.is_ok());
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "phải trả lời trong vòng 1 giây nhờ timeout, thực tế mất {elapsed:?}"
        );
    }
}
