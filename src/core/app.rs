use crate::db::{DbError, DbPools};
use crate::db::migrations::{MigrationsList, MigrationsRunner};
use crate::models::collection::Collection;

pub struct App {
    data_dir: String,
    pools: Option<DbPools>,
    system_migrations: MigrationsList,
}

impl App {
    pub fn new(data_dir: impl Into<String>) -> Self {
        let mut app = Self {
            data_dir: data_dir.into(),
            pools: None,
            system_migrations: MigrationsList::new(),
        };
        crate::migrations::register_system_migrations(&mut app.system_migrations);
        app
    }

    pub async fn bootstrap(&mut self) -> Result<(), DbError> {
        let pools = DbPools::connect(&self.data_dir).await?;
        let runner = MigrationsRunner::new(&pools, &self.system_migrations);
        runner.up().await?;
        self.pools = Some(pools);
        Ok(())
    }

    pub fn pools(&self) -> &DbPools {
        self.pools
            .as_ref()
            .expect("App not bootstrapped — call bootstrap() first")
    }

    pub async fn find_collection_by_name(&self, name: &str) -> Result<Collection, DbError> {
        sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections WHERE name = ? LIMIT 1"
        )
            .bind(name)
            .fetch_optional(&self.pools().data)
            .await?
            .ok_or(DbError::NotFound)
    }

    pub async fn list_collections(&self) -> Result<Vec<Collection>, DbError> {
        let collections = sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections ORDER BY name ASC"
        )
            .fetch_all(&self.pools().data)
            .await?;
        Ok(collections)
    }

    pub async fn find_collection_by_id(&self, id: &str) -> Result<Collection, DbError> {
        sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections WHERE id = ? LIMIT 1"
        )
            .bind(id)
            .fetch_optional(&self.pools().data)
            .await?
            .ok_or(DbError::NotFound)
    }

    pub async fn save_collection(&self, collection: &Collection) -> Result<(), DbError> {
        sqlx::query(
            "INSERT OR REPLACE INTO _collections (id, name, type, schema, indexes, options)
         VALUES (?, ?, ?, ?, ?, ?)"
        )
            .bind(&collection.base.id)
            .bind(&collection.name)
            .bind(collection.collection_type.to_string())
            .bind(serde_json::to_string(&*collection.schema).unwrap_or_default())
            .bind(serde_json::to_string(&*collection.indexes).unwrap_or_default())
            .bind(serde_json::to_string(&*collection.options).unwrap_or_default())
            .execute(&self.pools().data)
            .await?;
        Ok(())
    }
}