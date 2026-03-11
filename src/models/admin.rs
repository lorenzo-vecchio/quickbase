use serde::{Deserialize, Serialize};
use crate::db::model::BaseModel;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Admin {
    #[sqlx(flatten)]
    #[serde(flatten)]
    pub base: BaseModel,

    pub email: String,

    #[serde(skip_serializing)]  // never expose password hash in API responses
    pub password: String,

    #[serde(skip_serializing)]  // never expose tokenKey
    #[sqlx(rename = "tokenKey")]
    pub token_key: String,

    pub avatar: i64,

    pub created: String,
    pub updated: String,
}

impl Admin {
    pub fn id(&self) -> &str {
        self.base.id()
    }
}