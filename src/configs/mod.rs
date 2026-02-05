use deadpool_redis::{Runtime, redis::AsyncCommands};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{ENV, api::error};

pub async fn connect_database() -> Result<PgPool, error::SystemError> {
    let database_url = &ENV.database_url;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .min_connections(1)
        .acquire_slow_threshold(std::time::Duration::from_secs(3))
        .connect(&database_url)
        .await?;
    Ok(pool)
}

pub struct RedisCache {
    pool: deadpool_redis::Pool,
}

impl RedisCache {
    pub async fn new() -> Result<Self, error::SystemError> {
        let mut cfg = deadpool_redis::Config::from_url(&ENV.redis_url);
        cfg.pool = Some(deadpool_redis::PoolConfig { max_size: 16, ..Default::default() });
        let pool = cfg.create_pool(Some(Runtime::Tokio1))?;
        Ok(Self { pool })
    }

    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, error::SystemError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut conn = self.pool.get().await?;

        let value: Option<Vec<u8>> = conn.get(key).await?;

        match value {
            Some(v) => {
                let parsed = serde_json::from_slice(&v)?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    pub async fn set<T>(
        &self,
        key: &str,
        value: &T,
        expiration: usize,
    ) -> Result<(), error::SystemError>
    where
        T: serde::Serialize,
    {
        let mut conn = self.pool.get().await?;

        let serialized = serde_json::to_vec(value)?;

        conn.set_ex::<_, _, ()>(key, serialized, expiration as u64).await?;

        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), error::SystemError> {
        let mut conn = self.pool.get().await?;
        conn.del::<_, ()>(key).await?;
        Ok(())
    }
}
