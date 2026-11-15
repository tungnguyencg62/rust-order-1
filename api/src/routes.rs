mod health;
mod misc;
mod ready;

use crate::state::AppState;
use axum::Router;
use tower_http::trace::TraceLayer;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(misc::router())
        .merge(health::router())
        .merge(ready::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}
