use sqlx::SqlitePool;
use std::future::Future;
use std::pin::Pin;
use crate::db::{error::DbError, pool::DbPools};

type MigrationFn = fn(SqlitePool) -> Pin<Box<dyn Future<Output = Result<(), DbError>> + Send>>;

pub struct Migration {
    pub file: &'static str,
    pub up: MigrationFn,
    pub down: Option<MigrationFn>,
}

#[derive(Default)]
pub struct MigrationsList {
    items: Vec<Migration>,
}

impl MigrationsList {
    pub fn new() -> Self { Self::default() }
    pub fn register(&mut self, m: Migration) { self.items.push(m); }
    pub fn items(&self) -> &[Migration] { &self.items }
}

pub struct MigrationsRunner<'a> {
    pools: &'a DbPools,
    list: &'a MigrationsList,
}

impl<'a> MigrationsRunner<'a> {
    pub fn new(pools: &'a DbPools, list: &'a MigrationsList) -> Self {
        Self { pools, list }
    }

    pub async fn up(&self) -> Result<(), DbError> {
        self.ensure_migrations_table().await?;
        for migration in self.list.items() {
            if !self.is_applied(migration.file).await? {
                (migration.up)(self.pools.data.clone()).await?;
                self.mark_applied(migration.file).await?;
            }
        }
        Ok(())
    }

    async fn ensure_migrations_table(&self) -> Result<(), DbError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                file       TEXT PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            )",
        )
            .execute(&self.pools.data)
            .await?;
        Ok(())
    }

    async fn is_applied(&self, file: &str) -> Result<bool, DbError> {
        let row: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM _migrations WHERE file=?"
        )
            .bind(file)
            .fetch_one(&self.pools.data)
            .await?;
        Ok(row.0 > 0)
    }

    async fn mark_applied(&self, file: &str) -> Result<(), DbError> {
        sqlx::query("INSERT INTO _migrations (file) VALUES (?)")
            .bind(file)
            .execute(&self.pools.data)
            .await?;
        Ok(())
    }
}