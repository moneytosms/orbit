use std::{env, path::PathBuf};

#[tokio::main]
async fn main() {
    let data = PathBuf::from(env::var("ORBIT_DATA").unwrap_or_else(|_| "data".into()));
    let addr = env::var("ORBIT_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let app = orbit_server::app(&data).expect("open data dir");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind ORBIT_ADDR");
    eprintln!("orbit-server on {addr}, data in {}", data.display());
    axum::serve(listener, app).await.unwrap();
}
