//! Multi-Device sync harness: a real orbit-server on an ephemeral port and N in-memory clients.
use orbit_client::Client;
use rusqlite::Connection;
use std::net::SocketAddr;

pub const PASSWORD: &str = "a long password";

pub struct Server {
    pub url: String,
    pub data: tempfile::TempDir,
}

/// A server whose user is already set up.
pub async fn server() -> Server {
    let data = tempfile::tempdir().unwrap();
    let (app, code) = orbit_server::app(data.path()).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let body = serde_json::json!({ "code": code.unwrap(), "password": PASSWORD, "name": "Setup", "platform": "cli" });
    reqwest::Client::new()
        .post(format!("{url}/auth/setup"))
        .json(&body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    Server { url, data }
}

/// A Device with its own empty replica that has never logged in.
pub fn replica(url: &str) -> Client {
    Client::open(Connection::open_in_memory().unwrap(), url).unwrap()
}

/// A logged-in Device with its own empty replica.
pub async fn device(s: &Server) -> Client {
    let mut c = replica(&s.url);
    c.login(PASSWORD, None, "Test", "cli").await.unwrap();
    c
}
