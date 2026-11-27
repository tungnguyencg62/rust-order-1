use axum::{Json, Router, extract::State};

use crate::{
    error::AppError,
    repository::product::{Product, list_active},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/v1/products", axum::routing::get(list_products))
}

async fn list_products(State(state): State<AppState>) -> Result<Json<Vec<Product>>, AppError> {
    let products = list_active(&state.db).await?;
    Ok(Json(products))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use sqlx::PgPool;
    use tower::ServiceExt;

    #[sqlx::test]
    async fn list_products_works_without_authentication(pool: PgPool) {
        sqlx::query("INSERT INTO products (name, price_cents) VALUES ($1, $2)")
            .bind("Phở bò")
            .bind(50_000_i32)
            .execute(&pool)
            .await
            .unwrap();

        let app = crate::routes::router(crate::test_support::test_state(pool).await);

        let request = Request::builder()
            .uri("/v1/products")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json[0]["name"], "Phở bò");
        assert_eq!(json[0]["price_cents"], 50000);
    }
}
