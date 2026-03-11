mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use serde_json::json;

// helper: create an auth collection
async fn create_auth_collection(router: &axum::Router) {
    let body = json!({
        "name": "users",
        "type": "auth",
        "schema": []
    });
    router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
}

// helper: register a user
async fn register_user(router: &axum::Router, email: &str, password: &str) -> StatusCode {
    let body = json!({ "email": email, "password": password });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/records")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    res.status()
}

#[tokio::test]
async fn test_auth_register() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_auth_collection(&router).await;

    let status = register_user(&router, "test@example.com", "secret123").await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn test_auth_login_success() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_auth_collection(&router).await;
    register_user(&router, "test@example.com", "secret123").await;

    let body = json!({ "email": "test@example.com", "password": "secret123" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/auth-with-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert!(json["token"].is_string());
    assert!(json["record"]["id"].is_string());
    assert!(json["record"]["email"].as_str() == Some("test@example.com"));
    // password must never be exposed
    assert!(json["record"]["password"].is_null());
}

#[tokio::test]
async fn test_auth_login_wrong_password() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_auth_collection(&router).await;
    register_user(&router, "test@example.com", "secret123").await;

    let body = json!({ "email": "test@example.com", "password": "wrongpassword" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/auth-with-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_login_unknown_email() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_auth_collection(&router).await;

    let body = json!({ "email": "nobody@example.com", "password": "whatever" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/users/auth-with-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_on_base_collection_rejected() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    // create a base collection
    let body = json!({
        "name": "articles",
        "type": "base",
        "schema": []
    });
    router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = json!({ "email": "test@example.com", "password": "secret123" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/collections/articles/auth-with-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}