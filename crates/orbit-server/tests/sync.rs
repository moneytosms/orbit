use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;

const INBOX: &str = "00000000-0000-7000-8000-000000000001";
const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";

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

async fn sync((app, token): &(Router, String), body: Value) -> (StatusCode, Value) {
    let req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn no_token_returns_a_full_snapshot_with_inbox() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (status, res) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        res["delta"],
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox" }])
    );
    assert_eq!(res["statuses"], json!({}));
    assert!(res["token"].as_u64().unwrap() > 0);
    assert!(dir.path().join("orbit.db").exists());
}

const TASK: &str = "01900000-0000-7000-8000-00000000000a";

fn cmd(uuid: &str, r#type: &str, args: Value, wall: u64) -> Value {
    json!({ "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": DEVICE } })
}

#[tokio::test]
async fn commands_apply_in_order_and_other_devices_get_the_delta() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let t0 = snap["token"].clone();

    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd("01900000-0000-7000-8000-0000000000c1", "task.create", json!({ "id": TASK, "title": "Buy milk" }), 1000),
        cmd("01900000-0000-7000-8000-0000000000c2", "task.complete",
            json!({ "id": "01900000-0000-7000-8000-0000000000ff" }), 1001),
        cmd("01900000-0000-7000-8000-0000000000c3", "task.complete", json!({ "id": TASK }), 1002),
    ]})).await;
    assert_eq!(
        res["statuses"],
        json!({
            "01900000-0000-7000-8000-0000000000c1": { "status": "ok" },
            "01900000-0000-7000-8000-0000000000c2": { "status": "error", "code": "not_found" },
            "01900000-0000-7000-8000-0000000000c3": { "status": "ok" },
        })
    );
    let done = json!([{ "kind": "task", "id": TASK, "area_id": INBOX, "title": "Buy milk", "completed_at": 1002 }]);
    assert_eq!(res["delta"], done);

    // Another Device that last synced at t0 sees the same change.
    let (_, other) = sync(&app, json!({ "v": 1, "token": t0, "commands": [] })).await;
    assert_eq!(other["delta"], done);
    assert_eq!(other["token"], res["token"]);

    // Nothing new since the latest token.
    let (_, idle) = sync(
        &app,
        json!({ "v": 1, "token": res["token"], "commands": [] }),
    )
    .await;
    assert_eq!(idle["delta"], json!([]));
    assert_eq!(idle["token"], res["token"]);
}

#[tokio::test]
async fn a_replayed_uuid_returns_its_stored_status_without_reapplying() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let early_complete = cmd(
        "01900000-0000-7000-8000-0000000000c1",
        "task.complete",
        json!({ "id": TASK }),
        900,
    );
    let create = cmd(
        "01900000-0000-7000-8000-0000000000c2",
        "task.create",
        json!({ "id": TASK, "title": "Buy milk" }),
        1000,
    );

    let (_, first) = sync(
        &app,
        json!({ "v": 1, "commands": [early_complete, create] }),
    )
    .await;
    // Re-sent after a lost response: same uuids, same order.
    let (_, replay) = sync(
        &app,
        json!({ "v": 1, "commands": [early_complete, create] }),
    )
    .await;
    assert_eq!(replay["statuses"], first["statuses"]);
    assert_eq!(
        replay["statuses"]["01900000-0000-7000-8000-0000000000c1"]["code"],
        "not_found"
    );
    assert_eq!(
        replay["statuses"]["01900000-0000-7000-8000-0000000000c2"]["status"],
        "ok"
    );

    let task = replay["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == TASK)
        .unwrap()
        .clone();
    assert_eq!(task["completed_at"], Value::Null);
}

#[tokio::test]
async fn more_than_100_commands_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let commands: Vec<Value> = (0..101)
        .map(|i| {
            cmd(
                &format!("01900000-0000-7000-8000-{i:012}"),
                "task.complete",
                json!({ "id": TASK }),
                1,
            )
        })
        .collect();
    let (status, _) = sync(&app, json!({ "v": 1, "commands": commands })).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn a_token_newer_than_the_data_gets_a_full_snapshot() {
    // e.g. the data dir was wiped while a client kept its token.
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, res) = sync(&app, json!({ "v": 1, "token": 40, "commands": [] })).await;
    assert_eq!(
        res["delta"],
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox" }])
    );
    assert_eq!(res["token"], 1);
}

#[tokio::test]
async fn the_protocol_version_is_checked() {
    // The 426 window itself is tested in orbit-core with a larger current version.
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let v = orbit_core::PROTOCOL_V;
    let (status, _) = sync(&app, json!({ "v": v, "commands": [] })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = sync(&app, json!({ "v": v + 1, "commands": [] })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_data_dir_from_before_field_clocks_still_opens() {
    let dir = tempfile::tempdir().unwrap();
    rusqlite::Connection::open(dir.path().join("orbit.db"))
        .unwrap()
        .execute_batch(
            "CREATE TABLE rows (id TEXT PRIMARY KEY, data TEXT NOT NULL, seq INTEGER NOT NULL);
             INSERT INTO rows VALUES ('00000000-0000-7000-8000-000000000001', '{\"kind\":\"area\",\"id\":\"00000000-0000-7000-8000-000000000001\",\"name\":\"Inbox\"}', 1);",
        )
        .unwrap();
    let app = app(dir.path()).await;
    let (status, res) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(res["delta"][0]["name"], "Inbox");
}

#[tokio::test]
async fn a_rejected_command_sends_back_its_rows_current_state() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let create = cmd(
        "01900000-0000-7000-8000-0000000000c1",
        "task.create",
        json!({ "id": TASK, "title": "Buy milk" }),
        1000,
    );
    let (_, first) = sync(&app, json!({ "v": 1, "commands": [create] })).await;

    // Up to date, then a rename the server rejects: the row comes back anyway so the client can undo its guess.
    let blank = cmd(
        "01900000-0000-7000-8000-0000000000c2",
        "task.update",
        json!({ "id": TASK, "title": " " }),
        2000,
    );
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": first["token"], "commands": [blank] }),
    )
    .await;
    assert_eq!(
        res["statuses"]["01900000-0000-7000-8000-0000000000c2"]["code"],
        "title_required"
    );
    assert_eq!(
        res["delta"],
        json!([{ "kind": "task", "id": TASK, "area_id": INBOX, "title": "Buy milk", "completed_at": null }])
    );
}

#[tokio::test]
async fn the_response_clock_is_the_newest_edit_behind_the_delta() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let create = cmd(
        "01900000-0000-7000-8000-0000000000c1",
        "task.create",
        json!({ "id": TASK, "title": "Buy milk" }),
        1000,
    );
    let rename = cmd(
        "01900000-0000-7000-8000-0000000000c2",
        "task.update",
        json!({ "id": TASK, "title": "Oat milk" }),
        7000,
    );
    sync(&app, json!({ "v": 1, "commands": [create, rename] })).await;
    let (_, other) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(
        other["clock"],
        json!({ "wall": 7000, "counter": 0, "device": DEVICE })
    );
    assert_eq!(other["delta"][1]["title"], "Oat milk");
}
