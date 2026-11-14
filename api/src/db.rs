use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::error::AppError;

pub async fn connect(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}
