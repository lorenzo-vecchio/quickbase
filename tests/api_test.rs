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