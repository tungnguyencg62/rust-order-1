use axum::{Json, Router, routing::get};
use serde::Serialize;

use uuid::Uuid;

use crate::{auth::AuthUser, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/me", get(me))
}

#[derive(Serialize)]
struct MeResponse {
    id: Uuid,
    email: Option<String>,
    role: String,
}

async fn me(AuthUser { claims }: AuthUser) -> Json<MeResponse> {
    Json(MeResponse {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        test_support::{signup_and_login, unique_test_email},
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use std::sync::Arc;
    use tower::ServiceExt;

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

    #[sqlx::test]
    async fn me_without_token_returns_401(pool: PgPool) {
        let app = crate::routes::router(test_state(pool));

        let request = Request::builder()
            .uri("/v1/me")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn me_with_valid_token_returns_claims(pool: PgPool) {
        let app = crate::routes::router(test_state(pool));
        let email = unique_test_email("test-me-route");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let request = Request::builder()
            .uri("/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["email"], email);
        assert_eq!(json["role"], "");
    }
}
