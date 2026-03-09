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
    assert_eq!(json, serde_json::json!([]));
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
        .execute(&app.pools().data)
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
        .execute(&app.pools().data)
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
    assert_eq!(json, serde_json::json!([]));
}

#[tokio::test]
async fn test_get_record_found() {
    let (app, _dir) = setup_test_app().await;

    sqlx::query(
        "INSERT INTO _collections (id, name, type, schema, indexes, options)
         VALUES ('r00000000000001', 'articles', 'base', '[]', '[]', '{}')"
    )
        .execute(&app.pools().data)
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
        .execute(&app.pools().data)
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
        .execute(&app.pools().data)
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