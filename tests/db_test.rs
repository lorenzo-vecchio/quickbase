mod common;
use quickbase::db::migrations::{Migration, MigrationsList, MigrationsRunner};
use quickbase::db::model::BaseModel;

#[tokio::test]
async fn test_connect_in_memory() {
    let _db = common::setup_test_db().await;
}

#[tokio::test]
async fn test_has_table_false_before_migration() {
    let db = common::setup_test_db().await;
    let exists = db.has_table("_collections").await.unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_migrations_runner_up() {
    let db = common::setup_test_db().await;
    let mut list = MigrationsList::new();
    list.register(Migration {
        file: "001_initial",
        up: |pool| {
            let pool = pool.clone();
            Box::pin(async move {
                sqlx::query(
                    "CREATE TABLE IF NOT EXISTS _collections (
                        id     TEXT PRIMARY KEY,
                        name   TEXT NOT NULL UNIQUE,
                        schema TEXT NOT NULL
                    )",
                )
                    .execute(&pool)
                    .await?;
                Ok(())
            })
        },
        down: None,
    });

    let runner = MigrationsRunner::new(&db, &list);
    runner.up().await.unwrap();

    assert!(db.has_table("_collections").await.unwrap());
    assert!(db.has_table("_migrations").await.unwrap());
}

#[tokio::test]
async fn test_migrations_runner_idempotent() {
    let db = common::setup_test_db().await;
    let mut list = MigrationsList::new();
    list.register(Migration {
        file: "001_initial",
        up: |pool| {
            let pool = pool.clone();
            Box::pin(async move {
                sqlx::query("CREATE TABLE IF NOT EXISTS _test (id TEXT PRIMARY KEY)")
                    .execute(&pool)
                    .await?;
                Ok(())
            })
        },
        down: None,
    });

    let runner = MigrationsRunner::new(&db, &list);
    runner.up().await.unwrap();
    let result = runner.up().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_base_model_new_flag() {
    let mut m = BaseModel::new("abc123".to_string());
    assert!(m.is_new());
    m.mark_as_not_new();
    assert!(!m.is_new());
    assert_eq!(m.last_saved_pk(), "abc123");
}