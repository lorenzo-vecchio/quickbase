use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    extract::{State, Path, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use crate::core::App;
use crate::db::DbError;
use crate::models::collection::Collection;
use crate::forms::collection_upsert::CollectionUpsert;

#[derive(Deserialize, Default)]
pub struct ListCollectionsQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(rename = "perPage", default = "default_per_page")]
    pub per_page: u64,
}
fn default_page() -> u64 { 1 }
fn default_per_page() -> u64 { 200 }

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(list_collections).post(create_collection))
        // `/meta/scaffolds` is the path the official PocketBase SDK uses.
        // Keep the legacy `/scaffolds` route as well for test compatibility.
        .route("/meta/scaffolds", get(get_scaffolds))
        .route("/scaffolds", get(get_scaffolds))
        .route("/{name}", get(get_collection).patch(update_collection).delete(delete_collection))
}

async fn list_collections(
    State(app): State<Arc<App>>,
    Query(q): Query<ListCollectionsQuery>,
) -> impl IntoResponse {
    match app.list_collections().await {
        Ok(collections) => {
            let total = collections.len() as u64;
            let per_page = q.per_page.max(1);
            let page = q.page.max(1);
            let total_pages = ((total as f64) / (per_page as f64)).ceil() as u64;
            let offset = ((page - 1) * per_page) as usize;
            let items: Vec<&Collection> = collections.iter().skip(offset).take(per_page as usize).collect();
            (StatusCode::OK, Json(serde_json::json!({
                "page": page,
                "perPage": per_page,
                "totalItems": total,
                "totalPages": total_pages,
                "items": items,
            }))).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response(),
    }
}

async fn get_scaffolds() -> impl IntoResponse {
    // Returns scaffold/template definitions for the UI collection editor.
    // Fields must have system:true so the UI knows they are locked system fields
    // and doesn't also auto-inject its own copies of id/created/updated.
    (StatusCode::OK, Json(serde_json::json!({
        "base": {
            "id": "",
            "name": "",
            "type": "base",
            "system": false,
            "fields": [
                {"id":"systemmbase_id","name":"id","type":"text","system":true,"primaryKey":true,"required":false,"hidden":false,"presentable":false,"options":{}}
            ],
            "indexes": [],
            "listRule": null,
            "viewRule": null,
            "createRule": null,
            "updateRule": null,
            "deleteRule": null,
            "options": {}
        },
        "auth": {
            "id": "",
            "name": "",
            "type": "auth",
            "system": false,
            "fields": [
                {"id":"systemmauth_id","name":"id","type":"text","system":true,"primaryKey":true,"required":false,"hidden":false,"presentable":false,"options":{}},
                {"id":"systemmauth_pw","name":"password","type":"password","system":true,"primaryKey":false,"required":true,"hidden":true,"presentable":false,"options":{"min":8,"pattern":""}},
                {"id":"systemmauth_tk","name":"tokenKey","type":"text","system":true,"primaryKey":false,"required":true,"hidden":true,"presentable":false,"options":{}},
                {"id":"systemmauth_em","name":"email","type":"email","system":true,"primaryKey":false,"required":false,"hidden":true,"presentable":false,"options":{"exceptDomains":[],"onlyDomains":[]}},
                {"id":"systemmauth_ev","name":"emailVisibility","type":"bool","system":true,"primaryKey":false,"required":false,"hidden":false,"presentable":false,"options":{}},
                {"id":"systemmauth_ve","name":"verified","type":"bool","system":true,"primaryKey":false,"required":false,"hidden":true,"presentable":false,"options":{}}
            ],
            "indexes": [],
            "listRule": null,
            "viewRule": null,
            "createRule": null,
            "updateRule": null,
            "deleteRule": null,
            "options": {
                "authRule": "",
                "manageRule": null,
                "authAlert": {"enabled":true,"emailTemplate":{"subject":"Login from a new location","body":"..."}},
                "oauth2": {"mappedFields":{},"enabled":false},
                "passwordAuth": {"enabled":true,"identityFields":["email"]},
                "mfa": {"enabled":false,"duration":1800,"rule":""},
                "otp": {"enabled":false,"duration":180,"length":8,"emailTemplate":{"subject":"OTP for {APP_NAME}","body":"..."}},
                "authToken": {"duration":1209600},
                "passwordResetToken": {"duration":1800},
                "emailChangeToken": {"duration":1800},
                "verificationToken": {"duration":604800},
                "fileToken": {"duration":120},
                "verificationTemplate": {"subject":"Verify your {APP_NAME} email","body":"..."},
                "resetPasswordTemplate": {"subject":"Reset your {APP_NAME} password","body":"..."},
                "confirmEmailChangeTemplate": {"subject":"Confirm your {APP_NAME} new email address","body":"..."}
            }
        },
        "view": {
            "id": "",
            "name": "",
            "type": "view",
            "system": false,
            "fields": [
                {"id":"systemmview_id","name":"id","type":"text","system":true,"primaryKey":true,"required":false,"hidden":false,"presentable":false,"options":{}}
            ],
            "indexes": [],
            "listRule": null,
            "viewRule": null,
            "createRule": null,
            "updateRule": null,
            "deleteRule": null,
            "options": {"query": ""}
        }
    })))
}

async fn get_collection(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => (StatusCode::OK, Json(c)).into_response(),
        Err(DbError::NotFound) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_collection(
    State(app): State<Arc<App>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let collection_type = body.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("base");
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let collection = match collection_type {
        "auth" => Collection::new_auth(name),
        "view" => Collection::new_view(name),
        _ => Collection::new_base(name),
    };
    let mut form = CollectionUpsert::new(&app, collection);
    form.load(body);
    match form.submit().await {
        Ok(collection) => (StatusCode::CREATED, Json(collection)).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn update_collection(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let old = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let mut form = CollectionUpsert::new(&app, old.clone());
    form.load(body);

    match form.submit_update(old).await {
        Ok(collection) => (StatusCode::OK, Json(collection)).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_collection(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match app.delete_collection(&collection).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
