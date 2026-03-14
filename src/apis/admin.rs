use std::sync::Arc;
use axum::{
    Router,
    routing::{get, post, delete},
    extract::{State, Path},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use crate::core::App;

pub fn router() -> Router<Arc<App>> {
    Router::new()
        // Health
        .route("/health", get(health))
        // Settings
        .route("/settings", get(get_settings).patch(update_settings))
        // Backups
        .route("/backups", get(list_backups).post(create_backup))
        .route("/backups/{name}", delete(delete_backup))
        .route("/backups/{name}/restore", post(restore_backup))
        // Crons
        .route("/crons", get(list_crons))
        .route("/crons/{id}", post(run_cron))
        // Files token
        .route("/files/token", post(files_token))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({
        "code": 200,
        "message": "API is healthy."
    })))
}

async fn get_settings(State(app): State<Arc<App>>) -> impl IntoResponse {
    // Load settings from _params table
    let result = sqlx::query_as::<_, (String, String)>(
        "SELECT key, value FROM _params ORDER BY key ASC"
    )
    .fetch_all(&app.pools().system)
    .await;

    let mut settings = serde_json::json!({
        "meta": {
            "appName": "Quickbase",
            "appUrl": "",
            "hideControls": false,
            "senderName": "Support",
            "senderAddress": "support@example.com",
        },
        "smtp": {
            "enabled": false,
            "host": "",
            "port": 587,
            "username": "",
            "password": "",
            "authMethod": "PLAIN",
            "tls": true,
            "localName": ""
        },
        "s3": {
            "enabled": false,
            "bucket": "",
            "region": "",
            "endpoint": "",
            "accessKey": "",
            "secret": "",
            "forcePathStyle": false
        },
        "backupsS3": {
            "enabled": false,
            "bucket": "",
            "region": "",
            "endpoint": "",
            "accessKey": "",
            "secret": "",
            "forcePathStyle": false
        },
        "logs": {
            "maxDays": 5,
            "minLevel": 0,
            "logIp": true
        },
        "batch": {
            "enabled": true,
            "maxRequests": 50,
            "timeout": 3,
            "maxBodySize": 16000000
        },
        "rateLimits": {
            "enabled": false,
            "rules": []
        },
        "trustedProxy": {
            "headers": [],
            "useLeftmostIp": false
        }
    });

    // Override with stored values
    if let Ok(rows) = result {
        for (key, value) in rows {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&value) {
                if let Some(obj) = settings.as_object_mut() {
                    // key format: "section.field" or "section"
                    let parts: Vec<&str> = key.splitn(2, '.').collect();
                    if parts.len() == 2 {
                        if let Some(section) = obj.get_mut(parts[0]) {
                            if let Some(section_obj) = section.as_object_mut() {
                                section_obj.insert(parts[1].to_string(), v);
                            }
                        }
                    } else if parts.len() == 1 {
                        obj.insert(key, v);
                    }
                }
            }
        }
    }

    (StatusCode::OK, Json(settings))
}

async fn update_settings(
    State(app): State<Arc<App>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Store settings in _params table, flattened as "section.field"
    if let Some(obj) = body.as_object() {
        for (section, value) in obj {
            let key = section.clone();
            let val = serde_json::to_string(value).unwrap_or_default();
            let _ = sqlx::query(
                "INSERT OR REPLACE INTO _params (key, value) VALUES (?, ?)"
            )
            .bind(&key)
            .bind(&val)
            .execute(&app.pools().system)
            .await;
        }
    }

    get_settings(State(app)).await.into_response()
}

async fn list_backups() -> impl IntoResponse {
    // No backup implementation yet — return empty list
    (StatusCode::OK, Json(serde_json::json!([])))
}

async fn create_backup() -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

async fn delete_backup(Path(_name): Path<String>) -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

async fn restore_backup(Path(_name): Path<String>) -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

async fn list_crons() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!([])))
}

async fn run_cron(Path(_id): Path<String>) -> impl IntoResponse {
    (StatusCode::NO_CONTENT, "")
}

async fn files_token(State(_app): State<Arc<App>>) -> impl IntoResponse {
    // Generate a short-lived token for file access
    let token = crate::tools::security::generate_id();
    (StatusCode::OK, Json(serde_json::json!({
        "token": token
    })))
}
