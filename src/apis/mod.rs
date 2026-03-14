use std::sync::Arc;
use axum::Router;
use crate::core::App;

pub mod collections;
pub mod records;
pub mod auth;
pub mod admin;
pub mod logs;
pub mod ui;
pub mod files;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        // Collections CRUD
        .nest("/api/collections", collections::router())
        // Records CRUD (nested under collection)
        .nest("/api/collections/{name}/records", records::router())
        // Auth endpoints
        .route("/api/collections/{name}/auth-with-password", axum::routing::post(auth::auth_with_password))
        .route("/api/collections/{name}/auth-refresh", axum::routing::post(auth::auth_refresh))
        .route("/api/collections/{name}/auth-methods", axum::routing::get(auth::auth_methods))
        .route("/api/collections/{name}/request-verification", axum::routing::post(auth::request_verification))
        .route("/api/collections/{name}/confirm-verification", axum::routing::post(auth::confirm_verification))
        .route("/api/collections/{name}/request-password-reset", axum::routing::post(auth::request_password_reset))
        .route("/api/collections/{name}/confirm-password-reset", axum::routing::post(auth::confirm_password_reset))
        .route("/api/collections/{name}/request-email-change", axum::routing::post(auth::request_email_change))
        .route("/api/collections/{name}/confirm-email-change", axum::routing::post(auth::confirm_email_change))
        .route("/api/collections/{name}/request-otp", axum::routing::post(auth::request_otp))
        .route("/api/collections/{name}/auth-with-otp", axum::routing::post(auth::auth_with_otp))
        // Health
        .route("/api/health", axum::routing::get(health))
        // Admin routes
        .nest("/api/admin", admin::router())
        // Logs
        .nest("/api/logs", logs::router())
        // UI at /_/ and fallback
        .route("/", axum::routing::get(ui::serve_ui))
        .route("/_/{*path}", axum::routing::get(ui::serve_ui))
        .route("/{*path}", axum::routing::get(ui::serve_ui))
        .with_state(app)
}

async fn health() -> axum::response::Response {
    axum::response::IntoResponse::into_response(
        (axum::http::StatusCode::OK, axum::Json(serde_json::json!({
            "code": 200,
            "message": "API is healthy."
        })))
    )
}
