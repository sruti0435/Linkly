use crate::config::Config;
use redis::Client;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub cache: Option<Client>,
    pub config: Config,
}

impl AppState {
    pub fn new(db: PgPool, config: Config) -> Result<Self, redis::RedisError> {
        let cache = config.redis_url.as_deref().map(Client::open).transpose()?;
        Ok(Self { db, cache, config })
    }
}
