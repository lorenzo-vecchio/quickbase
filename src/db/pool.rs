use std::str::FromStr;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use crate::db::error::DbError;

/// Mirrors PocketBase's dual-DB setup.
///
/// - `system`: system tables (`_collections`, `_params`, `_superusers`, `_migrations`, …).
///   Maps to `system.db` on disk. Never touched by user code.
/// - `data`: user-created collection tables. Maps to `data.db` on disk.
pub struct DbPools {
    pub system: SqlitePool,
    pub data: SqlitePool,
}

impl DbPools {
    /// Connect both pools to file-based SQLite DBs.
    pub async fn connect(data_dir: &str) -> Result<Self, DbError> {
        let system = Self::open_pool(&format!("{}/system.db", data_dir)).await?;
        let data   = Self::open_pool(&format!("{}/data.db",   data_dir)).await?;
        Ok(Self { system, data })
    }

    /// In-memory version for tests only.
    ///
    /// Uses `max_connections(1)` so that every operation in the pool reuses the same
    /// SQLite connection — ensuring a single in-memory database per pool, not one per
    /// connection.  This gives each test full isolation without leaking state.
    pub async fn connect_in_memory() -> Result<Self, DbError> {
        let system = Self::open_in_memory_pool().await?;
        let data   = Self::open_in_memory_pool().await?;
        Ok(Self { system, data })
    }

    async fn open_pool(url: &str) -> Result<SqlitePool, DbError> {
        let options = sqlx::sqlite::SqliteConnectOptions::from_str(url)?
            .create_if_missing(true);
        Ok(SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(options)
            .await?)
    }

    /// Creates a single-connection pool backed by an anonymous in-memory SQLite database.
    /// `max_connections(1)` guarantees all pool operations reuse the same connection and
    /// therefore the same database — critical for in-memory SQLite.
    async fn open_in_memory_pool() -> Result<SqlitePool, DbError> {
        Ok(SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?)
    }

    /// Returns the pool that owns `table_name`.
    /// Tables whose names start with `_` live in the system DB; all others in data DB.
    pub fn pool_for_table<'a>(&'a self, table_name: &str) -> &'a SqlitePool {
        if table_name.starts_with('_') {
            &self.system
        } else {
            &self.data
        }
    }

    /// Mirrors BaseApp.HasTable — checks the data (user) database.
    pub async fn has_table(&self, name: &str) -> Result<bool, DbError> {
        let pool = self.pool_for_table(name);
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?"
        )
        .bind(name)
        .fetch_one(pool)
        .await?;
        Ok(row.0 > 0)
    }

    /// Mirrors BaseApp.RunInTransaction — runs a transaction on the data pool.
    pub async fn run_in_transaction<F, Fut>(&self, f: F) -> Result<(), DbError>
    where
        F: FnOnce(sqlx::Transaction<'_, sqlx::Sqlite>) -> Fut,
        Fut: std::future::Future<Output = Result<(), DbError>>,
    {
        let tx = self.data.begin().await?;
        f(tx).await
    }
}
