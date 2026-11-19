//! Multi-Device sync harness: a real orbit-server on an ephemeral port and N in-memory clients.
use orbit_client::Client;
use rusqlite::Connection;

pub struct Server {
    pub url: String,
    _data: tempfile::TempDir,
}

pub async fn server() -> Server {
    let data = tempfile::tempdir().unwrap();
    let app = orbit_server::app(data.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    Server { url, _data: data }
}

/// A Device with its own empty replica.
pub fn device(url: &str) -> Client {
    Client::open(Connection::open_in_memory().unwrap(), url).unwrap()
}
