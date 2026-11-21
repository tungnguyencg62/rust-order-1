mod auth;
mod cache;
mod config;
mod db;
mod error;
mod repository;
mod routes;
mod shutdown;
mod state;

use state::AppState;
use std::sync::Arc;

#[cfg(test)]
mod test_support;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

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

    let pool = match db::connect(&config.database_url).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("lỗi kết nối database: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = db::ping(&pool).await {
        tracing::error!("{e}");
        std::process::exit(1);
    };

    tracing::info!("kết nối Postgres OK");

    let redis_url = config.redis_url.clone();
    let mut redis_conn = match cache::connect(&redis_url).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = cache::ping(&mut redis_conn).await {
        tracing::error!("{e}");
        std::process::exit(1);
    };
    tracing::info!("kết nối Redis OK");

    let port = config.port;
    let state = AppState {
        config: Arc::new(config),
        db: pool,
        redis: redis_conn,
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
