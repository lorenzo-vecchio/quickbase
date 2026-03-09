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

#[tokio::test]
async fn test_collection_upsert_with_fields() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}},
            {"id": "f00000002", "name": "views", "type": "number", "required": false, "options": {}},
            {"id": "f00000003", "name": "published", "type": "bool", "required": false, "options": {}}
        ]
    }));
    form.submit().await.unwrap();

    // verify the columns exist by inserting a record with those fields
    let result = sqlx::query(
        "INSERT INTO articles (id, title, views, published, created, updated)
         VALUES ('r001', 'Hello', 42.0, 1, '', '')"
    )
        .execute(&app.pools().data)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_collection_schema_is_persisted() {
    let (app, _dir) = setup_test_app().await;
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}}
        ]
    }));
    form.submit().await.unwrap();

    // reload from db and verify schema was saved
    let reloaded = app.find_collection_by_name("articles").await.unwrap();
    assert_eq!(reloaded.fields().len(), 1);
    assert_eq!(reloaded.fields()[0].name, "title");
    assert_eq!(reloaded.fields()[0].field_type, quickbase::models::schema::FieldType::Text);
}

#[tokio::test]
async fn test_record_upsert_required_field() {
    let (app, _dir) = setup_test_app().await;

    // create collection with a required title field
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}}
        ]
    }));
    let collection = form.submit().await.unwrap();

    // try to create a record without the required field
    let record = quickbase::models::record::Record::new(collection);
    let mut form = quickbase::forms::record_upsert::RecordUpsert::new(&app, record);
    form.load(serde_json::json!({}));
    let result = form.submit().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.data.contains_key("title"));
}

#[tokio::test]
async fn test_record_upsert_invalid_type() {
    let (app, _dir) = setup_test_app().await;

    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "views", "type": "number", "required": false, "options": {}}
        ]
    }));
    let collection = form.submit().await.unwrap();

    // try to set a string value for a number field
    let record = quickbase::models::record::Record::new(collection);
    let mut form = quickbase::forms::record_upsert::RecordUpsert::new(&app, record);
    form.load(serde_json::json!({"views": "not a number"}));
    let result = form.submit().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.data.contains_key("views"));
}

#[tokio::test]
async fn test_record_upsert_valid_fields() {
    let (app, _dir) = setup_test_app().await;

    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}},
            {"id": "f00000002", "name": "views", "type": "number", "required": false, "options": {}}
        ]
    }));
    let collection = form.submit().await.unwrap();

    let record = quickbase::models::record::Record::new(collection);
    let mut form = quickbase::forms::record_upsert::RecordUpsert::new(&app, record);
    form.load(serde_json::json!({"title": "Hello", "views": 42}));
    let result = form.submit().await;
    assert!(result.is_ok());
    let record = result.unwrap();
    assert_eq!(record.get_string("title"), "Hello");
}


#[tokio::test]
async fn test_collection_update_adds_column() {
    let (app, _dir) = setup_test_app().await;

    // create initial collection with just a title field
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}}
        ]
    }));
    form.submit().await.unwrap();

    // now update with an additional views field
    let old = app.find_collection_by_name("articles").await.unwrap();
    let mut form = CollectionUpsert::new(&app, old.clone());
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": true, "options": {}},
            {"id": "f00000002", "name": "views", "type": "number", "required": false, "options": {}}
        ]
    }));
    form.submit_update(old).await.unwrap();

    // verify both columns exist
    let result = sqlx::query(
        "INSERT INTO articles (id, title, views, created, updated)
         VALUES ('r001', 'Hello', 42.0, '', '')"
    )
        .execute(&app.pools().data)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_collection_update_drops_column() {
    let (app, _dir) = setup_test_app().await;

    // create with two fields
    let collection = Collection::new_base("articles");
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": false, "options": {}},
            {"id": "f00000002", "name": "views", "type": "number", "required": false, "options": {}}
        ]
    }));
    form.submit().await.unwrap();

    // update removing views
    let old = app.find_collection_by_name("articles").await.unwrap();
    let mut form = CollectionUpsert::new(&app, old.clone());
    form.load(serde_json::json!({
        "name": "articles",
        "schema": [
            {"id": "f00000001", "name": "title", "type": "text", "required": false, "options": {}}
        ]
    }));
    form.submit_update(old).await.unwrap();

    // verify views column is gone — inserting with it should fail
    let result = sqlx::query(
        "INSERT INTO articles (id, title, views, created, updated)
         VALUES ('r001', 'Hello', 42.0, '', '')"
    )
        .execute(&app.pools().data)
        .await;
    assert!(result.is_err());
}