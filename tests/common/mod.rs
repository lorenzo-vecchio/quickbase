use quickbase::db::pool::DbPools;

#[allow(dead_code)]
pub async fn setup_test_db() -> DbPools {
    DbPools::connect_in_memory().await.expect("failed to create test DB")
}