mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;
use serde_json::json;

async fn create_admin(router: &axum::Router, email: &str, password: &str) -> StatusCode {
    let body = json!({ "email": email, "password": password });
    router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn test_create_admin() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);
    let status = create_admin(&router, "admin@example.com", "password123").await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_admin_invalid_email() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);
    let status = create_admin(&router, "notanemail", "password123").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_admin_short_password() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);
    let status = create_admin(&router, "admin@example.com", "short").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_admin_login_success() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_admin(&router, "admin@example.com", "password123").await;

    let body = json!({ "email": "admin@example.com", "password": "password123" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins/auth-with-password")
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
    assert_eq!(json["admin"]["email"].as_str(), Some("admin@example.com"));
    assert!(json["admin"]["password"].is_null());
}

#[tokio::test]
async fn test_admin_login_wrong_password() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_admin(&router, "admin@example.com", "password123").await;

    let body = json!({ "email": "admin@example.com", "password": "wrongpassword" });
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admins/auth-with-password")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_list_admins() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_admin(&router, "a@example.com", "password123").await;
    create_admin(&router, "b@example.com", "password123").await;

    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(json["totalItems"].as_u64(), Some(2));
}

#[tokio::test]
async fn test_delete_admin() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    create_admin(&router, "admin@example.com", "password123").await;

    // get the id from list
    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admins")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    println!("list response: {}", json);
    let id = json["items"][0]["id"].as_str().unwrap().to_string();

    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/admins/{}", id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_admin_not_found() {
    let (app, _dir) = common::setup_test_app().await;
    let router = quickbase::apis::router(app);

    let res = router.clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/admins/nonexistentid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}