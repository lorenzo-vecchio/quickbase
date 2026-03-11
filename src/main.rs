use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let data_dir = "./qb_data";
    let addr: SocketAddr = "0.0.0.0:8090".parse().unwrap();

    if let Err(e) = quickbase::cmd::serve::run(data_dir, addr).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}