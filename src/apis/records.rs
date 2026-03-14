use std::sync::Arc;
use std::collections::HashMap;
use axum::{
    Router,
    routing::get,
    extract::{State, Path, Query},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
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

#[derive(Deserialize, Default)]
pub struct ListQuery {
    /// Page number (1-based). Default: 1.
    #[serde(default = "default_page")]
    pub page: u64,
    /// Items per page. Default: 30. Max: 500.
    #[serde(rename = "perPage", default = "default_per_page")]
    pub per_page: u64,
    /// Sort expression, e.g. "-created,name". Default: "-created".
    #[serde(default)]
    pub sort: String,
    /// Filter expression in PocketBase filter syntax.
    #[serde(default)]
    pub filter: String,
    /// Comma-separated list of fields to return. Empty = all fields.
    #[serde(default)]
    pub fields: String,
    /// Whether to skip counting total items (faster queries).
    /// Accepts "true"/"false" or "1"/"0" (PocketBase SDK sends integers).
    #[serde(rename = "skipTotal", default = "default_skip_total", deserialize_with = "bool_from_str")]
    pub skip_total: bool,
}

fn default_page() -> u64 { 1 }
fn default_per_page() -> u64 { 30 }
fn default_skip_total() -> bool { false }

/// Deserializes a bool from a query-string value that may be "true"/"false"
/// OR "1"/"0" (which the PocketBase SDK sends for skipTotal).
fn bool_from_str<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" | "" => Ok(false),
        _ => Err(serde::de::Error::custom(format!("cannot parse '{}' as bool", s))),
    }
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

/// Converts a PocketBase sort string like "-created,name" into SQL ORDER BY clause.
/// Prefix "-" means DESC, no prefix means ASC.
fn build_order_by(sort: &str, table: &str) -> String {
    if sort.is_empty() {
        return format!("{}.created DESC", table);
    }
    let result = sort.split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() { return None; }
            if let Some(col) = part.strip_prefix('-') {
                // basic identifier validation — only allow alnum + underscore
                if col.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    Some(format!("{} DESC", col))
                } else {
                    None
                }
            } else if part.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '+') {
                let col = part.strip_prefix('+').unwrap_or(part);
                Some(format!("{} ASC", col))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    
    if result.is_empty() { 
        format!("{}.created DESC", table) 
    } else { 
        result 
    }
}

/// Very basic PocketBase filter → SQL WHERE converter.
/// Handles: field = 'val', field != 'val', field ~ 'val', field !~ 'val',
/// field > val, field < val, field >= val, field <= val,
/// && (AND), || (OR), and parentheses passthrough.
fn build_where_clause(filter: &str) -> Option<String> {
    if filter.is_empty() {
        return None;
    }

    // Replace PocketBase operators with SQL equivalents.
    // This is a simple token-based transform, not a full parser.
    let sql = filter
        .replace("&&", "AND")
        .replace("||", "OR")
        // ~ is LIKE with wildcards
        .replace("~ '", "LIKE '%' || '")
        // !~ is NOT LIKE
        .replace("!~ '", "NOT LIKE '%' || '");

    // Very naive: just pass it through if it only contains safe characters.
    // For a production system you'd use a proper parser.
    let allowed: bool = sql.chars().all(|c| {
        c.is_alphanumeric() || " _.'\"=!<>()%|ANDORLIKENOTandorlikenot\n\t+-*/".contains(c)
    });

    if allowed {
        Some(sql)
    } else {
        None // reject unsafe filter
    }
}

async fn list_records(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    };

    let pool = app.pool_for_collection(&collection.name);
    let per_page = q.per_page.min(500).max(1);
    let page = q.page.max(1);
    let offset = (page - 1) * per_page;

    let order = build_order_by(&q.sort, &collection.name);
    let where_clause = build_where_clause(&q.filter);

    let base_sql = if let Some(ref w) = where_clause {
        format!("FROM {} WHERE {}", collection.name, w)
    } else {
        format!("FROM {}", collection.name)
    };

    // Count total (skip if skipTotal=true)
    let total_items: i64 = if q.skip_total {
        -1
    } else {
        let count_sql = format!("SELECT COUNT(*) {}", base_sql);
        match sqlx::query_scalar::<_, i64>(&count_sql).fetch_one(pool).await {
            Ok(n) => n,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
        }
    };

    let data_sql = format!("SELECT * {} ORDER BY {} LIMIT ? OFFSET ?", base_sql, order);
    let rows = sqlx::query(&data_sql)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(pool)
        .await;

    match rows {
        Ok(rows) => {
            let items: Vec<Record> = rows
                .iter()
                .map(|row| {
                    let data = row_to_map(row);
                    let mut r = Record::new(collection.clone());
                    r.load(data);
                    r
                })
                .collect();

            let total_pages = if q.skip_total || total_items < 0 {
                0i64
            } else {
                ((total_items as f64) / (per_page as f64)).ceil() as i64
            };

            (StatusCode::OK, Json(serde_json::json!({
                "page": page,
                "perPage": per_page,
                "totalItems": total_items,
                "totalPages": total_pages,
                "items": items,
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    }
}

async fn get_record(
    State(app): State<Arc<App>>,
    Path((name, id)): Path<(String, String)>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    };

    let pool = app.pool_for_collection(&collection.name);
    let sql = format!("SELECT * FROM {} WHERE id = ?", collection.name);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(pool)
        .await;

    match row {
        Ok(Some(row)) => {
            let data = row_to_map(&row);
            let mut r = Record::new(collection);
            r.load(data);
            (StatusCode::OK, Json(r)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"record not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    }
}

async fn create_record(
    State(app): State<Arc<App>>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
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
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    };

    // fetch the existing record
    let pool = app.pool_for_collection(&collection.name);
    let sql = format!("SELECT * FROM {} WHERE id = ?", collection.name);
    let row = sqlx::query(&sql)
        .bind(&id)
        .fetch_optional(pool)
        .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"record not found"}))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
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
    let collection = match app.find_collection_by_name_or_id(&name).await {
        Ok(c) => c,
        Err(DbError::NotFound) => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"collection not found"}))).into_response()
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    };

    let pool = app.pool_for_collection(&collection.name);
    let sql = format!("DELETE FROM {} WHERE id = ?", collection.name);
    match sqlx::query(&sql)
        .bind(&id)
        .execute(pool)
        .await
    {
        Ok(result) if result.rows_affected() == 0 => {
            (StatusCode::NOT_FOUND, Json(serde_json::json!({"message":"record not found"}))).into_response()
        }
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message":e.to_string()}))).into_response(),
    }
}
