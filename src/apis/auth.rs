use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::core::app::App;
use sqlx::Column;

#[derive(Deserialize)]
pub struct PasswordLoginPayload {
    pub identity: String,
    pub password: String,
}

pub async fn auth_with_password(
    State(app): State<Arc<App>>,
    Path(collection_name): Path<String>,
    Json(payload): Json<PasswordLoginPayload>,
) -> impl IntoResponse {
    // 1. find the collection
    let collection = match app.find_collection_by_name(&collection_name).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "message": "collection not found"
        }))).into_response(),
    };

    // 2. must be an auth collection
    if !collection.is_auth() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "message": "collection is not an auth collection"
        }))).into_response();
    }

    // 3. look up record by email
    let row = sqlx::query(
        &format!("SELECT * FROM {} WHERE email = ? LIMIT 1", collection.name)
    )
        .bind(&payload.identity)
        .fetch_optional(&app.pools().data)
        .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "message": "invalid email or password"
        }))).into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "database error"
        }))).into_response(),
    };

    // 4. verify password
    use sqlx::Row;
    let stored_hash: String = row.try_get("password").unwrap_or_default();

    if !crate::tools::security::verify_password(&payload.password, &stored_hash) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "message": "invalid email or password"
        }))).into_response();
    }

    // 5. generate JWT
    let record_id: String = row.try_get("id").unwrap_or_default();
    let token = match crate::tools::security::generate_auth_token(&record_id, &collection_name) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "failed to generate token"
        }))).into_response(),
    };

    // 6. return token + record (without password)
    let mut record_json = serde_json::Map::new();
    for column in row.columns() {
        let name = column.name();
        if name == "password" || name == "tokenKey" {
            continue; // never expose these
        }
        let val: serde_json::Value = row.try_get(name)
            .map(|s: String| serde_json::Value::String(s))
            .unwrap_or(serde_json::Value::Null);
        record_json.insert(name.to_string(), val);
    }

    (StatusCode::OK, Json(serde_json::json!({
        "token": token,
        "record": record_json,
    }))).into_response()
}

pub async fn auth_methods() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "mfa": { "enabled": false, "duration": 0 },
        "otp": { "enabled": false, "duration": 0 },
        "password": { "enabled": true, "identityFields": ["email"] },
        "oauth2": { "enabled": false, "providers": [] }
    })))
}