use std::sync::Arc;
use quickbase::core::App;
use quickbase::models::collection::Collection;
use quickbase::forms::collection_upsert::CollectionUpsert;

async fn setup_test_app() -> (Arc<App>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();
    (Arc::new(app), dir)
}

#[tokio::test]
async fn test_collection_upsert_valid() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({ "name": "articles" }));
    let result = form.submit().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_collection_upsert_creates_table() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("articles");
    let form = CollectionUpsert::new(&app, collection);
    form.submit().await.unwrap();

    // verify the table exists by querying it
    let result = sqlx::query("SELECT * FROM articles")
        .fetch_all(&app.pools().data)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_collection_upsert_empty_name() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("");
    let form = CollectionUpsert::new(&app, collection);
    let result = form.submit().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.data.contains_key("name"));
}

#[tokio::test]
async fn test_collection_upsert_invalid_name() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("my-collection!");
    let form = CollectionUpsert::new(&app, collection);
    let result = form.submit().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.data.contains_key("name"));
}

#[tokio::test]
async fn test_collection_upsert_reserved_name() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("_collections");
    let form = CollectionUpsert::new(&app, collection);
    let result = form.submit().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.data.contains_key("name"));
}