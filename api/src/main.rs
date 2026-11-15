mod config;
mod db;
mod error;
mod repository;
mod routes;
mod shutdown;
mod state;

use state::AppState;
use std::sync::Arc;

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

    // --- TẠM THỜI: kiểm tra repository bằng tay, sẽ thay bằng test ở bước 3.5 ---
    let demo_id = uuid::Uuid::new_v4();
    let inserted = repository::profile::insert(&pool, demo_id)
        .await
        .expect("insert profile thất bại");
    tracing::info!(?inserted, "đã insert profile demo");
    let found = repository::profile::find_by_id(&pool, demo_id)
        .await
        .expect("find_by_id thất bại");
    tracing::info!(?found, "đã tìm profile demo (mong đợi Some)");
    let missing = repository::profile::find_by_id(&pool, uuid::Uuid::new_v4())
        .await
        .expect("find_by_id (missing) thất bại");
    tracing::info!(?missing, "đã tìm profile không tồn tại (mong đợi None)");
    // --- HẾT PHẦN TẠM THỜI ---

    let port = config.port;
    let state = AppState {
        config: Arc::new(config),
        db: pool,
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
