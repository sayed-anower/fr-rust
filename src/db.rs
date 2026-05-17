use deadpool_postgres::{
    Manager, ManagerConfig, Pool, RecyclingMethod, Runtime,
};
use std::{env, sync::Arc};
use tokio_postgres::{
    types::ToSql,
    Config, NoTls, Row,
};

pub type DbPool = Arc<Pool>;

// Create PostgreSQL connection pool
pub fn create_db_pool(database_url: &str) -> DbPool {


    let config: Config = database_url
        .parse()
        .expect("Invalid DATABASE_URL");

    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };

    let manager = Manager::from_config(
        config,
        NoTls,
        mgr_config,
    );

    // Optimized pool
    let pool = Pool::builder(manager)
        .max_size(32)
        .runtime(Runtime::Tokio1)
        .build()
        .expect("Failed to build pool");

    Arc::new(pool)
}

// Execute INSERT / UPDATE / DELETE
// Returns affected rows
pub async fn db_execute(
    pool: &DbPool,
    query: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<u64, tokio_postgres::Error> {
    let client = pool.get().await.unwrap();

    client.execute(query, params).await
}

// Get ONE row
pub async fn db_query_one(
    pool: &DbPool,
    query: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Row, Box<dyn std::error::Error>> { // Handles both pool and query errors
    let client = pool.get().await?; 
    let row = client.query_one(query, params).await?;
    Ok(row)
}


// Get ONE row or none
pub async fn db_query_opt(
    pool: &DbPool,
    query: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Row, tokio_postgres::Error> {
    let client = pool.get().await.unwrap();

    let row = client.query_opt(query, params).await?;
    Ok(row)
}

// Get MANY rows
pub async fn db_query(
    pool: &DbPool,
    query: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Vec<Row>, tokio_postgres::Error> {
    let client = pool.get().await.unwrap();

    let rows = client.query(query, params).await?;
    Ok(row)
}



