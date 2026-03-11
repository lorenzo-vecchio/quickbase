use std::sync::Arc;
use axum::Router;
use crate::core::App;

pub mod collections;
pub mod records;
pub mod auth;
pub mod ui;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api/collections", collections::router())
        .nest("/api/collections/{name}/records", records::router())
        .route("/api/collections/{name}/auth-with-password", axum::routing::post(auth::auth_with_password))
        .route("/api/collections/_superusers/auth-methods", axum::routing::get(auth::auth_methods))
        // serve UI at /_/ and fallback
        .route("/", axum::routing::get(ui::serve_ui))
        .route("/_/{*path}", axum::routing::get(ui::serve_ui))
        .route("/{*path}", axum::routing::get(ui::serve_ui))
        .with_state(app)
}