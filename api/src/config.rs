use crate::error::AppError;
use std::env;
use std::fmt;

pub struct Config {
    pub port: u16,
    pub database_url: String,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("port", &self.port)
            .field("database_url", &"<redacted>")
            .finish()
    }
}

pub fn load() -> Result<Config, AppError> {
    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);

    let database_url = env::var("DATABASE_URL")
        .map_err(|_| AppError::Config("thiếu biến DATABASE_URL trong môi trường".to_string()))?;

    Ok(Config { port, database_url })
}
