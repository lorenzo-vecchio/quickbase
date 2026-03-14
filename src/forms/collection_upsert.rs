use crate::core::app::App;
use crate::models::collection::Collection;
use crate::models::schema::SchemaField;
use crate::forms::errors::FormError;
use crate::db::DbError;

/// Reads an optional access rule from the JSON body.
/// JSON `null`  → `None`  (rule cleared / no restriction)
/// JSON string  → `Some(s)` (expression rule)
/// Key absent   → `None` (keep as-is; we overwrite below so this is fine)
fn load_rule(data: &serde_json::Value, key: &str) -> Option<String> {
    data.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

pub struct CollectionUpsert<'a> {
    app: &'a App,
    collection: Collection,
}

impl<'a> CollectionUpsert<'a> {
    pub fn new(app: &'a App, collection: Collection) -> Self {
        Self { app, collection }
    }

    pub fn load(&mut self, data: serde_json::Value) {
        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
            self.collection.name = name.to_string();
        }
        if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
            if let Ok(ct) = t.parse() {
                self.collection.collection_type = ct;
            }
        }
        // Accept "fields" (PocketBase v5+) or "schema" (legacy) from the request body.
        let fields_val = data.get("fields").or_else(|| data.get("schema"));
        if let Some(fields) = fields_val {
            if let Ok(parsed) = serde_json::from_value::<Vec<SchemaField>>(fields.clone()) {
                self.collection.schema = crate::db::Json(parsed);
            }
        }
        // Load access rules — PocketBase v5 uses camelCase keys.
        // If the key is present (even as JSON null) we honour it.
        self.collection.list_rule   = load_rule(&data, "listRule");
        self.collection.view_rule   = load_rule(&data, "viewRule");
        self.collection.create_rule = load_rule(&data, "createRule");
        self.collection.update_rule = load_rule(&data, "updateRule");
        self.collection.delete_rule = load_rule(&data, "deleteRule");
    }

    pub fn validate(&self) -> Result<(), FormError> {
        let mut err = FormError::new();

        if self.collection.name.is_empty() {
            err.add("name", "validation_required", "Name is required.");
        } else {
            // Names starting with '_' are reserved for system collections.
            if self.collection.name.starts_with('_') {
                err.add(
                    "name",
                    "validation_reserved_prefix",
                    "Name cannot start with '_'. That prefix is reserved for system collections.",
                );
            }

            let valid = self.collection.name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_');
            if !valid {
                err.add(
                    "name",
                    "validation_invalid_format",
                    "Name can only contain letters, numbers, and underscores.",
                );
            }

            let reserved = ["_collections", "_migrations", "_params", "_superusers", "sqlite_master"];
            if reserved.contains(&self.collection.name.as_str()) {
                err.add("name", "validation_reserved_name", "Name is reserved.");
            }
        }

        if err.is_empty() {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Create a new collection — validates, saves to _collections, creates table.
    /// Returns the collection re-read from the DB so timestamps are accurate.
    pub async fn submit(self) -> Result<Collection, FormError> {
        self.validate()?;

        let is_new = self.collection.base.is_new();
        let collection_id = self.collection.base.id.clone();

        self.save_collection().await.map_err(|e| {
            let mut err = FormError::new();
            err.add("_", "db_error", e.to_string());
            err
        })?;

        if is_new {
            self.create_table().await.map_err(|e| {
                let mut err = FormError::new();
                err.add("_", "db_error", e.to_string());
                err
            })?;
        }

        // Re-read from DB so the response has accurate created/updated timestamps.
        self.app.find_collection_by_id(&collection_id).await.map_err(|e| {
            let mut err = FormError::new();
            err.add("_", "db_error", e.to_string());
            err
        })
    }

    /// Update an existing collection — diffs old vs new schema, adds/drops columns.
    /// Returns the collection re-read from the DB so timestamps are accurate.
    pub async fn submit_update(self, old: Collection) -> Result<Collection, FormError> {
        self.validate()?;

        let collection_id = self.collection.base.id.clone();

        let old_names: std::collections::HashSet<&str> = old
            .fields()
            .iter()
            .map(|f| f.name.as_str())
            .collect();

        let new_names: std::collections::HashSet<&str> = self
            .collection
            .fields()
            .iter()
            .map(|f| f.name.as_str())
            .collect();

        // drop removed columns
        for name in &old_names {
            if !new_names.contains(name) {
                self.drop_column(name).await.map_err(|e| {
                    let mut err = FormError::new();
                    err.add("_", "db_error", e.to_string());
                    err
                })?;
            }
        }

        // add new columns
        for field in self.collection.fields() {
            if !old_names.contains(field.name.as_str()) {
                self.add_column(field).await.map_err(|e| {
                    let mut err = FormError::new();
                    err.add("_", "db_error", e.to_string());
                    err
                })?;
            }
        }

        self.save_collection().await.map_err(|e| {
            let mut err = FormError::new();
            err.add("_", "db_error", e.to_string());
            err
        })?;

        // Re-read from DB so the response has accurate updated timestamp.
        self.app.find_collection_by_id(&collection_id).await.map_err(|e| {
            let mut err = FormError::new();
            err.add("_", "db_error", e.to_string());
            err
        })
    }

    async fn save_collection(&self) -> Result<(), DbError> {
        self.app.save_collection(&self.collection).await
    }

    async fn create_table(&self) -> Result<(), DbError> {
        let mut columns = vec![
            "id      TEXT PRIMARY KEY DEFAULT ''".to_string(),
            "created TEXT DEFAULT '' NOT NULL".to_string(),
            "updated TEXT DEFAULT '' NOT NULL".to_string(),
        ];

        // auth collections get extra system columns
        if self.collection.is_auth() {
            columns.push("email      TEXT NOT NULL DEFAULT ''".to_string());
            columns.push("password   TEXT NOT NULL DEFAULT ''".to_string());
            columns.push("tokenKey   TEXT NOT NULL DEFAULT ''".to_string());
            columns.push("verified   INTEGER NOT NULL DEFAULT 0".to_string());
        }

        for field in self.collection.fields() {
            columns.push(format!(
                "{} {} DEFAULT '' NOT NULL",
                field.name,
                field.sqlite_type()
            ));
        }

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            self.collection.name,
            columns.join(", ")
        );

        // User-created collections always go to the data pool.
        // (The validation above prevents _ prefixed names from reaching here.)
        let pool = self.app.pool_for_collection(&self.collection.name);
        sqlx::query(&sql)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn add_column(&self, field: &SchemaField) -> Result<(), DbError> {
        let sql = format!(
            "ALTER TABLE {} ADD COLUMN {} {} DEFAULT '' NOT NULL",
            self.collection.name,
            field.name,
            field.sqlite_type()
        );
        let pool = self.app.pool_for_collection(&self.collection.name);
        sqlx::query(&sql)
            .execute(pool)
            .await?;
        Ok(())
    }

    async fn drop_column(&self, column: &str) -> Result<(), DbError> {
        let sql = format!(
            "ALTER TABLE {} DROP COLUMN {}",
            self.collection.name,
            column
        );
        let pool = self.app.pool_for_collection(&self.collection.name);
        sqlx::query(&sql)
            .execute(pool)
            .await?;
        Ok(())
    }
}
