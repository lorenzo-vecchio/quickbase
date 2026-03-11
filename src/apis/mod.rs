use std::sync::Arc;
use axum::{Router};
use crate::core::App;

pub mod collections;
pub mod records;
pub mod auth;
pub mod admins;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api/collections", collections::router())
        .nest("/api/collections/{name}/records", records::router())
        .route("/api/collections/{name}/auth-with-password", axum::routing::post(auth::auth_with_password))
        .nest("/api/admins", admins::router())
        .with_state(app)
}