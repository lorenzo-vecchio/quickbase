use quickbase::db::pool::DbPools;
use quickbase::core::app::App;
use std::sync::Arc;
use tempfile::TempDir;

#[allow(dead_code)]
pub async fn setup_test_db() -> DbPools {
    DbPools::connect_in_memory().await.expect("failed to create test DB")
}

#[allow(dead_code)]
pub async fn setup_test_app() -> (Arc<App>, TempDir) {
    let dir = TempDir::new().expect("failed to create temp dir");
    let mut app = App::new(dir.path().to_str().unwrap());
    app.bootstrap().await.expect("failed to bootstrap app");
    (Arc::new(app), dir)
}