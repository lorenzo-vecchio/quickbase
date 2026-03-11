use crate::db::error::DbError;
use crate::db::migrations::Migration;
use sqlx::SqlitePool;
use std::future::Future;
use std::pin::Pin;

pub fn migration() -> Migration {
    Migration {
        file: "1_initial",
        up,
        down: Some(down),
    }
}

fn up(pool: SqlitePool) -> Pin<Box<dyn Future<Output = Result<(), DbError>> + Send>> {
    Box::pin(async move {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _collections (
                id          TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))),
                name        TEXT NOT NULL UNIQUE,
                type        TEXT NOT NULL DEFAULT 'base',
                schema      TEXT NOT NULL DEFAULT '[]',
                indexes     TEXT NOT NULL DEFAULT '[]',
                list_rule   TEXT,
                view_rule   TEXT,
                create_rule TEXT,
                update_rule TEXT,
                delete_rule TEXT,
                options     TEXT NOT NULL DEFAULT '{}',
                created     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
                updated     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
            )",
        )
            .execute(&pool)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _params (
                id      TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))),
                key     TEXT NOT NULL UNIQUE,
                value   TEXT NOT NULL DEFAULT '{}',
                created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
                updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
            )",
        )
            .execute(&pool)
            .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _superusers (
        id       TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))),
        email    TEXT NOT NULL UNIQUE,
        password TEXT NOT NULL DEFAULT '',
        tokenKey TEXT NOT NULL DEFAULT '',
        verified INTEGER NOT NULL DEFAULT 0,
        created  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
        updated  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
    )",
        )
            .execute(&pool)
            .await?;

        sqlx::query(
            "INSERT OR IGNORE INTO _collections (id, name, type, schema, options)
     VALUES ('pbc_superusers', '_superusers', 'auth', '[]', '{}')"
        )
            .execute(&pool)
            .await?;

        Ok(())
    })
}

fn down(pool: SqlitePool) -> Pin<Box<dyn Future<Output = Result<(), DbError>> + Send>> {
    Box::pin(async move {
        sqlx::query("DROP TABLE IF EXISTS _collections")
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS _params")
            .execute(&pool)
            .await?;
        sqlx::query("DROP TABLE IF EXISTS _superusers")
            .execute(&pool)
            .await?;
        Ok(())
    })
}