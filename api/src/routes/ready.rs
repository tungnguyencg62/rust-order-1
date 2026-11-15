use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::{db, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new().route("/ready", get(ready))
}

async fn ready(State(state): State<AppState>) -> impl IntoResponse {
    match db::ping(&state.db).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "ready" })),
        ),
        Err(e) => {
            tracing::warn!(error = %e, "readiness check thất bại");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "status": "not_ready" })),
            )
        }
    }
}
