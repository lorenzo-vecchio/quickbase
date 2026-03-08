use quickbase::core::App;

mod common;

#[tokio::test]
async fn test_app_bootstrap_creates_tables() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();

    assert!(app.pools().has_table("_collections").await.unwrap());
    assert!(app.pools().has_table("_params").await.unwrap());
    assert!(app.pools().has_table("_migrations").await.unwrap());
}

#[tokio::test]
async fn test_app_bootstrap_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();
    app.bootstrap().await.unwrap(); // should not panic or error
}

#[tokio::test]
async fn test_find_collection_by_name_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();

    let result = app.find_collection_by_name("nonexistent").await;
    assert!(matches!(result, Err(quickbase::db::DbError::NotFound)));
}

#[tokio::test]
async fn test_find_collection_by_name_found() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let collection = app.find_collection_by_name("articles").await.unwrap();
    assert_eq!(collection.name, "articles");
    assert!(collection.is_base());
}