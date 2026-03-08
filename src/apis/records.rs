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
use crate::models::record::Record;

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(list_records))
        .route("/{id}", get(get_record))
}

async fn list_records(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };

    let sql = format!("SELECT * FROM {} ORDER BY created DESC", collection.name);
    let rows = sqlx::query(&sql)
        .fetch_all(&app.pools().data)
        .await;

    match rows {
        Ok(rows) => {
            let records: Vec<Record> = rows
                .into_iter()
                .map(|_row| {
                    let mut r = Record::new(collection.clone());
                    r.load(std::collections::HashMap::new());
                    r
                })
                .collect();
            (StatusCode::OK, Json(records)).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_record(
    State(app): State<Arc<App>>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    };

    let sql = format!("SELECT * FROM {} WHERE id = ?", collection.name);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&app.pools().data)
        .await;

    match row {
        Ok(Some(_row)) => {
            let mut r = Record::new(collection);
            r.base.id = id;
            r.base.mark_as_not_new();
            (StatusCode::OK, Json(r)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "record not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}