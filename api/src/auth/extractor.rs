use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};

use crate::{
    auth::{Claims, verify},
    error::AppError,
    state::AppState,
};

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
        Ok(AuthUser { claims })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        test_support::{signup_and_login, unique_test_email},
    };
    use axum::http::Request;
    use sqlx::PgPool;
    use std::sync::Arc;

    fn test_state(pool: PgPool) -> AppState {
        AppState {
            config: Arc::new(Config {
                port: 0,
                database_url: String::new(),
                jwt_secret: std::env::var("JWT_SECRET")
                    .expect("thiếu JWT_SECRET — đã `source api/.env` chưa?"),
            }),
            db: pool,
        }
    }

    fn parts_with_header(value: Option<&str>) -> Parts {
        let mut builder = Request::builder().uri("/");
        if let Some(v) = value {
            builder = builder.header(header::AUTHORIZATION, v);
        }
        builder.body(()).unwrap().into_parts().0
    }

    #[sqlx::test]
    async fn accepts_valid_bearer_token(pool: PgPool) {
        let state = test_state(pool);
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
        let state = test_state(pool);
        let mut parts = parts_with_header(None);

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }

    #[sqlx::test]
    async fn rejects_header_without_bearer_prefix(pool: PgPool) {
        let state = test_state(pool);
        let mut parts = parts_with_header(Some("Basic dXNlcjpwYXNz"));

        let result = AuthUser::from_request_parts(&mut parts, &state).await;

        assert!(result.is_err());
    }
}
