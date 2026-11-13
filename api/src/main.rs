use axum::extract::State;
use axum::{Json, Router, routing::get};
use serde::Serialize;
use std::env;
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
}

struct Config {
    port: u16,
    database_url: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("database_url", &"<redacted>")
            .finish()
    }
}

fn load_config() -> Result<Config, String> {
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "thiếu biến DATABASE_URL trong môi trường".to_string())?;

    Ok(Config { port, database_url })
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            tracing::error!("lỗi cấu hình: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(?config, "đã nạp cấu hình");

    let port = config.port;
    let state = AppState {
        config: Arc::new(config),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!(%addr, "server đang chạy");

    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello, World!"
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
