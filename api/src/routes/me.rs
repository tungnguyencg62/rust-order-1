use axum::{Json, Router, extract::State, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;

use uuid::Uuid;

use crate::{
    auth::{AuthUser, Claims},
    error::AppError,
    repository::profile,
    state::{self, AppState},
};
use jsonwebtoken::{EncodingKey, Header, encode};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn make_claims(exp_offset_secs: i64) -> Claims {
    Claims {
        sub: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        aud: "authenticated".to_string(),
        role: "".to_string(),
        email: Some("fabricated@example.com".to_string()),
        exp: now_secs() + exp_offset_secs,
        iat: now_secs() - 10,
    }
}

fn sign(claims: &Claims, secret: &str) -> String {
    let decode_key =
        EncodingKey::from_secret(secret.as_bytes()).expect("cấu hình JWT secret không hợp lệ");

    encode(&Header::default(), claims, &decode_key).unwrap()
}

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/me", get(me))
}

#[derive(Serialize)]
struct MeResponse {
    id: Uuid,
    email: Option<String>,
    role: String,
    profile_created_at: DateTime<Utc>,
}

async fn me(
    State(state): State<AppState>,
    AuthUser { claims }: AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let profile = profile::ensure_exists(&state.db, claims.sub).await?;
    Ok(Json(MeResponse {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
        profile_created_at: profile.created_at,
    }))
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
    #[sqlx::test]
    async fn me_with_expired_token_returns_401(pool: PgPool) {
        let app = crate::routes::router(test_state(pool));
        let secret =
            std::env::var("JWT_SECRET").expect("thiếu JWT_SECRET — đã `source .env` chưa?");

        let claims = make_claims(-120); // exp cách đây 120 giây
        let token = sign(&claims, &secret);

        let request = Request::builder()
            .uri("/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "token đã hết hạn");
    }
    #[sqlx::test]
    async fn me_with_wrong_signature_returns_401(pool: PgPool) {
        let app = crate::routes::router(test_state(pool));

        let claims = make_claims(300); // còn hạn 5 phút, nhưng ký sai secret
        let token = sign(&claims, "secret-hoan-toan-sai-khac-voi-that");

        let request = Request::builder()
            .uri("/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "chữ ký token không hợp lệ");
    }
    #[sqlx::test]
    async fn me_creates_profile_on_first_call_and_is_idempotent(pool: PgPool) {
        let app = crate::routes::router(test_state(pool.clone()));
        let email = unique_test_email("test-me-upsert");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let request = Request::builder()
            .uri("/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let first: serde_json::Value = serde_json::from_slice(&body).unwrap();

        let request = Request::builder()
            .uri("/v1/me")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let second: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(first["profile_created_at"], second["profile_created_at"]);

        let id = Uuid::parse_str(first["id"].as_str().unwrap()).unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM profiles WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}
