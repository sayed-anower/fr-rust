use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime, PoolError};
use tokio_postgres::{Config, NoTls, Row, types::ToSql, Error as PgError};

/// A highly optimized, easily clonable Database Pool wrapper.
#[derive(Clone)]
pub struct DbPool {
    pool: Pool,
}

/// Unified error type for both connection pooling and database queries.
#[derive(Debug)]
pub enum DbError {
    Pool(PoolError),
    Query(PgError),
}

impl From<PoolError> for DbError {
    fn from(err: PoolError) -> Self {
        DbError::Pool(err)
    }
}

impl From<PgError> for DbError {
    fn from(err: PgError) -> Self {
        DbError::Query(err)
    }
}

impl DbPool {
    /// Create a new PostgreSQL connection pool
    pub fn new(database_url: String) -> Self {
        let config: Config = database_url.parse().expect("Invalid DATABASE_URL");

        let mgr_config = ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        };

        let manager = Manager::from_config(config, NoTls, mgr_config);

        // Build the optimized pool
        let pool = Pool::builder(manager)
            .max_size(32) // Adjust based on your server's core count (e.g., 4x cores)
            .runtime(Runtime::Tokio1)
            .build()
            .expect("Failed to build pool");

        DbPool { pool }
    }

    /// Execute INSERT / UPDATE / DELETE
    /// Returns affected rows
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