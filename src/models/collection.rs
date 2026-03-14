use serde::{Deserialize, Serialize};
use crate::db::model::BaseModel;
use crate::db::Json;
use crate::models::schema::SchemaField;

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
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Collection {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub base: BaseModel,

    pub name: String,

    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub collection_type: CollectionType,

    /// JSON array of field definitions.
    /// DB column is named `schema`; JSON key is `fields` (PocketBase v5+).
    #[serde(rename = "fields")]
    pub schema: Json<Vec<SchemaField>>,

    /// JSON array of CREATE INDEX statements
    pub indexes: Json<serde_json::Value>,

    // API access rules — None means the rule is not set (locked down)
    // DB columns are snake_case; JSON uses PocketBase's camelCase keys.
    #[serde(rename = "listRule")]
    pub list_rule: Option<String>,
    #[serde(rename = "viewRule")]
    pub view_rule: Option<String>,
    #[serde(rename = "createRule")]
    pub create_rule: Option<String>,
    #[serde(rename = "updateRule")]
    pub update_rule: Option<String>,
    #[serde(rename = "deleteRule")]
    pub delete_rule: Option<String>,

    /// JSON blob of type-specific options (e.g. auth settings)
    pub options: Json<serde_json::Value>,

    pub created: String,
    pub updated: String,

    /// Whether this is a system-managed collection (name starts with `_`).
    /// Not stored in the DB — computed after load.
    #[sqlx(skip)]
    #[serde(default)]
    pub system: bool,
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
        let name = name.into();
        let system = name.starts_with('_');
        Self {
            base: BaseModel::new(crate::models::collection::generate_id()),
            name,
            collection_type,
            schema: Json(vec![]),
            indexes: Json(serde_json::json!([])),
            list_rule: None,
            view_rule: None,
            create_rule: None,
            update_rule: None,
            delete_rule: None,
            options: Json(serde_json::json!({})),
            created: String::new(),
            updated: String::new(),
            system,
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

    pub fn fields(&self) -> &Vec<SchemaField> {
        &self.schema
    }

    /// Add a field to the schema.
    pub fn add_field(&mut self, field: SchemaField) {
        self.schema.0.push(field);
    }

    /// Find a field by name.
    pub fn field_by_name(&self, name: &str) -> Option<&SchemaField> {
        self.schema.0.iter().find(|f| f.name == name)
    }

    /// Compute and set the `system` flag from the collection name.
    pub fn apply_system_flag(&mut self) {
        self.system = self.name.starts_with('_');
    }
}

impl Default for Collection {
    fn default() -> Self {
        Self::new_base("unknown")
    }
}

impl sqlx::Type<sqlx::Sqlite> for CollectionType {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for CollectionType {
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        match raw.as_str() {
            "auth" => Ok(Self::Auth),
            "view" => Ok(Self::View),
            _ => Ok(Self::Base),
        }
    }
}

impl std::str::FromStr for CollectionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auth" => Ok(Self::Auth),
            "view" => Ok(Self::View),
            "base" => Ok(Self::Base),
            _ => Err(()),
        }
    }
}

/// Generates a random ID in PocketBase's format: 'r' + 14 hex chars.
fn generate_id() -> String {
    crate::tools::security::generate_id()
}
