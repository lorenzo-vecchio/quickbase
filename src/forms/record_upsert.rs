use crate::core::app::App;
use crate::models::record::Record;
use crate::forms::errors::FormError;
use crate::db::DbError;
use std::collections::HashMap;
use serde_json::Value;

pub struct RecordUpsert<'a> {
    app: &'a App,
    record: Record,
    data: HashMap<String, Value>,
    password: Option<String>,
    password_confirm: Option<String>,
}

impl<'a> RecordUpsert<'a> {
    pub fn new(app: &'a App, record: Record) -> Self {
        Self {
            app,
            record,
            data: HashMap::new(),
            password: None,
            password_confirm: None,
        }
    }

    /// Load field values from a raw JSON body.
    pub fn load(&mut self, body: serde_json::Value) {
        if let Some(map) = body.as_object() {
            for (k, v) in map {
                // never allow the client to set system fields
                if matches!(k.as_str(), "id" | "created" | "updated") {
                    continue;
                }
                // for auth collections, pull password out separately
                if k == "password" && self.record.collection.is_auth() {
                    if let Some(s) = v.as_str() {
                        self.password = Some(s.to_string());
                    }
                    continue;
                }
                if k == "passwordConfirm" && self.record.collection.is_auth() {
                    if let Some(s) = v.as_str() {
                        self.password_confirm = Some(s.to_string());
                    }
                    continue;
                }
                self.data.insert(k.clone(), v.clone());
            }
        }
    }

    /// Validate the record fields.
    pub fn validate(&self) -> Result<(), FormError> {
        use crate::models::schema::FieldType;

        let mut err = FormError::new();

        for field in self.record.collection.fields() {
            let value = self.data.get(&field.name);

            if field.required {
                let is_empty = match value {
                    None => true,
                    Some(Value::Null) => true,
                    Some(Value::String(s)) => s.is_empty(),
                    _ => false,
                };
                if is_empty {
                    err.add(
                        &field.name,
                        "validation_required",
                        format!("{} is required.", field.name),
                    );
                    continue;
                }
            }

            if let Some(value) = value {
                if value.is_null() {
                    continue;
                }
                let type_ok = match field.field_type {
                    FieldType::Number   => value.is_number(),
                    FieldType::Bool     => value.is_boolean(),
                    FieldType::Text
                    | FieldType::Email
                    | FieldType::Url
                    | FieldType::Date
                    | FieldType::Select
                    | FieldType::Relation
                    | FieldType::Editor
                    | FieldType::Autodate
                    | FieldType::Password
                    | FieldType::File   => value.is_string() || value.is_array(),
                    FieldType::Json     => true,
                };
                if !type_ok {
                    err.add(
                        &field.name,
                        "validation_invalid_type",
                        format!("{} must be a {}.", field.name, field.field_type),
                    );
                }
            }
        }

        if self.record.collection.is_auth() {
            if let (Some(pw), Some(confirm)) = (&self.password, &self.password_confirm) {
                if pw != confirm {
                    err.add("passwordConfirm", "validation_mismatch", "passwords do not match.".to_string());
                }
            }
        }

        if err.is_empty() {
            Ok(())
        } else {
            Err(err)
        }
    }

    pub async fn submit(mut self) -> Result<Record, FormError> {
        self.validate()?;

        let is_new = self.record.base.is_new();

        if is_new {
            self.insert().await.map_err(|e| {
                let mut err = FormError::new();
                err.add("_", "db_error", e.to_string());
                err
            })?;
        } else {
            self.update().await.map_err(|e| {
                let mut err = FormError::new();
                err.add("_", "db_error", e.to_string());
                err
            })?;
        }

        Ok(self.record)
    }

    async fn insert(&mut self) -> Result<(), DbError> {
        let now = now_utc();
        self.record.created = now.clone();
        self.record.updated = now.clone();

        let mut columns = vec!["id", "created", "updated"];
        let mut values: Vec<Value> = vec![
            Value::String(self.record.base.id.clone()),
            Value::String(now.clone()),
            Value::String(now.clone()),
        ];

        // auth collections: hash password if provided, generate tokenKey
        if self.record.collection.is_auth() {
            use crate::tools::security::{hash_password, generate_id};

            let hashed = if let Some(ref pw) = self.password {
                hash_password(pw).map_err(|e| DbError::Migration(e.to_string()))?
            } else {
                String::new()
            };

            columns.push("email");
            values.push(
                self.data.remove("email").unwrap_or(Value::String(String::new()))
            );
            columns.push("password");
            values.push(Value::String(hashed));
            columns.push("tokenKey");
            values.push(Value::String(generate_id()));
            columns.push("verified");
            values.push(Value::Bool(false));
        }

        for (col, val) in &self.data {
            columns.push(col.as_str());
            values.push(val.clone());
        }

        let placeholders: Vec<&str> = vec!["?"; columns.len()];
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.record.table_name(),
            columns.join(", "),
            placeholders.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for val in &values {
            query = match val {
                Value::String(s) => query.bind(s.clone()),
                Value::Number(n) => query.bind(n.as_f64().unwrap_or(0.0)),
                Value::Bool(b)   => query.bind(*b),
                _                => query.bind(val.to_string()),
            };
        }

        // Route to the correct pool based on collection name.
        let pool = self.app.pool_for_collection(self.record.table_name());
        query.execute(pool).await?;

        self.record.load(
            self.data.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );

        Ok(())
    }

    async fn update(&mut self) -> Result<(), DbError> {
        let now = now_utc();
        self.record.updated = now.clone();

        if self.data.is_empty() {
            return Ok(());
        }

        let sets: Vec<String> = self.data.keys()
            .map(|k| format!("{} = ?", k))
            .collect();
        let sql = format!(
            "UPDATE {} SET {}, updated = ? WHERE id = ?",
            self.record.table_name(),
            sets.join(", ")
        );

        let mut query = sqlx::query(&sql);
        for val in self.data.values() {
            query = match val {
                Value::String(s) => query.bind(s.clone()),
                Value::Number(n) => query.bind(n.as_f64().unwrap_or(0.0)),
                Value::Bool(b)   => query.bind(*b),
                _                => query.bind(val.to_string()),
            };
        }
        query = query.bind(now).bind(self.record.base.id.clone());

        let pool = self.app.pool_for_collection(self.record.table_name());
        query.execute(pool).await?;

        self.record.load(self.data.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        Ok(())
    }
}

fn now_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (y, mo, d, h, min, s) = secs_to_datetime(secs);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}Z", y, mo, d, h, min, s)
}

fn secs_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let min = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let days = secs / 86400;
    let y = days / 365 + 1970;
    let mo = (days % 365) / 30 + 1;
    let d = (days % 365) % 30 + 1;
    (y, mo, d, h, min, s)
}
