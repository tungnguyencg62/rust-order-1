use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    extract::{Path, State},
    routing::post,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{
    auth::{Admin, MAX_TOKEN_LIFETIME_SECS, RequireRole},
    error::AppError,
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/admin/users/{user_id}/ban", post(ban_user))
}

async fn ban_user(
    State(state): State<AppState>,
    RequireRole { .. }: RequireRole<Admin>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state
        .revocation
        .revoke_user(user_id, MAX_TOKEN_LIFETIME_SECS)
        .await?;
    let service_token = mint_service_role_token(&state.config.jwt_secret)?;

    let client = reqwest::Client::new();
    let resp = client
        .put(format!("http://127.0.0.1:9999/admin/users/{user_id}"))
        .header("Authorization", format!("Bearer {service_token}"))
        .json(&serde_json::json!({ "ban_duration": "876000h" }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("gọi GoTrue admin API thất bại: {e}")))?;

    if !resp.status().is_success() {
        return Err(AppError::Internal(format!(
            "GoTrue admin API trả lỗi: {}",
            resp.status()
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}

fn mint_service_role_token(secret: &str) -> Result<String, AppError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let claims = serde_json::json!({
        "role":"service_role",
        "iss":"order-app",
        "iat":now,
        "exp":now + 60,
    });

    let encoding_key = EncodingKey::from_secret(secret.as_bytes())
        .map_err(|e| AppError::Internal(format!("tạo encoding key thất bại: {e}")))?;

    encode(&Header::default(), &claims, &encoding_key)
        .map_err(|e| AppError::Internal(format!("ký service-role token thất bại: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{
        make_claims, sign, signup_and_login_with_id, test_state, unique_test_email,
    };
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::PgPool;
    use tower::ServiceExt;

    #[sqlx::test]
    async fn admin_can_ban_user_and_revokes_immediately(pool: PgPool) {
        let state = test_state(pool).await;
        let secret = state.config.jwt_secret.clone();
        let app = crate::routes::router(state);

        let target_email = unique_test_email("test-ban-target");
        let (target_token, target_id) =
            signup_and_login_with_id(&target_email, "TestPassword123!").await;

        let me_request = || {
            Request::builder()
                .uri("/v1/me")
                .header("Authorization", format!("Bearer {target_token}"))
                .body(Body::empty())
                .unwrap()
        };
        assert_eq!(
            app.clone().oneshot(me_request()).await.unwrap().status(),
            StatusCode::OK
        );

        let admin_claims = make_claims("authenticated", serde_json::json!({"role": "admin"}), 300);
        let admin_token = sign(&admin_claims, &secret);

        let ban_request = Request::builder()
            .method("POST")
            .uri(format!("/v1/admin/users/{target_id}/ban"))
            .header("Authorization", format!("Bearer {admin_token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(ban_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        assert_eq!(
            app.oneshot(me_request()).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[sqlx::test]
    async fn non_admin_cannot_ban(pool: PgPool) {
        let state = test_state(pool).await;
        let secret = state.config.jwt_secret.clone();
        let app = crate::routes::router(state);

        let target_email = unique_test_email("test-ban-target2");
        let (_, target_id) = signup_and_login_with_id(&target_email, "TestPassword123!").await;

        let customer_claims = make_claims(
            "authenticated",
            serde_json::json!({"role": "customer"}),
            300,
        );
        let customer_token = sign(&customer_claims, &secret);

        let ban_request = Request::builder()
            .method("POST")
            .uri(format!("/v1/admin/users/{target_id}/ban"))
            .header("Authorization", format!("Bearer {customer_token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(ban_request).await.unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
