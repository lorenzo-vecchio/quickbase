use std::sync::Arc;
use axum::{
    Router,
    routing::get,
    extract::{State, Path},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use crate::core::App;
use crate::db::DbError;
use crate::models::collection::Collection;
use crate::forms::collection_upsert::CollectionUpsert;

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(list_collections).post(create_collection))
        .route("/{name}", get(get_collection).patch(update_collection).delete(delete_collection))
}

async fn list_collections(State(app): State<Arc<App>>) -> impl IntoResponse {
    match app.list_collections().await {
        Ok(collections) => (StatusCode::OK, Json(collections)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_collection(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match app.find_collection_by_name(&name).await {
        Ok(c) => (StatusCode::OK, Json(c)).into_response(),
        Err(DbError::NotFound) => (StatusCode::NOT_FOUND, "collection not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_collection(
    State(app): State<Arc<App>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let collection = Collection::new_base(name);
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
    let old = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
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
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    match app.delete_collection(&collection).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}