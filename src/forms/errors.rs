use std::collections::HashMap;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;

/// A structured validation error that maps field names to error messages.
/// Mirrors PocketBase's validation error response shape:
/// {"data": {"name": {"code": "...", "message": "..."}}}
#[derive(Debug, Serialize)]
pub struct FormError {
    pub data: HashMap<String, FieldError>,
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub code: String,
    pub message: String,
}

impl FormError {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn add(&mut self, field: impl Into<String>, code: impl Into<String>, message: impl Into<String>) {
        self.data.insert(field.into(), FieldError {
            code: code.into(),
            message: message.into(),
        });
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl IntoResponse for FormError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}