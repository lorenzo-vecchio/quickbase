use std::sync::Arc;
use axum::{Router};
use crate::core::App;

pub mod collections;
pub mod records;

pub fn router(app: Arc<App>) -> Router {
    Router::new()
        .nest("/api/collections", collections::router())
        .nest("/api/collections/{name}/records", records::router())
        .with_state(app)
}