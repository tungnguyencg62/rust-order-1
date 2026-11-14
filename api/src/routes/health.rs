use crate::state::AppState;
use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

pub fn router() -> Router<AppState> {
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    port: u16,
}
async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK",
        service: "order-api",
        port: state.config.port,
    })
}
