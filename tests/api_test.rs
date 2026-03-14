use std::sync::Arc;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;
use quickbase::core::App;

mod common;

async fn setup_test_app() -> (Arc<App>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.unwrap();
    (Arc::new(app), dir)
}

#[tokio::test]
async fn test_list_collections_empty() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(app);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Collections list returns a paginated wrapper (same shape as records list)
    assert_eq!(json["page"], 1);
    assert_eq!(json["totalItems"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_collection_not_found() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(app);

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_collection_found() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "articles");
}

#[tokio::test]
async fn test_list_records_collection_not_found() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/nonexistent/records")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_list_records_empty() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
            updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // Records list now returns a paginated wrapper object
    assert_eq!(json["page"], 1);
    assert_eq!(json["perPage"], 30);
    assert_eq!(json["totalItems"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_record_found() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            title   TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
            updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO articles (id, title) VALUES ('rec001', 'Hello World')"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records/rec001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], "rec001");
    assert_eq!(json["title"], "Hello World");
}

#[tokio::test]
async fn test_get_record_not_found() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ')),
            updated TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records/doesnotexist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_collection() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name": "articles"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "articles");
}

#[tokio::test]
async fn test_create_collection_invalid_name() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name": "my-invalid!"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_record() {
    let (app, _dir) = setup_test_app().await;

    // create the collection first
    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            title   TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/articles/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title": "Hello World"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["title"], "Hello World");
    assert!(json["id"].as_str().is_some());
}

#[tokio::test]
async fn test_create_record_collection_not_found() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/nonexistent/records")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title": "test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_collection() {
    let (app, _dir) = setup_test_app().await;

    // create a collection first
    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name": "articles"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // delete it
    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/collections/articles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // verify it's gone
    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_collection_not_found() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/collections/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_record() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            title   TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    sqlx::query("INSERT INTO articles (id, title, created, updated) VALUES ('r001', 'Hello', '', '')")
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/collections/articles/records/r001")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"title": "Updated"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["title"], "Updated");
    assert_eq!(json["id"], "r001");
}

#[tokio::test]
async fn test_delete_record() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    sqlx::query("INSERT INTO articles (id, created, updated) VALUES ('r001', '', '')")
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/collections/articles/records/r001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    // verify it's gone
    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records/r001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_record_not_found() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));
    let response = router
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/collections/articles/records/doesnotexist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ─── Health endpoint tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], 200);
    assert!(json["message"].as_str().is_some());
}

#[tokio::test]
async fn test_admin_health_endpoint() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], 200);
}

// ─── Admin settings tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_admin_get_settings() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["meta"].is_object());
    assert!(json["smtp"].is_object());
    assert!(json["logs"].is_object());
}

#[tokio::test]
async fn test_admin_update_settings() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/admin/settings")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"meta": {"appName": "TestApp"}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["meta"].is_object());
}

// ─── Scaffolds endpoint tests ────────────────────────────────────────────────

#[tokio::test]
async fn test_get_scaffolds() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/scaffolds")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["base"].is_object());
    assert!(json["auth"].is_object());
    assert!(json["base"]["fields"].is_array());
}

// ─── Logs endpoint tests ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_logs_empty() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/logs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["totalItems"], 0);
    assert!(json["items"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_logs_stats() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/logs/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

// ─── Record response shape tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_record_response_has_collection_fields() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('col001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            title   TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO articles (id, title) VALUES ('rec001', 'Hello World')"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records/rec001")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // PocketBase API requires collectionId and collectionName in every record response
    assert_eq!(json["collectionId"], "col001");
    assert_eq!(json["collectionName"], "articles");
    assert_eq!(json["id"], "rec001");
    assert_eq!(json["title"], "Hello World");
}

// ─── Collection response shape tests ─────────────────────────────────────────

#[tokio::test]
async fn test_collection_response_uses_fields_key() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    // Create a collection with fields
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"name": "posts", "fields": [{"id": "fld1", "name": "title", "type": "text"}]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // PocketBase v5+: JSON key must be "fields", not "schema"
    assert!(json["fields"].is_array(), "response must have 'fields' key");
    assert!(!json["fields"].as_array().unwrap().is_empty());
    assert_eq!(json["system"], false);
}

// ─── Pagination tests ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_records_pagination() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('col001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    sqlx::query(
        "CREATE TABLE articles (
            id      TEXT PRIMARY KEY,
            title   TEXT NOT NULL DEFAULT '',
            created TEXT NOT NULL DEFAULT '',
            updated TEXT NOT NULL DEFAULT ''
        )"
    )
        .execute(&app.pools().data)
        .await
        .unwrap();

    // Insert 5 records
    for i in 1..=5u32 {
        sqlx::query("INSERT INTO articles (id, title) VALUES (?, ?)")
            .bind(format!("rec{:03}", i))
            .bind(format!("Article {}", i))
            .execute(&app.pools().data)
            .await
            .unwrap();
    }

    let router = quickbase::apis::router(Arc::clone(&app));

    // Page 1, 2 items per page
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/collections/articles/records?page=1&perPage=2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["page"], 1);
    assert_eq!(json["perPage"], 2);
    assert_eq!(json["totalItems"], 5);
    assert_eq!(json["totalPages"], 3);
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
}

// ─── Auth stub endpoints tests ────────────────────────────────────────────────

#[tokio::test]
async fn test_request_verification_stub() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('col001', 'users', 'auth', '[]', '[]', '{}')"
    )
        .execute(&app.pools().system)
        .await
        .unwrap();

    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/request-verification")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"email": "test@example.com"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_admin_files_token() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/files/token")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["token"].as_str().is_some());
    assert!(!json["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_admin_list_backups() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/backups")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[tokio::test]
async fn test_admin_list_crons() {
    let (app, _dir) = setup_test_app().await;
    let router = quickbase::apis::router(Arc::clone(&app));

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/admin/crons")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}