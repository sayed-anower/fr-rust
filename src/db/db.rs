use deadpool_postgres::{BuildError, Manager, ManagerConfig, Pool, PoolError, RecyclingMethod, Runtime};
use thiserror::Error;
use tokio_postgres::{Config, Error as PgError, NoTls, Row, types::ToSql};

/// Unified error type for connection pooling, configuration, and database queries.
#[derive(Debug, Error)]
pub enum DbError {
    #[error("Failed to acquire connection from pool: {0}")]
    Pool(#[from] PoolError),

    #[error("Database query error: {0}")]
    Query(#[from] PgError),

    #[error("Invalid database configuration: {0}")]
    Config(String),

    #[error("Failed to build connection pool: {0}")]
    Build(#[from] BuildError),
}

/// A highly optimized, easily clonable Database Pool wrapper.
#[derive(Clone)]
pub struct DbPool {
    pool: Pool,
}

impl DbPool {
    /// Create a new PostgreSQL connection pool without panicking.
    /// Takes `&str` to avoid unnecessary allocations and allows configuring max_size.
    pub fn new(database_url: &str, max_connections: usize) -> Result<Self, DbError> {
        let config: Config = database_url
            .parse()
            .map_err(|e| DbError::Config(format!("{}", e)))?;

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, mgr_config);

        // Build the optimized pool, propagating any errors instead of panicking
        let pool = Pool::builder(manager)
            .max_size(max_connections)
            .runtime(Runtime::Tokio1)
            .build()?;

        Ok(DbPool { pool })
    }

    /// Execute INSERT / UPDATE / DELETE
    /// Returns affected rows
    #[inline]
    pub async fn execute(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, DbError> {
        let client = self.pool.get().await?;
        let affected = client.execute(query, params).await?;
        Ok(affected)
    }

    /// Get ONE row
    #[inline]
    pub async fn query_one(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, DbError> {
        let client = self.pool.get().await?;
        let row = client.query_one(query, params).await?;
        Ok(row)
    }

    /// Get ONE row or none
    #[inline]
    pub async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, DbError> {
        let client = self.pool.get().await?;
        let row_opt = client.query_opt(query, params).await?;
        Ok(row_opt)
    }

    /// Get MANY rows
    #[inline]
    pub async fn query(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, DbError> {
        let client = self.pool.get().await?;
        let rows = client.query(query, params).await?;
        Ok(rows)
    }
}
