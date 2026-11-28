use axum::{Json, Router, extract::State, routing::post};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    error::AppError,
    order_status::OrderStatus,
    repository::{
        self,
        order::{self, NewOrderItem},
    },
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/orders", post(create_order))
}

#[derive(Deserialize)]
struct CreateOrderItemRequest {
    product_id: Uuid,
    quantity: i32,
}

#[derive(Deserialize)]
struct CreateOrderRequest {
    items: Vec<CreateOrderItemRequest>,
}

#[derive(Serialize)]
struct OrderItemResponse {
    product_id: Uuid,
    product_name: String,
    unit_price_cents: i32,
    quantity: i32,
}

#[derive(Serialize)]
struct OrderResponse {
    id: Uuid,
    status: OrderStatus,
    total_cents: i32,
    items: Vec<OrderItemResponse>,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn create_order(
    State(state): State<AppState>,
    AuthUser { claims, .. }: AuthUser,
    Json(body): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>), AppError> {
    repository::profile::ensure_exists(&state.db, claims.sub).await?;

    let items: Vec<NewOrderItem> = body
        .items
        .into_iter()
        .map(|i| NewOrderItem {
            product_id: i.product_id,
            quantity: i.quantity,
        })
        .collect();

    let (new_order, new_items) = order::create(&state.db, claims.sub, &items).await?;
    let response = OrderResponse {
        id: new_order.id,
        status: new_order.status,
        total_cents: new_order.total_cents,
        items: new_items
            .into_iter()
            .map(|item| OrderItemResponse {
                product_id: item.product_id,
                product_name: item.product_name,
                unit_price_cents: item.unit_price_cents,
                quantity: item.quantity,
            })
            .collect(),
        created_at: new_order.created_at,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{signup_and_login, test_state, unique_test_email};
    use axum::body::Body;
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use tower::ServiceExt;

    async fn seed_product(pool: &PgPool, name: &str, price_cents: i32) -> Uuid {
        sqlx::query_scalar("INSERT INTO products (name, price_cents) VALUES ($1, $2) RETURNING id")
            .bind(name)
            .bind(price_cents)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    #[sqlx::test]
    async fn create_order_requires_authentication(pool: PgPool) {
        let app = crate::routes::router(test_state(pool).await);
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/orders")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":[]}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[sqlx::test]
    async fn create_order_succeeds_with_valid_items(pool: PgPool) {
        let product_id = seed_product(&pool, "Phở bò", 50_000).await;
        let app = crate::routes::router(test_state(pool).await);
        let email = unique_test_email("test-create-order");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let payload = serde_json::json!({ "items": [{ "product_id": product_id, "quantity": 2 }] });
        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/orders")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from(payload.to_string()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["total_cents"], 100_000);
        assert_eq!(json["status"], "pending");
    }

    #[sqlx::test]
    async fn create_order_rejects_empty_items(pool: PgPool) {
        let app = crate::routes::router(test_state(pool).await);
        let email = unique_test_email("test-create-order-empty");
        let token = signup_and_login(&email, "TestPassword123!").await;

        let request = axum::http::Request::builder()
            .method("POST")
            .uri("/v1/orders")
            .header("Authorization", format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"items":[]}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
