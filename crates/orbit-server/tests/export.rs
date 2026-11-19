use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::{io::Read, net::SocketAddr};
use tower::ServiceExt;

const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";
const TASK: &str = "01900000-0000-7000-8000-00000000000a";

/// A server on `dir` with its user set up, and a Device token for it.
async fn app(dir: &std::path::Path) -> (Router, String) {
    let (app, code) = orbit_server::app(dir).unwrap();
    let body = json!({ "code": code.unwrap(), "password": "a long password", "name": "Test", "platform": "cli" });
    let mut req = Request::post("/auth/setup")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 1))));
    let res = app.clone().oneshot(req).await.unwrap();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let res: Value = serde_json::from_slice(&bytes).unwrap();
    (app, res["token"].as_str().unwrap().into())
}

async fn get(
    (app, token): &(Router, String),
    path: &str,
    auth: bool,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::get(path);
    if auth {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, headers, bytes.to_vec())
}

async fn sync((app, token): &(Router, String), body: Value) {
    let req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn export_is_a_zip_with_every_item() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    sync(
        &app,
        json!({ "v": 1, "commands": [{
            "uuid": "01900000-0000-7000-8000-0000000000c1",
            "type": "task.create",
            "args": { "id": TASK, "title": "Buy milk" },
            "hlc": { "wall": 1000, "counter": 0, "device": DEVICE },
        }] }),
    )
    .await;

    let (status, headers, body) = get(&app, "/export", true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_TYPE], "application/zip");
    assert_eq!(
        headers[header::CONTENT_DISPOSITION],
        "attachment; filename=\"orbit-export.zip\""
    );

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let mut file = archive.by_name("items.json").unwrap();
    let mut text = String::new();
    file.read_to_string(&mut text).unwrap();
    let items: Value = serde_json::from_str(&text).unwrap();
    let items = items.as_array().unwrap();
    assert!(
        items
            .iter()
            .any(|item| item["id"] == TASK && item["title"] == "Buy milk")
    );
}

#[tokio::test]
async fn disk_reports_free_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (status, _, body) = get(&app, "/disk", true).await;
    assert_eq!(status, StatusCode::OK);
    let res: Value = serde_json::from_slice(&body).unwrap();
    assert!(res["free_bytes"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn export_and_disk_need_a_device() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (status, _, _) = get(&app, "/export", false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _, _) = get(&app, "/disk", false).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
