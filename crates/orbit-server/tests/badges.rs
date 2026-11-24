use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;

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

fn cmd(uuid: &str, r#type: &str, args: Value, wall: u64) -> Value {
    json!({ "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": DEVICE } })
}

fn task_id(n: u64) -> String {
    format!("01900000-0000-7000-8000-{:012x}", 0x1000 + n)
}

fn cmd_id(n: u64) -> String {
    format!("01900000-0000-7000-8000-{:012x}", 0x2000 + n)
}

/// The award rows in a sync response's delta.
fn awards(res: &Value) -> Vec<Value> {
    res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "award")
        .cloned()
        .collect()
}

#[tokio::test]
async fn completing_ten_tasks_writes_synced_awards_and_never_repeats_them() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let mut token = snap["token"].clone();

    let creates: Vec<Value> = (0..10)
        .map(|n| {
            cmd(
                &cmd_id(n),
                "task.create",
                json!({ "id": task_id(n), "title": "Task" }),
                1000 + n,
            )
        })
        .collect();
    let (status, res) = sync(&app, json!({ "v": 1, "token": token, "commands": creates })).await;
    assert_eq!(status, StatusCode::OK);
    assert!(awards(&res).is_empty());
    token = res["token"].clone();

    let mut earned = Vec::new();
    for n in 0..10 {
        let done = cmd(
            &cmd_id(100 + n),
            "task.complete",
            json!({ "id": task_id(n) }),
            2000 + n,
        );
        let (_, res) = sync(&app, json!({ "v": 1, "token": token, "commands": [done] })).await;
        token = res["token"].clone();
        earned.extend(awards(&res));
        if n < 9 {
            assert!(earned.is_empty(), "no Badge before the tenth completion");
        }
    }

    // Ten Tasks in one day, the Inbox emptied after five or more, and the tenth Task itself.
    let mut badges: Vec<&str> = earned
        .iter()
        .map(|a| a["badge"].as_str().unwrap())
        .collect();
    badges.sort();
    assert_eq!(badges, ["busy-10", "inbox-zero", "tasks-10"]);
    let tasks = earned.iter().find(|a| a["badge"] == "tasks-10").unwrap();
    assert_eq!(tasks["key"], "tasks-10");
    assert_eq!(tasks["points"], 50);
    assert!(tasks["earned_at"].as_u64().unwrap() > 0);

    // Completing more Tasks the same day never writes those Badges again.
    let more = cmd(
        &cmd_id(200),
        "task.create",
        json!({ "id": task_id(50), "title": "Extra" }),
        3000,
    );
    let done = cmd(
        &cmd_id(201),
        "task.complete",
        json!({ "id": task_id(50) }),
        3001,
    );
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": token, "commands": [more, done] }),
    )
    .await;
    assert!(awards(&res).is_empty(), "{:?}", awards(&res));

    // The full snapshot holds the three Award rows.
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(awards(&snap).len(), 3);
}

#[tokio::test]
async fn ops_that_cannot_affect_badges_skip_the_check_but_task_ops_still_award() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let creates: Vec<Value> = (0..10)
        .map(|n| {
            let id = task_id(n);
            [
                cmd(
                    &cmd_id(n),
                    "task.create",
                    json!({ "id": id, "title": "T" }),
                    1000 + n,
                ),
                cmd(
                    &cmd_id(100 + n),
                    "task.complete",
                    json!({ "id": id }),
                    2000 + n,
                ),
            ]
        })
        .flatten()
        .collect();
    let (_, res) = sync(&app, json!({ "v": 1, "commands": creates })).await;
    assert!(!awards(&res).is_empty());

    // Drop the Award rows, so any badge check that runs would write them again.
    let conn = rusqlite::Connection::open(dir.path().join("orbit.db")).unwrap();
    conn.execute(
        "DELETE FROM rows WHERE json_extract(data, '$.kind') = 'award'",
        [],
    )
    .unwrap();

    let label = cmd(
        &cmd_id(300),
        "label.create",
        json!({ "id": task_id(60), "name": "L" }),
        4000,
    );
    let (_, res) = sync(&app, json!({ "v": 1, "commands": [label] })).await;
    assert!(awards(&res).iter().all(|a| a["earned_at"] != 4000));
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert!(awards(&snap).is_empty(), "label.create skipped the check");

    let task = cmd(
        &cmd_id(301),
        "task.create",
        json!({ "id": task_id(61), "title": "N" }),
        4001,
    );
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [task] })).await;
    assert!(
        !awards(&snap).is_empty(),
        "task.create still runs the check"
    );
}
