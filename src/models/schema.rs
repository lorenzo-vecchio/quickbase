use serde::{Deserialize, Serialize};

/// The supported field types, mirroring PocketBase's field types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Number,
    Bool,
    Date,
    Json,
    Email,
    Url,
    Select,
    Relation,
    File,
    Editor,
    Autodate,
    Password,
}

impl std::fmt::Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text     => write!(f, "text"),
            Self::Number   => write!(f, "number"),
            Self::Bool     => write!(f, "bool"),
            Self::Date     => write!(f, "date"),
            Self::Json     => write!(f, "json"),
            Self::Email    => write!(f, "email"),
            Self::Url      => write!(f, "url"),
            Self::Select   => write!(f, "select"),
            Self::Relation => write!(f, "relation"),
            Self::File     => write!(f, "file"),
            Self::Editor   => write!(f, "editor"),
            Self::Autodate => write!(f, "autodate"),
            Self::Password => write!(f, "password"),
        }
    }
}

impl std::str::FromStr for FieldType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text"     => Ok(Self::Text),
            "number"   => Ok(Self::Number),
            "bool"     => Ok(Self::Bool),
            "date"     => Ok(Self::Date),
            "json"     => Ok(Self::Json),
            "email"    => Ok(Self::Email),
            "url"      => Ok(Self::Url),
            "select"   => Ok(Self::Select),
            "relation" => Ok(Self::Relation),
            "file"     => Ok(Self::File),
            "editor"   => Ok(Self::Editor),
            "autodate" => Ok(Self::Autodate),
            "password" => Ok(Self::Password),
            _          => Err(()),
        }
    }
}

/// A single field definition in a collection's schema.
/// Mirrors PocketBase's SchemaField struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaField {
    /// Unique identifier for this field.
    pub id: String,

    /// The field name, used as the SQLite column name.
    pub name: String,

    /// The field type — determines SQLite column type and validation rules.
    #[serde(rename = "type")]
    pub field_type: FieldType,

    /// Whether a non-empty value is required.
    #[serde(default)]
    pub required: bool,

    /// Whether this is a primary key field (system).
    #[serde(default, rename = "primaryKey")]
    pub primary_key: bool,

    /// Whether the field is hidden in the API.
    #[serde(default)]
    pub hidden: bool,

    /// Type-specific options, e.g. min/max for text, choices for select.
    #[serde(default)]
    pub options: serde_json::Value,
}

impl SchemaField {
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            id: generate_field_id(),
            name: name.into(),
            field_type,
            required: false,
            primary_key: false,
            hidden: false,
            options: serde_json::json!({}),
        }
    }

    /// Returns the SQLite column type for this field.
    /// Mirrors PocketBase's field-to-column type mapping.
    pub fn sqlite_type(&self) -> &'static str {
        match self.field_type {
            FieldType::Number   => "REAL",
            FieldType::Bool     => "INTEGER",
            _                   => "TEXT",
        }
    }
}

fn generate_field_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("f{:08x}", nanos)
}
