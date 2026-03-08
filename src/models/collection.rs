use serde::{Deserialize, Serialize};
use crate::db::model::BaseModel;

/// The three collection types PocketBase supports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionType {
    Base,
    Auth,
    View,
}

impl Default for CollectionType {
    fn default() -> Self {
        Self::Base
    }
}

impl std::fmt::Display for CollectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base => write!(f, "base"),
            Self::Auth => write!(f, "auth"),
            Self::View => write!(f, "view"),
        }
    }
}

/// Mirrors PocketBase's core.Collection model.
/// Stored in the `_collections` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    #[serde(flatten)]
    pub base: BaseModel,

    pub name: String,

    #[serde(rename = "type")]
    pub collection_type: CollectionType,

    /// JSON array of field definitions e.g. `[{"id":"...","name":"title","type":"text"}]`
    pub schema: serde_json::Value,

    /// JSON array of CREATE INDEX statements
    pub indexes: serde_json::Value,

    // API access rules — None means the rule is not set (locked down)
    pub list_rule: Option<String>,
    pub view_rule: Option<String>,
    pub create_rule: Option<String>,
    pub update_rule: Option<String>,
    pub delete_rule: Option<String>,

    /// JSON blob of type-specific options (e.g. auth settings)
    pub options: serde_json::Value,

    pub created: String,
    pub updated: String,
}

impl Collection {
    /// Create a new base collection. Mirrors PocketBase's NewBaseCollection().
    pub fn new_base(name: impl Into<String>) -> Self {
        Self::new(name, CollectionType::Base)
    }

    /// Create a new auth collection. Mirrors PocketBase's NewAuthCollection().
    pub fn new_auth(name: impl Into<String>) -> Self {
        Self::new(name, CollectionType::Auth)
    }

    /// Create a new view collection. Mirrors PocketBase's NewViewCollection().
    pub fn new_view(name: impl Into<String>) -> Self {
        Self::new(name, CollectionType::View)
    }

    fn new(name: impl Into<String>, collection_type: CollectionType) -> Self {
        Self {
            base: BaseModel::new(crate::models::collection::generate_id()),
            name: name.into(),
            collection_type,
            schema: serde_json::json!([]),
            indexes: serde_json::json!([]),
            list_rule: None,
            view_rule: None,
            create_rule: None,
            update_rule: None,
            delete_rule: None,
            options: serde_json::json!({}),
            created: String::new(),
            updated: String::new(),
        }
    }

    pub fn id(&self) -> &str {
        self.base.id()
    }

    pub fn is_base(&self) -> bool {
        self.collection_type == CollectionType::Base
    }

    pub fn is_auth(&self) -> bool {
        self.collection_type == CollectionType::Auth
    }

    pub fn is_view(&self) -> bool {
        self.collection_type == CollectionType::View
    }

    pub fn table_name() -> &'static str {
        "_collections"
    }
}

impl Default for Collection {
    fn default() -> Self {
        Self::new_base("unknown")
    }
}

/// Generates a random ID in PocketBase's format: 'r' + 14 hex chars.
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Simple deterministic-enough ID for now; we'll replace with proper random later
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("r{:014x}", nanos)
}