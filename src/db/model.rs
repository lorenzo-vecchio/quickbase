use serde::{Deserialize, Serialize};

/// Mirrors PocketBase's core.Model interface.
/// Every DB-backed struct implements this.
pub trait Model: Send + Sync {
    fn table_name() -> &'static str where Self: Sized;
    fn pk(&self) -> &str;
    fn is_new(&self) -> bool;
    fn mark_as_new(&mut self);
    fn mark_as_not_new(&mut self);
    fn last_saved_pk(&self) -> &str;
    fn post_scan(&mut self) {} // hook called after DB scan, default no-op
}

/// Mirrors PocketBase's core.BaseModel.
/// Embed this in every model struct.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BaseModel {
    pub id: String,
    #[sqlx(skip)]
    is_new: bool,
    #[sqlx(skip)]
    last_saved_pk: String,
}

impl BaseModel {
    pub fn new(id: String) -> Self {
        Self {
            id,
            is_new: true,
            last_saved_pk: String::new(),
        }
    }

    pub fn id(&self) -> &str { &self.id }
    pub fn is_new(&self) -> bool { self.is_new }
    pub fn mark_as_new(&mut self) { self.is_new = true; }
    pub fn mark_as_not_new(&mut self) {
        self.last_saved_pk = self.id.clone();
        self.is_new = false;
    }
    pub fn last_saved_pk(&self) -> &str { &self.last_saved_pk }
}