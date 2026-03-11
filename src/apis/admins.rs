use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum::routing::{get, post};
use serde::Deserialize;
use std::sync::Arc;
use sqlx::Row;
use crate::core::app::App;
use crate::models::admin::Admin;
use crate::forms::admin_upsert::AdminUpsert;

#[derive(Deserialize)]
pub struct CreateAdminPayload {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct UpdateAdminPayload {
    pub email: Option<String>,
    pub password: Option<String>,
    pub avatar: Option<i64>,
}

#[derive(Deserialize)]
pub struct PasswordLoginPayload {
    pub email: String,
    pub password: String,
}

// POST /api/admins/auth-with-password
pub async fn auth_with_password(
    State(app): State<Arc<App>>,
    Json(payload): Json<PasswordLoginPayload>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT * FROM _admins WHERE email = ? LIMIT 1")
        .bind(&payload.email)
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

    let stored_hash: String = row.try_get("password").unwrap_or_default();
    if !crate::tools::security::verify_password(&payload.password, &stored_hash) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "message": "invalid email or password"
        }))).into_response();
    }

    let id: String = row.try_get("id").unwrap_or_default();
    let token = match crate::tools::security::generate_auth_token(&id, "_admins") {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "failed to generate token"
        }))).into_response(),
    };

    let email: String = row.try_get("email").unwrap_or_default();
    let avatar: i64 = row.try_get("avatar").unwrap_or_default();
    let created: String = row.try_get("created").unwrap_or_default();
    let updated: String = row.try_get("updated").unwrap_or_default();

    (StatusCode::OK, Json(serde_json::json!({
        "token": token,
        "admin": {
            "id": id,
            "email": email,
            "avatar": avatar,
            "created": created,
            "updated": updated,
        }
    }))).into_response()
}

// POST /api/admins/auth-refresh
pub async fn auth_refresh(
    State(_app): State<Arc<App>>,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let token = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "message": "missing or invalid authorization header"
        }))).into_response(),
    };

    let claims = match crate::tools::security::verify_auth_token(&token) {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "message": "invalid or expired token"
        }))).into_response(),
    };

    let new_token = match crate::tools::security::generate_auth_token(&claims.sub, &claims.collection) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "failed to generate token"
        }))).into_response(),
    };

    (StatusCode::OK, Json(serde_json::json!({ "token": new_token }))).into_response()
}

// POST /api/admins
pub async fn create_admin(
    State(app): State<Arc<App>>,
    Json(payload): Json<CreateAdminPayload>,
) -> impl IntoResponse {
    let form = AdminUpsert::new_create(&app, payload.email, payload.password);
    match form.submit().await {
        Ok(admin) => (StatusCode::CREATED, Json(serde_json::to_value(&admin).unwrap())).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::to_value(&e).unwrap())).into_response(),
    }
}

// GET /api/admins
pub async fn list_admins(
    State(app): State<Arc<App>>,
) -> impl IntoResponse {
    let admins = sqlx::query_as::<_, Admin>("SELECT * FROM _admins ORDER BY created ASC")
        .fetch_all(&app.pools().data)
        .await;

    match admins {
        Ok(list) => (StatusCode::OK, Json(serde_json::json!({
            "items": list,
            "totalItems": list.len(),
        }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({
            "message": "database error"
        }))).into_response(),
    }
}

// GET /api/admins/:id
pub async fn get_admin(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let admin = sqlx::query_as::<_, Admin>("SELECT * FROM _admins WHERE id = ? LIMIT 1")
        .bind(&id)
        .fetch_optional(&app.pools().data)
        .await;

    match admin {
        Ok(Some(a)) => (StatusCode::OK, Json(serde_json::to_value(&a).unwrap())).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "message": "admin not found" }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "message": "database error" }))).into_response(),
    }
}

// PATCH /api/admins/:id
pub async fn update_admin(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateAdminPayload>,
) -> impl IntoResponse {
    let admin = sqlx::query_as::<_, Admin>("SELECT * FROM _admins WHERE id = ? LIMIT 1")
        .bind(&id)
        .fetch_optional(&app.pools().data)
        .await;

    let mut admin = match admin {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({ "message": "admin not found" }))).into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "message": "database error" }))).into_response(),
    };

    if let Some(email) = payload.email { admin.email = email; }
    if let Some(avatar) = payload.avatar { admin.avatar = avatar; }

    let form = AdminUpsert::new_update(&app, admin, payload.password);
    match form.submit().await {
        Ok(a) => (StatusCode::OK, Json(serde_json::to_value(&a).unwrap())).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::to_value(&e).unwrap())).into_response(),
    }
}

// DELETE /api/admins/:id
pub async fn delete_admin(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM _admins WHERE id = ?")
        .bind(&id)
        .execute(&app.pools().data)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "message": "admin not found" }))).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "message": "database error" }))).into_response(),
    }
}

pub fn router() -> axum::Router<Arc<App>> {
    axum::Router::new()
        .route("/auth-with-password", post(auth_with_password))
        .route("/auth-refresh", post(auth_refresh))
        .route("/", post(create_admin).get(list_admins))
        .route("/{id}", get(get_admin).patch(update_admin).delete(delete_admin))
}