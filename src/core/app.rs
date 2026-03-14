use sqlx::SqlitePool;
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

    /// Returns the correct connection pool for `collection_name`.
    ///
    /// - Collections whose names start with `_` (e.g. `_superusers`) are system
    ///   collections and their rows live in `system.db`.
    /// - All user-created collections live in `data.db`.
    pub fn pool_for_collection<'a>(&'a self, collection_name: &str) -> &'a SqlitePool {
        self.pools().pool_for_table(collection_name)
    }

    // -------------------------------------------------------------------------
    // Collection metadata helpers (always query the system DB)
    // -------------------------------------------------------------------------

    pub async fn find_collection_by_name(&self, name: &str) -> Result<Collection, DbError> {
        let mut c = sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections WHERE name = ? LIMIT 1"
        )
        .bind(name)
        .fetch_optional(&self.pools().system)
        .await?
        .ok_or(DbError::NotFound)?;
        c.apply_system_flag();
        Ok(c)
    }

    /// Lists only user-created collections (excludes system collections whose
    /// names start with `_`). This mirrors PocketBase's `/api/collections` behaviour.
    pub async fn list_collections(&self) -> Result<Vec<Collection>, DbError> {
        let mut collections = sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections WHERE name NOT LIKE '\\_%' ESCAPE '\\' ORDER BY name ASC"
        )
        .fetch_all(&self.pools().system)
        .await?;
        for c in &mut collections { c.apply_system_flag(); }
        Ok(collections)
    }

    /// Lists all collections including system ones (names starting with `_`).
    pub async fn list_all_collections(&self) -> Result<Vec<Collection>, DbError> {
        let mut collections = sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections ORDER BY name ASC"
        )
        .fetch_all(&self.pools().system)
        .await?;
        for c in &mut collections { c.apply_system_flag(); }
        Ok(collections)
    }

    /// Looks up a collection by name first, then by ID. Mirrors PocketBase's
    /// `:collectionIdOrName` path parameter behaviour.
    pub async fn find_collection_by_name_or_id(&self, name_or_id: &str) -> Result<Collection, DbError> {
        if let Ok(c) = self.find_collection_by_name(name_or_id).await {
            return Ok(c);
        }
        self.find_collection_by_id(name_or_id).await
    }

    pub async fn find_collection_by_id(&self, id: &str) -> Result<Collection, DbError> {
        let mut c = sqlx::query_as::<_, Collection>(
            "SELECT * FROM _collections WHERE id = ? LIMIT 1"
        )
        .bind(id)
        .fetch_optional(&self.pools().system)
        .await?
        .ok_or(DbError::NotFound)?;
        c.apply_system_flag();
        Ok(c)
    }

    pub async fn save_collection(&self, collection: &Collection) -> Result<(), DbError> {
        // Use INSERT ... ON CONFLICT upsert so that:
        // - on INSERT: created/updated are set by their DB DEFAULT
        // - on UPDATE: created is preserved; updated is refreshed to NOW
        sqlx::query(
            "INSERT INTO _collections
                (id, name, type, schema, indexes, options,
                 list_rule, view_rule, create_rule, update_rule, delete_rule)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                name        = excluded.name,
                type        = excluded.type,
                schema      = excluded.schema,
                indexes     = excluded.indexes,
                options     = excluded.options,
                list_rule   = excluded.list_rule,
                view_rule   = excluded.view_rule,
                create_rule = excluded.create_rule,
                update_rule = excluded.update_rule,
                delete_rule = excluded.delete_rule,
                updated     = strftime('%Y-%m-%d %H:%M:%fZ')"
        )
        .bind(&collection.base.id)
        .bind(&collection.name)
        .bind(collection.collection_type.to_string())
        .bind(serde_json::to_string(&*collection.schema).unwrap_or_default())
        .bind(serde_json::to_string(&*collection.indexes).unwrap_or_default())
        .bind(serde_json::to_string(&*collection.options).unwrap_or_default())
        .bind(&collection.list_rule)
        .bind(&collection.view_rule)
        .bind(&collection.create_rule)
        .bind(&collection.update_rule)
        .bind(&collection.delete_rule)
        .execute(&self.pools().system)
        .await?;
        Ok(())
    }

    pub async fn delete_collection(&self, collection: &Collection) -> Result<(), DbError> {
        // Drop the actual data table from the appropriate pool.
        let pool = self.pool_for_collection(&collection.name);
        let sql = format!("DROP TABLE IF EXISTS {}", collection.name);
        sqlx::query(&sql)
            .execute(pool)
            .await?;

        // Remove the metadata row from the system DB.
        sqlx::query("DELETE FROM _collections WHERE id = ?")
            .bind(&collection.base.id)
            .execute(&self.pools().system)
            .await?;

        Ok(())
    }
}
