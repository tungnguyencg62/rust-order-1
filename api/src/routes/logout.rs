use axum::{
    Router,
    extract::State,
    routing::{get, post},
};
use reqwest::StatusCode;

use crate::{auth::AuthUser, error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/auth/logout", post(logout))
}

async fn logout(
    State(state): State<AppState>,
    AuthUser { claims, token }: AuthUser,
) -> Result<StatusCode, AppError> {
    state
        .revocation
        .revoke_session(claims.session_id, claims.remaining_ttl_secs())
        .await?;

    let client = reqwest::Client::new();

    let resp = client
        .post("http://127.0.0.1:9999/logout?scope=local")
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("gọi GoTrue /logout thất bại: {e}")))?;

    if !resp.status().is_success() {
        tracing::warn!(
            status = %resp.status(),
            "GoTrue /logout trả lỗi, nhưng Redis đã ghi denylist nên request đã an toàn"
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{signup_and_login, test_state, unique_test_email};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use sqlx::PgPool;
    use tower::ServiceExt;

    #[sqlx::test]
    async fn logout_revokes_session_immediately(pool: PgPool) {
        let app = crate::routes::router(test_state(pool).await);
        let email = unique_test_email("test-logout");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let me_request = || {
            Request::builder()
                .uri("/v1/me")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap()
        };

        let response = app.clone().oneshot(me_request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let logout_request = Request::builder()
            .method("POST")
            .uri("/v1/auth/logout")
            .header("Authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(logout_request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = app.oneshot(me_request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
