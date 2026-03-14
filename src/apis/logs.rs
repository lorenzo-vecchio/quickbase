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

pub fn router() -> Router<Arc<App>> {
    Router::new()
        .route("/", get(list_logs))
        .route("/stats", get(logs_stats))
        .route("/{id}", get(get_log))
}

#[derive(Deserialize, Default)]
pub struct LogsQuery {
    #[serde(default = "default_page")]
    pub page: u64,
    #[serde(rename = "perPage", default = "default_per_page")]
    pub per_page: u64,
    #[serde(default)]
    pub filter: String,
}

fn default_page() -> u64 { 1 }
fn default_per_page() -> u64 { 30 }

/// Ensures the _logs table exists in system.db.
async fn ensure_logs_table(app: &Arc<App>) -> Result<(), sqlx::Error> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _logs (
            id      TEXT PRIMARY KEY DEFAULT ('r'||lower(hex(randomblob(7)))),
            level   INTEGER NOT NULL DEFAULT 0,
            message TEXT NOT NULL DEFAULT '',
            data    TEXT NOT NULL DEFAULT '{}',
            created TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%fZ'))
        )"
    )
    .execute(&app.pools().system)
    .await?;
    Ok(())
}

async fn list_logs(
    State(app): State<Arc<App>>,
    Query(q): Query<LogsQuery>,
) -> impl IntoResponse {
    if let Err(e) = ensure_logs_table(&app).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response();
    }

    let per_page = q.per_page.min(500).max(1);
    let page = q.page.max(1);
    let offset = (page - 1) * per_page;

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _logs")
        .fetch_one(&app.pools().system)
        .await
        .unwrap_or(0);

    let rows = sqlx::query_as::<_, (String, i64, String, String, String)>(
        "SELECT id, level, message, data, created FROM _logs ORDER BY created DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&app.pools().system)
    .await;

    match rows {
        Ok(rows) => {
            let items: Vec<serde_json::Value> = rows.into_iter().map(|(id, level, message, data, created)| {
                let data_val: serde_json::Value = serde_json::from_str(&data).unwrap_or(serde_json::json!({}));
                serde_json::json!({
                    "id": id,
                    "level": level,
                    "message": message,
                    "data": data_val,
                    "created": created,
                })
            }).collect();

            let total_pages = ((total as f64) / (per_page as f64)).ceil() as i64;
            (StatusCode::OK, Json(serde_json::json!({
                "page": page,
                "perPage": per_page,
                "totalItems": total,
                "totalPages": total_pages,
                "items": items,
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response(),
    }
}

async fn get_log(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = ensure_logs_table(&app).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response();
    }

    let row = sqlx::query_as::<_, (String, i64, String, String, String)>(
        "SELECT id, level, message, data, created FROM _logs WHERE id = ?"
    )
    .bind(&id)
    .fetch_optional(&app.pools().system)
    .await;

    match row {
        Ok(Some((id, level, message, data, created))) => {
            let data_val: serde_json::Value = serde_json::from_str(&data).unwrap_or(serde_json::json!({}));
            (StatusCode::OK, Json(serde_json::json!({
                "id": id,
                "level": level,
                "message": message,
                "data": data_val,
                "created": created,
            }))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"message": "log not found"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response(),
    }
}

async fn logs_stats(State(app): State<Arc<App>>) -> impl IntoResponse {
    if let Err(e) = ensure_logs_table(&app).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response();
    }

    // Return per-day stats for the last 30 days
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT date(created) as day, COUNT(*) as count
         FROM _logs
         WHERE created >= datetime('now', '-30 days')
         GROUP BY day
         ORDER BY day ASC"
    )
    .fetch_all(&app.pools().system)
    .await;

    match rows {
        Ok(rows) => {
            let stats: Vec<serde_json::Value> = rows.into_iter().map(|(day, count)| {
                serde_json::json!({"date": day, "total": count})
            }).collect();
            (StatusCode::OK, Json(serde_json::json!(stats))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"message": e.to_string()}))).into_response(),
    }
}
