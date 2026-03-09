use crate::core::app::App;
use crate::models::collection::Collection;
use crate::models::schema::SchemaField;
use crate::forms::errors::FormError;
use crate::db::DbError;

pub struct CollectionUpsert<'a> {
    app: &'a App,
    collection: Collection,
}

impl<'a> CollectionUpsert<'a> {
    /// Mirrors PocketBase's forms.NewCollectionUpsert(app, collection).
    pub fn new(app: &'a App, collection: Collection) -> Self {
        Self { app, collection }
    }

    /// Populate form fields from a raw JSON body.
    /// Mirrors PocketBase's form.LoadData(data).
    pub fn load(&mut self, data: serde_json::Value) {
        if let Some(name) = data.get("name").and_then(|v| v.as_str()) {
            self.collection.name = name.to_string();
        }
        if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
            if let Ok(ct) = t.parse() {
                self.collection.collection_type = ct;
            }
        }
        if let Some(schema) = data.get("schema") {
            if let Ok(fields) = serde_json::from_value::<Vec<SchemaField>>(schema.clone()) {
                self.collection.schema = crate::db::Json(fields);
            }
        }
    }

    /// Validate the collection fields.
    /// Returns a FormError with one entry per invalid field.
    pub fn validate(&self) -> Result<(), FormError> {
        let mut err = FormError::new();

        if self.collection.name.is_empty() {
            err.add("name", "validation_required", "Name is required.");
        } else {
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

            let reserved = ["_collections", "_migrations", "sqlite_master"];
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

    /// Validate and persist the collection.
    /// Creates the SQLite table if this is a new collection.
    /// Mirrors PocketBase's form.Submit().
    pub async fn submit(self) -> Result<Collection, FormError> {
        self.validate()?;

        let is_new = self.collection.base.is_new();

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

        Ok(self.collection)
    }

    async fn save_collection(&self) -> Result<(), DbError> {
        sqlx::query(
            "INSERT OR REPLACE INTO _collections (id, name, type, schema, indexes, options)
             VALUES (?, ?, ?, ?, ?, ?)"
        )
            .bind(&self.collection.base.id)
            .bind(&self.collection.name)
            .bind(self.collection.collection_type.to_string())
            .bind(serde_json::to_string(&*self.collection.schema).unwrap_or_default())
            .bind(serde_json::to_string(&*self.collection.indexes).unwrap_or_default())
            .bind(serde_json::to_string(&*self.collection.options).unwrap_or_default())
            .execute(&self.app.pools().data)
            .await?;
        Ok(())
    }

    async fn create_table(&self) -> Result<(), DbError> {
        let mut columns = vec![
            "id      TEXT PRIMARY KEY DEFAULT ''".to_string(),
            "created TEXT DEFAULT '' NOT NULL".to_string(),
            "updated TEXT DEFAULT '' NOT NULL".to_string(),
        ];

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

        sqlx::query(&sql)
            .execute(&self.app.pools().data)
            .await?;
        Ok(())
    }
}