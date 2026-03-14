use axum::{
    extract::{Path, State},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::core::app::App;
use sqlx::{Column, Row};

#[derive(Deserialize)]
pub struct PasswordLoginPayload {
    /// Can be email or username, depending on the collection's identityFields setting.
    /// Matches PocketBase's `identity` field name.
    pub identity: String,
    pub password: String,
}

pub async fn auth_with_password(
    State(app): State<Arc<App>>,
    Path(collection_name): Path<String>,
    Json(payload): Json<PasswordLoginPayload>,
) -> impl IntoResponse {
    // 1. find the collection
    let collection = match app.find_collection_by_name_or_id(&collection_name).await {
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

    // 3. look up record by email (identity = email for now)
    //    Route to the right pool: _superusers lives in system.db, user collections in data.db.
    let pool = app.pool_for_collection(&collection.name);
    let row = sqlx::query(
        &format!("SELECT * FROM {} WHERE email = ? LIMIT 1", collection.name)
    )
    .bind(&payload.identity)
    .fetch_optional(pool)
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

    // 6. return token + record (without password / tokenKey)
    let mut record_json = serde_json::Map::new();
    // Inject PocketBase-required collection fields.
    record_json.insert("collectionId".to_string(), serde_json::Value::String(collection.id().to_string()));
    record_json.insert("collectionName".to_string(), serde_json::Value::String(collection.name.clone()));
    for column in row.columns() {
        let name = column.name();
        if name == "password" || name == "tokenKey" {
            continue;
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

pub async fn auth_refresh(
    State(app): State<Arc<App>>,
    Path(collection_name): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 1. extract Bearer token
    let token = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "status": 401,
            "message": "The request requires valid record authorization token.",
            "data": {}
        }))).into_response(),
    };

    // 2. verify the token
    let claims = match crate::tools::security::verify_auth_token(&token) {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "status": 401,
            "message": "The request requires valid record authorization token.",
            "data": {}
        }))).into_response(),
    };

    // 3. find the collection
    let collection = match app.find_collection_by_name_or_id(&collection_name).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "status": 404,
            "message": "Missing collection context.",
            "data": {}
        }))).into_response(),
    };

    // 4. fetch the record from the correct pool
    let pool = app.pool_for_collection(&collection.name);
    let row = sqlx::query(&format!("SELECT * FROM {} WHERE id = ? LIMIT 1", collection.name))
        .bind(&claims.id)
        .fetch_optional(pool)
        .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "status": 401,
            "message": "The request requires valid record authorization token.",
            "data": {}
        }))).into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "database error"
        }))).into_response(),
    };

    // 5. issue a fresh token
    let record_id: String = row.try_get("id").unwrap_or_default();
    let new_token = match crate::tools::security::generate_auth_token(&record_id, &collection_name) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "failed to generate token"
        }))).into_response(),
    };

    // 6. return token + record (without password / tokenKey)
    let mut record_json = serde_json::Map::new();
    // Inject PocketBase-required collection fields.
    record_json.insert("collectionId".to_string(), serde_json::Value::String(collection.id().to_string()));
    record_json.insert("collectionName".to_string(), serde_json::Value::String(collection.name.clone()));
    for column in row.columns() {
        let name = column.name();
        if name == "password" || name == "tokenKey" {
            continue;
        }
        let val: serde_json::Value = row.try_get(name)
            .map(|s: String| serde_json::Value::String(s))
            .unwrap_or(serde_json::Value::Null);
        record_json.insert(name.to_string(), val);
    }

    (StatusCode::OK, Json(serde_json::json!({
        "token": new_token,
        "record": record_json,
    }))).into_response()
}

pub async fn auth_methods(
    State(app): State<Arc<App>>,
    Path(collection_name): Path<String>,
) -> impl IntoResponse {
    // Verify the collection exists and is an auth collection.
    let collection = match app.find_collection_by_name_or_id(&collection_name).await {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({
            "message": "collection not found"
        }))).into_response(),
    };

    if !collection.is_auth() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "message": "collection is not an auth collection"
        }))).into_response();
    }

    (StatusCode::OK, Json(serde_json::json!({
        "mfa": { "enabled": false, "duration": 0 },
        "otp": { "enabled": false, "duration": 0 },
        "password": { "enabled": true, "identityFields": ["email"] },
        "oauth2": { "enabled": false, "providers": [] }
    }))).into_response()
}

// ─── Additional auth endpoints (stub implementations) ───────────────────────

pub async fn request_verification(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // In a real implementation, this would send a verification email.
    StatusCode::NO_CONTENT
}

pub async fn confirm_verification(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn request_password_reset(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn confirm_password_reset(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn request_email_change(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn confirm_email_change(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

pub async fn request_otp(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "otpId": ""
    })))
}

pub async fn auth_with_otp(
    State(_app): State<Arc<App>>,
    Path(_collection_name): Path<String>,
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
        "message": "OTP not enabled"
    })))
}
