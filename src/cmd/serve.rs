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
    let app = Arc::new(app);

    let router = apis::router(Arc::clone(&app));
    let listener = TcpListener::bind(addr).await?;

    println!("Listening on http://{}", addr);
    axum::serve(listener, router).await?;

    Ok(())
}