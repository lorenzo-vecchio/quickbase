use sqlx::{Decode, Type};
use serde::{Deserialize, Serialize};

/// A wrapper that teaches sqlx how to read a JSON-encoded text column
/// directly into any type T that implements serde::DeserializeOwned.
///
/// Usage: instead of `pub schema: serde_json::Value`,
///        write    `pub schema: Json<serde_json::Value>`
///
/// sqlx will fetch the column as a raw String, then call
/// serde_json::from_str() to produce the T.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// --- sqlx integration ---

// Tell sqlx what SQL type this maps to (TEXT)
impl<T> Type<sqlx::Sqlite> for Json<T> {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as Type<sqlx::Sqlite>>::type_info()
    }
}

// Tell sqlx how to decode a raw SQLite value into Json<T>
impl<'r, T> Decode<'r, sqlx::Sqlite> for Json<T>
where
    T: serde::de::DeserializeOwned,
{
    fn decode(
        value: sqlx::sqlite::SqliteValueRef<'r>,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let raw = <String as Decode<sqlx::Sqlite>>::decode(value)?;
        let parsed = serde_json::from_str(&raw)?;
        Ok(Json(parsed))
    }
}