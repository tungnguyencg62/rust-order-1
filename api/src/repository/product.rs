use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Product {
    pub id: Uuid,
    pub name: String,
    pub price_cents: i32,
}

pub async fn list_active(pool: &PgPool) -> Result<Vec<Product>, AppError> {
    let products = sqlx::query_as::<_, Product>(
        "SELECT id, name, price_cents FROM products WHERE is_active = true ORDER BY name",
    )
    .fetch_all(pool)
    .await?;
    Ok(products)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn list_active_returns_empty_when_no_products(pool: PgPool) -> sqlx::Result<()> {
        let products = list_active(&pool).await.unwrap();
        assert!(products.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn list_active_excludes_inactive_products(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query("INSERT INTO products (name, price_cents, is_active) VALUES ($1, $2, $3)")
            .bind("Phở bò")
            .bind(50_000_i32)
            .bind(true)
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO products (name, price_cents, is_active) VALUES ($1, $2, $3)")
            .bind("Món đã ngừng bán")
            .bind(30_000_i32)
            .bind(false)
            .execute(&pool)
            .await?;

        let products = list_active(&pool).await.unwrap();

        assert_eq!(products.len(), 1);
        assert_eq!(products[0].name, "Phở bò");

        Ok(())
    }

    #[sqlx::test]
    async fn list_active_orders_by_name(pool: PgPool) -> sqlx::Result<()> {
        for name in ["Trà đá", "Bún chả", "Cơm tấm"] {
            sqlx::query("INSERT INTO products (name, price_cents) VALUES ($1, $2)")
                .bind(name)
                .bind(20_000_i32)
                .execute(&pool)
                .await?;
        }

        let products = list_active(&pool).await.unwrap();
        let names: Vec<&str> = products.iter().map(|p| p.name.as_str()).collect();

        assert_eq!(names, vec!["Bún chả", "Cơm tấm", "Trà đá"]);

        Ok(())
    }
}
