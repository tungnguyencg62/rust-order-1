mod config;
mod error;
mod routes;
mod shutdown;
mod state;

use state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let config = match config::load() {
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

    let app = routes::router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    tracing::info!(%addr, "server đang chạy");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::signal())
        .await
        .unwrap();
    tracing::info!("server đã tắt hoàn toàn");
}
