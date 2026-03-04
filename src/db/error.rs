use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("migration error: {0}")]
    Migration(String),

    #[error("record not found")]
    NotFound,

    #[error("model is not new")]
    NotNew,
}