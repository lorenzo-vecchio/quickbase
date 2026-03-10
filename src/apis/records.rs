use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Router,
    routing::get,
    extract::{State, Path},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use sqlx::Row;
use crate::core::App;
use crate::db::DbError;
use crate::models::record::Record;
use crate::forms::record_upsert::RecordUpsert;

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(list_records).post(create_record))
        .route("/{id}", get(get_record).patch(update_record).delete(delete_record))
}

fn row_to_map(row: &sqlx::sqlite::SqliteRow) -> HashMap<String, serde_json::Value> {
    use sqlx::Column;
    let mut map = HashMap::new();
    for col in row.columns() {
        let name = col.name().to_string();
        let value: serde_json::Value = row
            .try_get::<Option<String>, _>(col.ordinal())
            .ok()
            .flatten()
            .map(|s| {
                // try to parse as JSON first (for schema, options etc.)
                serde_json::from_str(&s).unwrap_or(serde_json::Value::String(s))
            })
            .or_else(|| {
                row.try_get::<Option<i64>, _>(col.ordinal())
                    .ok()
                    .flatten()
                    .map(|n| serde_json::json!(n))
            })
            .or_else(|| {
                row.try_get::<Option<f64>, _>(col.ordinal())
                    .ok()
                    .flatten()
                    .map(|n| serde_json::json!(n))
            })
            .unwrap_or(serde_json::Value::Null);
        map.insert(name, value);
    }
    map
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
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let sql = format!("SELECT * FROM {} ORDER BY created DESC", collection.name);
    let rows = sqlx::query(&sql)
        .fetch_all(&app.pools().data)
        .await;

    match rows {
        Ok(rows) => {
            let records: Vec<Record> = rows
                .iter()
                .map(|row| {
                    let data = row_to_map(row);
                    let mut r = Record::new(collection.clone());
                    r.load(data);
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
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let sql = format!("SELECT * FROM {} WHERE id = ?", collection.name);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&app.pools().data)
        .await;

    match row {
        Ok(Some(row)) => {
            let data = row_to_map(&row);
            let mut r = Record::new(collection);
            r.load(data);
            (StatusCode::OK, Json(r)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "record not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn create_record(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let record = Record::new(collection);
    let mut form = RecordUpsert::new(&app, record);
    form.load(body);
    match form.submit().await {
        Ok(record) => (StatusCode::CREATED, Json(record)).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn update_record(
    State(app): State<Arc<App>>,
    Path((name, id)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    // fetch the existing record
    let sql = format!("SELECT * FROM {} WHERE id = ?", collection.name);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(&app.pools().data)
        .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "record not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let data = row_to_map(&row);
    let mut record = Record::new(collection);
    record.load(data);

    let mut form = RecordUpsert::new(&app, record);
    form.load(body);
    match form.submit().await {
        Ok(record) => (StatusCode::OK, Json(record)).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_record(
    State(app): State<Arc<App>>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, "collection not found").into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    let sql = format!("DELETE FROM {} WHERE id = ?", collection.name);
    match sqlx::query(&sql)
        .bind(&id)
        .execute(&app.pools().data)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => {
            (StatusCode::NOT_FOUND, "record not found").into_response()
        }
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}