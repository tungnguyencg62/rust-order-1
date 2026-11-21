use sqlx::PgPool;

use crate::{cache::RevocationStore, config::Config};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub revocation: Arc<dyn RevocationStore>,
}
