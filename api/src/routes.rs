mod admin;
mod health;
mod logout;
mod me;
mod misc;
mod products;
mod ready;

use crate::state::AppState;
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(misc::router())
        .merge(health::router())
        .merge(ready::router())
        .merge(me::router())
        .merge(logout::router())
        .merge(admin::router())
        .merge(products::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
