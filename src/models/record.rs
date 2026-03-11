use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::db::model::BaseModel;
use crate::models::collection::Collection;

/// Mirrors PocketBase's core.Record.
/// Wraps a Collection and holds field values in a dynamic map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    #[serde(flatten)]
    pub base: BaseModel,

    /// The collection this record belongs to.
    /// Not stored in the record row itself — joined at query time.
    #[serde(skip)]
    pub collection: Collection,

    /// The record's field values, keyed by field name.
    /// e.g. {"title": "Hello", "views": 42, "published": true}
    #[serde(flatten)]
    pub data: HashMap<String, Value>,

    /// Expanded relation fields, keyed by field name.
    /// Populated only when ?expand=fieldName is requested.
    /// e.g. {"author": {"id": "...", "name": "..."}}
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub expand: HashMap<String, Value>,

    pub created: String,
    pub updated: String,
}

impl Record {
    /// Mirrors PocketBase's NewRecord(collection).
    pub fn new(collection: Collection) -> Self {
        Self {
            base: BaseModel::new(generate_id()),
            collection,
            data: HashMap::new(),
            expand: HashMap::new(),
            created: String::new(),
            updated: String::new(),
        }
    }

    pub fn id(&self) -> &str {
        self.base.id()
    }

    /// Returns the SQLite table name for this record's collection.
    /// Mirrors PocketBase's record.TableName() which returns the collection name.
    pub fn table_name(&self) -> &str {
        &self.collection.name
    }

    /// Get a field value by name.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Set a field value by name.
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.data.insert(key.into(), value);
    }

    /// Get a field as a string, returning empty string if missing or wrong type.
    /// Mirrors PocketBase's record.GetString(key).
    pub fn get_string(&self, key: &str) -> &str {
        self.data
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
    }

    /// Get a field as a bool, returning false if missing or wrong type.
    /// Mirrors PocketBase's record.GetBool(key).
    pub fn get_bool(&self, key: &str) -> bool {
        self.data
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// Get a field as an f64, returning 0.0 if missing or wrong type.
    /// Mirrors PocketBase's record.GetFloat(key).
    pub fn get_float(&self, key: &str) -> f64 {
        self.data
            .get(key)
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    /// Get a field as an i64, returning 0 if missing or wrong type.
    /// Mirrors PocketBase's record.GetInt(key).
    pub fn get_int(&self, key: &str) -> i64 {
        self.data
            .get(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
    }

    /// Load a map of values into the record's data fields.
    /// Mirrors PocketBase's record.Load(data).
    pub fn load(&mut self, data: HashMap<String, Value>) {
        if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
            self.base.id = id.to_string();
            self.base.mark_as_not_new();
        }
        self.data.extend(data);
    }
}

fn generate_id() -> String {
    crate::tools::security::generate_id()
}