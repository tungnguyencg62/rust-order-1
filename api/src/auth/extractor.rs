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
        Ok(AuthUser { claims })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        parts_with_auth_header, signup_and_login, test_state, unique_test_email,
    };
    use axum::http::Request;
    use sqlx::PgPool;

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
}
