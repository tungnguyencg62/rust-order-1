use axum::{Json, Router, routing::get};
use serde::Serialize;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    print!("Đang chạy tại http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "OK",
        service: "order-api",
    })
}
