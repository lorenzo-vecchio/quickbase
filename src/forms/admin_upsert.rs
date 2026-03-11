use crate::core::app::App;
use crate::models::admin::Admin;
use crate::db::model::BaseModel;
use crate::db::DbError;
use crate::forms::errors::FormError;
use crate::tools::security::{hash_password, generate_id};

pub struct AdminUpsert<'a> {
    app: &'a App,
    admin: Admin,
    password: Option<String>,
}

impl<'a> AdminUpsert<'a> {
    pub fn new_create(app: &'a App, email: String, password: String) -> Self {
        Self {
            app,
            admin: Admin {
                base: BaseModel::new(generate_id()),
                email,
                password: String::new(),
                token_key: generate_id(),
                avatar: 0,
                created: String::new(),
                updated: String::new(),
            },
            password: Some(password),
        }
    }

    pub fn new_update(app: &'a App, admin: Admin, password: Option<String>) -> Self {
        Self { app, admin, password }
    }

    pub fn validate(&self) -> Result<(), FormError> {
        let mut err = FormError::new();

        if self.admin.email.is_empty() {
            err.add("email", "validation_required", "email is required.".to_string());
        } else if !self.admin.email.contains('@') {
            err.add("email", "validation_invalid", "email is invalid.".to_string());
        }

        if self.admin.base.is_new() && self.password.is_none() {
            err.add("password", "validation_required", "password is required.".to_string());
        }

        if let Some(ref pw) = self.password {
            if pw.len() < 8 {
                err.add("password", "validation_too_short", "password must be at least 8 characters.".to_string());
            }
        }

        if err.is_empty() { Ok(()) } else { Err(err) }
    }

    pub async fn submit(mut self) -> Result<Admin, FormError> {
        self.validate()?;

        if self.admin.base.is_new() {
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

        Ok(self.admin)
    }

    async fn insert(&mut self) -> Result<(), DbError> {
        let hashed = hash_password(self.password.as_deref().unwrap_or(""))
            .map_err(|e| DbError::Migration(e.to_string()))?;

        let now = now_utc();
        self.admin.created = now.clone();
        self.admin.updated = now.clone();

        sqlx::query(
            "INSERT INTO _admins (id, email, password, tokenKey, avatar, created, updated)
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
            .bind(&self.admin.base.id)
            .bind(&self.admin.email)
            .bind(&hashed)
            .bind(&self.admin.token_key)
            .bind(self.admin.avatar)
            .bind(&self.admin.created)
            .bind(&self.admin.updated)
            .execute(&self.app.pools().data)
            .await?;

        self.admin.password = hashed;
        Ok(())
    }

    async fn update(&mut self) -> Result<(), DbError> {
        let now = now_utc();
        self.admin.updated = now.clone();

        if let Some(ref pw) = self.password {
            let hashed = hash_password(pw)
                .map_err(|e| DbError::Migration(e.to_string()))?;
            sqlx::query(
                "UPDATE _admins SET email = ?, password = ?, avatar = ?, updated = ? WHERE id = ?"
            )
                .bind(&self.admin.email)
                .bind(&hashed)
                .bind(self.admin.avatar)
                .bind(&now)
                .bind(&self.admin.base.id)
                .execute(&self.app.pools().data)
                .await?;
            self.admin.password = hashed;
        } else {
            sqlx::query(
                "UPDATE _admins SET email = ?, avatar = ?, updated = ? WHERE id = ?"
            )
                .bind(&self.admin.email)
                .bind(self.admin.avatar)
                .bind(&now)
                .bind(&self.admin.base.id)
                .execute(&self.app.pools().data)
                .await?;
        }

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