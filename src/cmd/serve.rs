use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use crate::core::App;
use crate::apis;

pub async fn run(data_dir: &str, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(data_dir)?;

    println!("Bootstrapping app at {}", data_dir);
    let mut app = App::new(data_dir);
    app.bootstrap().await?;

    // check if any superusers exist — if not, generate installer link
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _superusers")
        .fetch_one(&app.pools().data)
        .await?;

    if count.0 == 0 {
        let temp_id = crate::tools::security::generate_id();
        let temp_password = crate::tools::security::generate_id();
        let hashed = crate::tools::security::hash_password(&temp_password)
            .map_err(|e| format!("hash error: {}", e))?;
        let token_key = crate::tools::security::generate_id();

        sqlx::query(
            "INSERT INTO _superusers (id, email, password, tokenKey, verified)
             VALUES (?, ?, ?, ?, 1)"
        )
            .bind(&temp_id)
            .bind("_system_@quickbase.local")
            .bind(&hashed)
            .bind(&token_key)
            .execute(&app.pools().data)
            .await?;

        let token = crate::tools::security::generate_installer_token(&temp_id, "_superusers")
            .map_err(|e| format!("token error: {}", e))?;

        println!("────────────────────────────────────────────────────────");
        println!(" No superusers found.");
        println!(" Open the URL below to set up your first superuser:");
        println!();
        println!("   http://{}/_/#/pbinstal/{}", addr, token);
        println!();
        println!(" The link is valid for 30 minutes.");
        println!("────────────────────────────────────────────────────────");
    }

    let app = Arc::new(app);
    let router = apis::router(Arc::clone(&app));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on http://{}", addr);
    axum::serve(listener, router).await?;

    Ok(())
}