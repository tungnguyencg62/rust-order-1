use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{error::AppError, order_status::OrderStatus};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Order {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub status: OrderStatus,
    pub total_cents: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderItem {
    pub id: Uuid,
    pub order_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub unit_price_cents: i32,
    pub quantity: i32,
}

pub struct NewOrderItem {
    pub product_id: Uuid,
    pub quantity: i32,
}

pub async fn create(
    pool: &PgPool,
    customer_id: Uuid,
    items: &[NewOrderItem],
) -> Result<(Order, Vec<OrderItem>), AppError> {
    if items.is_empty() {
        return Err(AppError::BadRequest(
            "đơn hàng phải có ít nhất một món".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    let mut staged = Vec::with_capacity(items.len());
    let mut total_cents: i64 = 0;

    for item in items {
        if item.quantity <= 0 {
            return Err(AppError::BadRequest(format!(
                "số lượng phải lớn hơn 0 (sản phẩm {})",
                item.product_id
            )));
        }

        let row: Option<(String, i32)> = sqlx::query_as(
            "SELECT name, price_cents FROM products WHERE id = $1 AND is_active = true FOR UPDATE",
        )
        .bind(item.product_id)
        .fetch_optional(&mut *tx)
        .await?;

        let (name, price_cents) = row.ok_or_else(|| {
            AppError::NotFound(format!(
                "sản phẩm {} không tồn tại hoặc đã ngừng bán",
                item.product_id
            ))
        })?;

        total_cents += price_cents as i64 * item.quantity as i64;
        staged.push((item.product_id, name, price_cents, item.quantity));
    }

    let total_cents = i32::try_from(total_cents).map_err(|_| {
        AppError::BadRequest("tổng tiền đơn hàng vượt quá giới hạn cho phép".to_string())
    })?;

    let order: Order = sqlx::query_as(
        "INSERT INTO orders (customer_id, total_cents) VALUES ($1, $2) 
        RETURNING id, customer_id, status, total_cents, created_at, updated_at",
    )
    .bind(customer_id)
    .bind(total_cents)
    .fetch_one(&mut *tx)
    .await?;

    let mut order_items = Vec::with_capacity(staged.len());

    for (product_id, name, price_cents, quantity) in staged {
        let inserted = sqlx::query_as(
            "INSERT INTO order_items (order_id, product_id, product_name, unit_price_cents, quantity)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, order_id, product_id, product_name, unit_price_cents, quantity",
        )
        .bind(order.id)
        .bind(product_id)
        .bind(name)
        .bind(price_cents)
        .bind(quantity)
        .fetch_one(&mut *tx)
        .await?;

        order_items.push(inserted);
    }

    sqlx::query(
        "INSERT INTO order_events (order_id, from_status, to_status) 
    VALUES ($1, NULL, 'pending')",
    )
    .bind(order.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((order, order_items))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seed_product(pool: &PgPool, name: &str, price_cents: i32) -> Uuid {
        sqlx::query_scalar("INSERT INTO products (name, price_cents) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(price_cents)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn seed_customer(pool: &PgPool) -> Uuid {
        let id = Uuid::new_v4();
        crate::repository::profile::ensure_exists(pool, id)
            .await
            .unwrap();
        id
    }

    #[sqlx::test]
    async fn create_fails_with_empty_items(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        assert!(matches!(
            create(&pool, customer_id, &[]).await,
            Err(AppError::BadRequest(_))
        ));
    }

    #[sqlx::test]
    async fn create_fails_with_zero_quantity(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        let product_id = seed_product(&pool, "Phở bò", 50_000).await;

        let result = create(
            &pool,
            customer_id,
            &[NewOrderItem {
                product_id,
                quantity: 0,
            }],
        )
        .await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[sqlx::test]
    async fn create_fails_with_nonexistent_product(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;

        let result = create(
            &pool,
            customer_id,
            &[NewOrderItem {
                product_id: Uuid::new_v4(),
                quantity: 1,
            }],
        )
        .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[sqlx::test]
    async fn create_fails_with_inactive_product(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        let product_id = seed_product(&pool, "Ngừng bán", 10_000).await;
        sqlx::query("UPDATE products SET is_active = false WHERE id = $1")
            .bind(product_id)
            .execute(&pool)
            .await
            .unwrap();

        let result = create(
            &pool,
            customer_id,
            &[NewOrderItem {
                product_id,
                quantity: 1,
            }],
        )
        .await;
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[sqlx::test]
    async fn create_computes_total_and_snapshots_items(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        let pho_id = seed_product(&pool, "Phở bò", 50_000).await;
        let tra_id = seed_product(&pool, "Trà đá", 10_000).await;

        let (order, items) = create(
            &pool,
            customer_id,
            &[
                NewOrderItem {
                    product_id: pho_id,
                    quantity: 2,
                },
                NewOrderItem {
                    product_id: tra_id,
                    quantity: 3,
                },
            ],
        )
        .await
        .unwrap();

        assert_eq!(order.total_cents, 2 * 50_000 + 3 * 10_000);
        assert_eq!(order.status, OrderStatus::Pending);
        assert_eq!(items.len(), 2);

        let pho_item = items.iter().find(|i| i.product_id == pho_id).unwrap();
        assert_eq!(pho_item.product_name, "Phở bò");
        assert_eq!(pho_item.unit_price_cents, 50_000);
    }

    #[sqlx::test]
    async fn create_rolls_back_fully_when_one_product_is_invalid(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        let pho_id = seed_product(&pool, "Phở bò", 50_000).await;

        let result = create(
            &pool,
            customer_id,
            &[
                NewOrderItem {
                    product_id: pho_id,
                    quantity: 1,
                },
                NewOrderItem {
                    product_id: Uuid::new_v4(),
                    quantity: 1,
                },
            ],
        )
        .await;

        assert!(result.is_err());

        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM orders WHERE customer_id = $1")
            .bind(customer_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            count, 0,
            "transaction phải rollback hoàn toàn, không được sót lại order nào"
        );
    }

    #[sqlx::test]
    async fn create_records_initial_order_event(pool: PgPool) {
        let customer_id = seed_customer(&pool).await;
        let product_id = seed_product(&pool, "Phở bò", 50_000).await;

        let (order, _) = create(
            &pool,
            customer_id,
            &[NewOrderItem {
                product_id,
                quantity: 1,
            }],
        )
        .await
        .unwrap();

        let (from_status, to_status): (Option<String>, String) =
            sqlx::query_as("SELECT from_status, to_status FROM order_events WHERE order_id = $1")
                .bind(order.id)
                .fetch_one(&pool)
                .await
                .unwrap();

        assert_eq!(from_status, None);
        assert_eq!(to_status, "pending");
    }
}
