use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;
use tower::ServiceExt;

const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";
const PAGE: &str = "01900000-0000-7000-8000-00000000000a";
const AREA: &str = "01900000-0000-7000-8000-0000000000a1";
const DAY: i64 = 24 * 60 * 60 * 1000;

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

fn hash(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn put(app: &Router, token: Option<&str>, hash: &str, body: Vec<u8>) -> StatusCode {
    let mut req = Request::put(format!("/blobs/{hash}"));
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    res.status()
}

async fn get(app: &Router, token: Option<&str>, hash: &str) -> (StatusCode, Vec<u8>) {
    let mut req = Request::get(format!("/blobs/{hash}"));
    if let Some(t) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, bytes.to_vec())
}

async fn sync(app: &Router, token: &str, commands: Value) -> Value {
    let req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(
            json!({ "v": 1, "commands": commands }).to_string(),
        ))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn cmd(uuid: &str, r#type: &str, args: Value, wall: u64) -> Value {
    json!({ "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": DEVICE } })
}

fn area() -> Value {
    cmd(
        "01900000-0000-7000-8000-0000000000c0",
        "area.create",
        json!({ "id": AREA, "name": "Home", "order": "a" }),
        999,
    )
}

fn page(body: &str) -> Value {
    cmd(
        "01900000-0000-7000-8000-0000000000c1",
        "page.create",
        json!({ "id": PAGE, "parent_id": AREA, "title": "P", "body": body }),
        1000,
    )
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn trash_page(wall: i64) -> Value {
    cmd(
        "01900000-0000-7000-8000-0000000000c2",
        "item.delete",
        json!({ "id": PAGE }),
        wall as u64,
    )
}

#[tokio::test]
async fn put_and_get_need_a_device() {
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path()).await;
    let data = b"hello".to_vec();
    let h = hash(&data);
    assert_eq!(
        put(&app, None, &h, data.clone()).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        put(&app, Some(&token), &h, data.clone()).await,
        StatusCode::CREATED
    );
    assert_eq!(get(&app, None, &h).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(get(&app, Some(&token), &h).await, (StatusCode::OK, data));
    assert!(
        dir.path()
            .join("blobs")
            .join(&h[..2])
            .join(&h[2..4])
            .join(&h)
            .is_file()
    );
    assert_eq!(
        get(&app, Some(&token), &hash(b"nope")).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn a_body_that_does_not_match_its_hash_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path()).await;
    let h = hash(b"one");
    assert_eq!(
        put(&app, Some(&token), &h, b"two".to_vec()).await,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(get(&app, Some(&token), &h).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_blob_over_the_limit_is_413() {
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path()).await;
    let big = vec![7u8; 25 * 1024 * 1024 + 1];
    assert_eq!(
        put(&app, Some(&token), &hash(&big), big).await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn a_command_with_an_unknown_blob_is_rejected_until_it_is_uploaded() {
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path()).await;
    sync(&app, &token, json!([area()])).await;
    let data = b"pixels".to_vec();
    let h = hash(&data);
    let body = format!("![a.png](blob:{h})");
    let c = page(&body);
    let uuid = c["uuid"].as_str().unwrap().to_string();
    let res = sync(&app, &token, json!([c.clone()])).await;
    assert_eq!(
        res["statuses"][&uuid],
        json!({ "status": "error", "code": "missing_blob" })
    );
    assert_eq!(put(&app, Some(&token), &h, data).await, StatusCode::CREATED);
    // The same command goes through now: the rejection wasn't remembered.
    let res = sync(&app, &token, json!([c])).await;
    assert_eq!(res["statuses"][&uuid], json!({ "status": "ok" }));
    let row = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == PAGE)
        .unwrap();
    assert_eq!(row["blobs"], json!([h]));
}

#[tokio::test]
async fn unreferenced_blobs_go_after_the_grace_and_trash_counts_as_referenced() {
    let dir = tempfile::tempdir().unwrap();
    let (app, token) = app(dir.path()).await;
    let data = b"keep me".to_vec();
    let h = hash(&data);
    let file = dir
        .path()
        .join("blobs")
        .join(&h[..2])
        .join(&h[2..4])
        .join(&h);
    put(&app, Some(&token), &h, data).await;
    let t0 = now();

    // Unreferenced: kept through the grace, then deleted.
    orbit_server::purge_at(dir.path(), t0).unwrap();
    orbit_server::purge_at(dir.path(), t0 + 6 * DAY).unwrap();
    assert!(file.is_file());
    orbit_server::purge_at(dir.path(), t0 + 8 * DAY).unwrap();
    assert!(!file.exists());

    // Referenced, even from the Trash: kept however long.
    sync(&app, &token, json!([area()])).await;
    let data = b"keep me too".to_vec();
    let h = hash(&data);
    let file = dir
        .path()
        .join("blobs")
        .join(&h[..2])
        .join(&h[2..4])
        .join(&h);
    put(&app, Some(&token), &h, data).await;
    let res = sync(
        &app,
        &token,
        json!([page(&format!("[f](blob:{h})")), trash_page(t0)]),
    )
    .await;
    assert_eq!(res["statuses"].as_object().unwrap().len(), 2);
    orbit_server::purge_at(dir.path(), t0).unwrap();
    orbit_server::purge_at(dir.path(), t0 + 20 * DAY).unwrap();
    assert!(file.is_file());
    // Once the Page is purged (trashed over 30 days) the blob starts its grace.
    orbit_server::purge_at(dir.path(), t0 + 31 * DAY).unwrap();
    assert!(file.is_file());
    orbit_server::purge_at(dir.path(), t0 + 40 * DAY).unwrap();
    assert!(!file.exists());
}
