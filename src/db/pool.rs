use std::str::FromStr;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use crate::db::error::DbError;

/// Mirrors PocketBase's dual-DB setup.
/// `data` = main app DB, `aux` = logs/auxiliary DB.
pub struct DbPools {
    pub data: SqlitePool,
    pub aux: SqlitePool,
}

impl DbPools {
    /// Connect both pools to file-based SQLite DBs.
    pub async fn connect(data_dir: &str) -> Result<Self, DbError> {
        let data = Self::open_pool(&format!("{}/data.db", data_dir)).await?;
        let aux  = Self::open_pool(&format!("{}/aux.db",  data_dir)).await?;
        Ok(Self { data, aux })
    }

    /// In-memory version for tests only.
    pub async fn connect_in_memory() -> Result<Self, DbError> {
        let data = Self::open_pool("sqlite::memory:").await?;
        let aux  = Self::open_pool("sqlite::memory:").await?;
        Ok(Self { data, aux })
    }

    async fn open_pool(url: &str) -> Result<SqlitePool, DbError> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(url)?
            .create_if_missing(true);
        Ok(SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?)
    }

    /// Mirrors BaseApp.HasTable
    pub async fn has_table(&self, name: &str) -> Result<bool, DbError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?"
        )
            .bind(name)
            .fetch_one(&self.data)
            .await?;
        Ok(row.0 > 0)
    }

    /// Mirrors BaseApp.RunInTransaction
    pub async fn run_in_transaction<F, Fut>(&self, f: F) -> Result<(), DbError>
    where
        F: FnOnce(sqlx::Transaction<'_, sqlx::Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<(), DbError>>,
    {
        let tx = self.data.begin().await?;
        f(tx).await
    }
}