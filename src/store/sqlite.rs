use std::path::Path;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

use crate::error::PanError;

/// Initialise the SQLite connection pool and run all pending migrations.
/// Creates the database file if it does not exist.
pub async fn init_pool(data_dir: &Path) -> Result<SqlitePool, PanError> {
    let db_path = data_dir.join("index").join("pan.db");

    let opts = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    let pool = SqlitePool::connect_with(opts).await?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| PanError::StorageError(format!("migration failed: {}", e)))?;

    Ok(pool)
}
