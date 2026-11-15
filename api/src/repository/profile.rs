use crate::error::AppError;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Profile {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}
pub async fn insert(pool: &PgPool, id: Uuid) -> Result<Profile, AppError> {
    let profile = sqlx::query_as::<_, Profile>(
        "INSERT INTO profiles (id) VALUES ($1) RETURNING id, created_at",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(profile)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Profile>, AppError> {
    let profile = sqlx::query_as::<_, Profile>("SELECT id, created_at FROM profiles WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(profile)
}
