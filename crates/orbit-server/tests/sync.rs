use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use tower::ServiceExt;

const INBOX: &str = "00000000-0000-7000-8000-000000000001";
const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";

async fn sync(app: &Router, body: Value) -> (StatusCode, Value) {
    let req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
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
    let app = orbit_server::app(dir.path()).unwrap();
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
    let app = orbit_server::app(dir.path()).unwrap();
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
    let app = orbit_server::app(dir.path()).unwrap();
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
    let app = orbit_server::app(dir.path()).unwrap();
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
    let app = orbit_server::app(dir.path()).unwrap();
    let (_, res) = sync(&app, json!({ "v": 1, "token": 40, "commands": [] })).await;
    assert_eq!(
        res["delta"],
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox" }])
    );
    assert_eq!(res["token"], 1);
}
