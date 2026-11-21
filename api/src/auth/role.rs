use std::marker::PhantomData;

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{auth::AuthUser, error::AppError, state::AppState};

pub trait Role: Send + Sync {
    const NAME: &'static str;
}

pub struct Merchant;

impl Role for Merchant {
    const NAME: &'static str = "merchant";
}

pub struct Admin;
impl Role for Admin {
    const NAME: &'static str = "admin";
}

pub struct RequireRole<R: Role> {
    pub user: AuthUser,
    _role: PhantomData<R>,
}

impl<R: Role> FromRequestParts<AppState> for RequireRole<R> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;

        if user.claims.custom_role() != Some(R::NAME) {
            return Err(AppError::Forbidden(format!(
                "yêu cầu vai trò '{}'",
                R::NAME
            )));
        }
        Ok(RequireRole {
            user,
            _role: PhantomData,
        })
    }
}
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::{
//         config::Config,
//         test_support::{make_claims, sign},
//     };
//     use axum::http::Request;
//     use sqlx::PgPool;
//     use std::sync::Arc;

//     fn test_state(pool: PgPool) -> AppState {
//         AppState {
//             config: Arc::new(Config {
//                 port: 0,
//                 database_url: String::new(),
//                 jwt_secret: std::env::var("JWT_SECRET")
//                     .expect("thiếu JWT_SECRET — đã `source .env` chưa?"),
//             }),
//             db: pool,
//         }
//     }

//     fn parts_with_bearer(token: &str) -> Parts {
//         Request::builder()
//             .uri("/")
//             .header("Authorization", format!("Bearer {token}"))
//             .body(())
//             .unwrap()
//             .into_parts()
//             .0
//     }

//     #[sqlx::test]
//     async fn allows_user_with_matching_role(pool: PgPool) {
//         let state = test_state(pool);
//         let claims = make_claims(
//             "authenticated",
//             serde_json::json!({"role": "merchant"}),
//             300,
//         );
//         let token = sign(&claims, &state.config.jwt_secret);
//         let mut parts = parts_with_bearer(&token);

//         let result = RequireRole::<Merchant>::from_request_parts(&mut parts, &state).await;

//         assert!(result.is_ok());
//     }

//     #[sqlx::test]
//     async fn rejects_user_with_different_role(pool: PgPool) {
//         let state = test_state(pool);
//         let claims = make_claims(
//             "authenticated",
//             serde_json::json!({"role": "customer"}),
//             300,
//         );
//         let token = sign(&claims, &state.config.jwt_secret);
//         let mut parts = parts_with_bearer(&token);

//         let result = RequireRole::<Merchant>::from_request_parts(&mut parts, &state).await;

//         assert!(result.is_err());
//     }

//     #[sqlx::test]
//     async fn rejects_user_with_no_role_at_all(pool: PgPool) {
//         let state = test_state(pool);
//         let claims = make_claims("authenticated", serde_json::json!({}), 300);
//         let token = sign(&claims, &state.config.jwt_secret);
//         let mut parts = parts_with_bearer(&token);

//         let result = RequireRole::<Merchant>::from_request_parts(&mut parts, &state).await;

//         assert!(result.is_err());
//     }
// }
