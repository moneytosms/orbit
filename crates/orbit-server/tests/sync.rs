use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode, header},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::net::SocketAddr;
use tower::ServiceExt;

const INBOX: &str = "00000000-0000-7000-8000-000000000001";
const DEVICE: &str = "01900000-0000-7000-8000-0000000000d1";
const OTHER_DEVICE: &str = "01900000-0000-7000-8000-0000000000d2";

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

async fn sync_graph(
    (app, token): &(Router, String),
    body: Value,
    cursor: Option<&str>,
) -> (StatusCode, Value, Option<String>) {
    let mut req = Request::post("/sync")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("x-orbit-graph", "v1");
    if let Some(cursor) = cursor {
        req = req.header("x-orbit-graph-cursor", cursor);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let graph_cursor = res
        .headers()
        .get("x-orbit-graph-cursor")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        graph_cursor,
    )
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[allow(dead_code)]
enum LegacyRow {
    Area(Value),
    Project(Value),
    Section(Value),
    Task(Value),
    Label(Value),
    Settings(Value),
    Completion(Value),
    Goal(Value),
    Milestone(Value),
    Page(Value),
    Purged(Value),
    View(Value),
    Routine(Value),
    Checkin(Value),
    Roadmap(Value),
    Phase(Value),
    Placement(Value),
    Award(Value),
}

#[tokio::test]
async fn no_token_returns_a_full_snapshot_with_inbox() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (status, res) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        res["delta"],
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "color": "", "icon": "", "cover": "", "order": "", "trashed_at": null, "archived_at": null }])
    );
    assert_eq!(res["statuses"], json!({}));
    assert!(res["token"].as_u64().unwrap() > 0);
    assert!(dir.path().join("orbit.db").exists());
}

const TASK: &str = "01900000-0000-7000-8000-00000000000a";

fn cmd(uuid: &str, r#type: &str, args: Value, wall: u64) -> Value {
    cmd_on(uuid, r#type, args, wall, DEVICE)
}

fn cmd_on(uuid: &str, r#type: &str, args: Value, wall: u64, device: &str) -> Value {
    json!({ "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": device } })
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
    let done = json!([{ "kind": "task", "id": TASK, "parent_id": INBOX, "title": "Buy milk", "completed_at": 1002, "order": "", "description": "", "due": null, "deadline": null, "duration_min": null, "priority": 4, "label_ids": [], "recurrence": null, "goal_id": null, "created_at": 1000, "trashed_at": null, "pinned_on": null, "not_today_on": null, "links": [], "reminders": [], "blobs": [] }]);
    // The completion record ships after the task.
    assert_eq!(res["delta"][0], done[0]);
    assert_eq!(res["delta"][1]["kind"], "completion");
    assert_eq!(res["delta"].as_array().unwrap().len(), 2);

    // Another Device that last synced at t0 sees the same change.
    let (_, other) = sync(&app, json!({ "v": 1, "token": t0, "commands": [] })).await;
    assert_eq!(other["delta"], res["delta"]);
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
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "color": "", "icon": "", "cover": "", "order": "", "trashed_at": null, "archived_at": null }])
    );
    assert_eq!(res["token"], 1);
}

#[tokio::test]
async fn the_protocol_version_is_checked() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let v = orbit_core::PROTOCOL_V;
    let (status, _) = sync(&app, json!({ "v": v, "commands": [] })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, body) = sync(&app, json!({ "v": 0, "commands": [] })).await;
    assert_eq!(status, StatusCode::UPGRADE_REQUIRED);
    assert_eq!(body["error"], "upgrade_required");
    assert_eq!(body["client_protocol"], 0);
    assert_eq!(body["min_client_protocol"], orbit_core::MIN_CLIENT_PROTOCOL);
    assert!(
        body["upgrade"]
            .as_str()
            .unwrap()
            .ends_with("/releases/latest")
    );
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
        json!([{ "kind": "task", "id": TASK, "parent_id": INBOX, "title": "Buy milk", "completed_at": null, "order": "", "description": "", "due": null, "deadline": null, "duration_min": null, "priority": 4, "label_ids": [], "recurrence": null, "goal_id": null, "created_at": 1000, "trashed_at": null, "pinned_on": null, "not_today_on": null, "links": [], "reminders": [], "blobs": [] }])
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

const AREA: &str = "01900000-0000-7000-8000-0000000000a1";
const PROJECT: &str = "01900000-0000-7000-8000-0000000000b1";
const SUB: &str = "01900000-0000-7000-8000-00000000000b";
const SUBSUB: &str = "01900000-0000-7000-8000-00000000000c";

fn c(n: u32) -> String {
    format!("01900000-0000-7000-8000-{n:012x}")
}

#[tokio::test]
async fn a_bad_parent_is_rejected_with_a_code() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let cmds = vec![
        cmd(
            &c(0xc1),
            "area.create",
            json!({ "id": AREA, "name": "Home", "order": "a" }),
            1000,
        ),
        // A Project under the Inbox, a Project under a missing parent, a Section under a Task.
        cmd(
            &c(0xc2),
            "project.create",
            json!({ "id": PROJECT, "parent_id": INBOX, "name": "P", "order": "a" }),
            1001,
        ),
        cmd(
            &c(0xc3),
            "project.create",
            json!({ "id": PROJECT, "parent_id": SUB, "name": "P", "order": "a" }),
            1002,
        ),
        cmd(
            &c(0xc4),
            "task.create",
            json!({ "id": TASK, "title": "T", "parent_id": AREA, "order": "a" }),
            1003,
        ),
        cmd(
            &c(0xc5),
            "section.create",
            json!({ "id": SUB, "parent_id": TASK, "name": "S", "order": "a" }),
            1004,
        ),
        // The Inbox can't be moved, and an Area can't get a parent.
        cmd(
            &c(0xc6),
            "item.move",
            json!({ "id": INBOX, "parent_id": null, "order": "z" }),
            1005,
        ),
        cmd(
            &c(0xc7),
            "item.move",
            json!({ "id": AREA, "parent_id": TASK, "order": "z" }),
            1006,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let st = &res["statuses"];
    assert_eq!(st[c(0xc1)], json!({ "status": "ok" }));
    for n in [0xc2, 0xc3, 0xc5, 0xc6, 0xc7] {
        assert_eq!(
            st[c(n)],
            json!({ "status": "error", "code": "bad_parent" }),
            "{n:x}"
        );
    }
    assert_eq!(st[c(0xc4)], json!({ "status": "ok" }));
}

#[tokio::test]
async fn item_move_reparents_and_a_task_cannot_move_under_its_own_descendant() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let cmds = vec![
        cmd(
            &c(0xc1),
            "area.create",
            json!({ "id": AREA, "name": "Home", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xc2),
            "task.create",
            json!({ "id": TASK, "title": "T", "order": "a" }),
            1001,
        ),
        cmd(
            &c(0xc3),
            "task.create",
            json!({ "id": SUB, "title": "S", "parent_id": TASK, "order": "a" }),
            1002,
        ),
        cmd(
            &c(0xc4),
            "item.move",
            json!({ "id": TASK, "parent_id": AREA, "order": "b" }),
            1003,
        ),
        cmd(
            &c(0xc5),
            "item.move",
            json!({ "id": TASK, "parent_id": SUB, "order": "c" }),
            1004,
        ),
        cmd(
            &c(0xc6),
            "item.move",
            json!({ "id": TASK, "parent_id": TASK, "order": "c" }),
            1005,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let st = &res["statuses"];
    assert_eq!(st[c(0xc4)], json!({ "status": "ok" }));
    assert_eq!(
        st[c(0xc5)],
        json!({ "status": "error", "code": "bad_parent" })
    );
    assert_eq!(
        st[c(0xc6)],
        json!({ "status": "error", "code": "bad_parent" })
    );
    let moved = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == TASK)
        .unwrap();
    assert_eq!(
        (&moved["parent_id"], &moved["order"]),
        (&json!(AREA), &json!("b"))
    );
}

#[tokio::test]
async fn completing_a_parent_completes_open_descendants_and_ships_them_in_deltas() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let cmds = vec![
        cmd(
            &c(0xc1),
            "task.create",
            json!({ "id": TASK, "title": "T", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xc2),
            "task.create",
            json!({ "id": SUB, "title": "S", "parent_id": TASK, "order": "a" }),
            1001,
        ),
        cmd(
            &c(0xc3),
            "task.create",
            json!({ "id": SUBSUB, "title": "SS", "parent_id": SUB, "order": "a" }),
            1002,
        ),
        cmd(&c(0xc4), "task.complete", json!({ "id": SUBSUB }), 1500),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let token = res["token"].clone();
    // A second Device already has everything; it then gets only the cascade.
    let done = cmd(&c(0xc5), "task.complete", json!({ "id": TASK }), 5000);
    let (_, res) = sync(&app, json!({ "v": 1, "token": token, "commands": [done] })).await;
    let done_at = |id: &str| {
        res["delta"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .map(|r| r["completed_at"].clone())
    };
    assert_eq!(done_at(TASK), Some(json!(5000)));
    assert_eq!(done_at(SUB), Some(json!(5000)));
    // Already-completed descendants keep their own time and don't ship.
    assert_eq!(done_at(SUBSUB), None);
    assert_eq!(res["clock"]["wall"], 5000);
}

#[tokio::test]
async fn undo_reverses_only_exact_cascade_records_through_completed_and_reparented_tasks() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let base = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let (root, completed_mid, deep, leaf, independent, moved, recurring) = (
        TASK,
        SUB,
        SUBSUB,
        "01900000-0000-7000-8000-00000000000d",
        "01900000-0000-7000-8000-00000000000e",
        "01900000-0000-7000-8000-00000000000f",
        "01900000-0000-7000-8000-000000000010",
    );
    let setup = vec![
        cmd(
            &c(0xd1),
            "task.create",
            json!({ "id": root, "title": "root" }),
            base + 1000,
        ),
        cmd(
            &c(0xd2),
            "task.create",
            json!({ "id": completed_mid, "title": "mid", "parent_id": root }),
            base + 1001,
        ),
        cmd(
            &c(0xd5),
            "task.create",
            json!({ "id": independent, "title": "independent", "parent_id": root }),
            base + 1004,
        ),
        cmd(
            &c(0xd6),
            "task.create",
            json!({ "id": moved, "title": "moved", "parent_id": root }),
            base + 1005,
        ),
        cmd(
            &c(0xd7),
            "task.create",
            json!({ "id": recurring, "title": "recurring", "parent_id": root }),
            base + 1006,
        ),
        cmd(
            &c(0xd8),
            "task.update",
            json!({ "id": recurring, "due": { "date": "2026-11-22" }, "recurrence": { "rule": "day", "from_completion": false } }),
            base + 1100,
        ),
    ];
    let (_, setup_res) = sync(&app, json!({ "v": 1, "commands": setup })).await;
    let completed_mid_done = cmd(
        &c(0xd9),
        "task.complete",
        json!({ "id": completed_mid }),
        base + 4000,
    );
    let (_, mid_res) = sync(
        &app,
        json!({ "v": 1, "token": setup_res["token"], "commands": [completed_mid_done] }),
    )
    .await;
    let nested = [
        cmd(
            &c(0xe1),
            "task.create",
            json!({ "id": deep, "title": "deep", "parent_id": completed_mid }),
            base + 4100,
        ),
        cmd(
            &c(0xe2),
            "task.create",
            json!({ "id": leaf, "title": "leaf", "parent_id": deep }),
            base + 4101,
        ),
    ];
    let (_, nested_res) = sync(
        &app,
        json!({ "v": 1, "token": mid_res["token"], "commands": nested }),
    )
    .await;
    let mut sibling_done = cmd(
        &c(0xda),
        "task.complete",
        json!({ "id": independent }),
        base + 5000,
    );
    sibling_done["hlc"]["counter"] = json!(1);
    let mut root_done = cmd(
        &c(0xdb),
        "task.complete",
        json!({ "id": root, "today": "2026-11-22" }),
        base + 5000,
    );
    root_done["hlc"]["counter"] = json!(2);
    let (_, done_res) = sync(&app, json!({ "v": 1, "token": nested_res["token"], "commands": [sibling_done, root_done.clone()] })).await;
    let root_completion = done_res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "completion" && r["item_id"] == root)
        .unwrap()
        .clone();
    assert_eq!(root_completion["source_id"], root_done["uuid"]);
    assert_eq!(
        done_res["delta"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "completion" && r["item_id"] == deep)
            .unwrap()["source_id"],
        root_done["uuid"]
    );
    let token = done_res["token"].clone();

    let move_task = cmd(
        &c(0xdc),
        "item.move",
        json!({ "id": moved, "parent_id": INBOX, "order": "z" }),
        base + 6000,
    );
    let same_due_edit = cmd(
        &c(0xde),
        "task.update",
        json!({ "id": recurring, "due": { "date": "2026-11-23" } }),
        base + 6000,
    );
    let undo = cmd(
        &c(0xdd),
        "task.uncomplete",
        json!({
            "id": root,
            "completion_id": root_completion["id"],
            "completed_at": base + 5000,
            "completion_hlc": root_completion["completion_hlc"],
            "expected_due": null,
            "restore_due": null,
        }),
        base + 7000,
    );
    let (_, undone) = sync(
        &app,
        json!({ "v": 1, "token": token, "commands": [same_due_edit, move_task, undo] }),
    )
    .await;
    let undo_rows = undone["delta"].as_array().unwrap();
    let (_, snapshot) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let rows = snapshot["delta"].as_array().unwrap();
    let task = |id: &str| {
        rows.iter().find(|r| r["id"] == id).unwrap_or_else(|| {
            panic!(
                "missing task {id}; delta={rows:?}; statuses={}",
                undone["statuses"]
            )
        })
    };
    assert_eq!(task(root)["completed_at"], Value::Null);
    assert_eq!(task(completed_mid)["completed_at"], json!(base + 4000));
    assert_eq!(task(independent)["completed_at"], json!(base + 5000));
    assert_eq!(task(deep)["completed_at"], Value::Null);
    assert_eq!(task(leaf)["completed_at"], Value::Null);
    assert_eq!(task(moved)["completed_at"], Value::Null);
    assert_eq!(task(moved)["parent_id"], INBOX);
    assert_eq!(task(recurring)["due"]["date"], "2026-11-23");
    let expected_purges = [deep, leaf, moved, recurring].map(|id| {
        let id: orbit_core::Uuid = id.parse().unwrap();
        let source: orbit_core::Uuid = root_done["uuid"].as_str().unwrap().parse().unwrap();
        orbit_core::completion_id(id, source).to_string()
    });
    for id in expected_purges {
        assert!(
            undo_rows
                .iter()
                .any(|r| r["kind"] == "purged" && r["id"] == id),
            "missing completion tombstone {id}; got {:?}",
            undo_rows
                .iter()
                .map(|r| (r["kind"].clone(), r["id"].clone()))
                .collect::<Vec<_>>()
        );
    }
    for id in [completed_mid, independent] {
        let id: orbit_core::Uuid = id.parse().unwrap();
        let source: orbit_core::Uuid = root_done["uuid"].as_str().unwrap().parse().unwrap();
        let completion_id = orbit_core::completion_id(id, source).to_string();
        assert!(
            !undo_rows
                .iter()
                .any(|r| r["kind"] == "purged" && r["id"] == completion_id)
        );
    }
}

#[tokio::test]
async fn completing_a_recurring_task_advances_its_due_and_logs_each_time() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let setup = vec![
        cmd(
            &c(0xc1),
            "task.create",
            json!({ "id": TASK, "title": "Water", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xc2),
            "task.update",
            json!({ "id": TASK, "due": { "date": "2026-11-19", "time": null },
                    "recurrence": { "rule": "day", "from_completion": false } }),
            1001,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": setup })).await;
    let mut token = res["token"].clone();
    let mut dues = vec![];
    let mut completions = vec![];
    for (n, wall) in [(0xc3, 2000), (0xc4, 3000)] {
        let done = cmd(
            &c(n),
            "task.complete",
            json!({ "id": TASK, "today": "2026-11-19" }),
            wall,
        );
        let (_, res) = sync(&app, json!({ "v": 1, "token": token, "commands": [done] })).await;
        token = res["token"].clone();
        let rows = res["delta"].as_array().unwrap();
        let task = rows.iter().find(|r| r["id"] == TASK).unwrap();
        assert_eq!(task["completed_at"], Value::Null);
        dues.push(task["due"]["date"].clone());
        completions.extend(rows.iter().filter(|r| r["kind"] == "completion").cloned());
    }
    // `every day` moves from the due date: it advances one day per completion.
    assert_eq!(dues, vec![json!("2026-11-20"), json!("2026-11-21")]);
    assert_eq!(completions.len(), 2);
    assert_ne!(completions[0]["id"], completions[1]["id"]);
    assert!(completions.iter().all(|r| r["item_id"] == TASK));
}

#[tokio::test]
async fn completing_a_goal_or_milestone_logs_a_completion_and_sub_goals_stay_one_deep() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (goal, sub, subsub, ms) = (c(0x10), c(0x11), c(0x12), c(0x13));
    let cmds = vec![
        cmd(
            &c(0xc1),
            "area.create",
            json!({ "id": AREA, "name": "Home", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xc2),
            "goal.create",
            json!({ "id": goal, "parent_id": AREA, "name": "Get fit", "order": "a",
                    "start": "2026-01-01", "target_date": "2026-12-31" }),
            1001,
        ),
        cmd(
            &c(0xc3),
            "goal.create",
            json!({ "id": sub, "parent_id": goal, "name": "Run", "order": "a" }),
            1002,
        ),
        cmd(
            &c(0xc4),
            "goal.create",
            json!({ "id": subsub, "parent_id": sub, "name": "Too deep", "order": "a" }),
            1003,
        ),
        cmd(
            &c(0xc5),
            "milestone.create",
            json!({ "id": ms, "goal_id": goal, "title": "First 5k", "order": "a" }),
            1004,
        ),
        cmd(
            &c(0xc6),
            "milestone.update",
            json!({ "id": ms, "done": true }),
            1005,
        ),
        cmd(&c(0xc7), "goal.complete", json!({ "id": goal }), 1006),
        cmd(
            &c(0xc8),
            "goal.create",
            json!({ "id": c(0x14), "parent_id": AREA, "name": "Other", "order": "b" }),
            1007,
        ),
        // `goal` has a Sub-goal, so moving it under another Goal would nest two deep.
        cmd(
            &c(0xc9),
            "item.move",
            json!({ "id": goal, "parent_id": c(0x14), "order": "a" }),
            1008,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let st = &res["statuses"];
    for n in [0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7, 0xc8] {
        assert_eq!(st[c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    for n in [0xc4, 0xc9] {
        assert_eq!(
            st[c(n)],
            json!({ "status": "error", "code": "bad_parent" }),
            "{n:x}"
        );
    }
    let rows = res["delta"].as_array().unwrap();
    let mut items: Vec<_> = rows
        .iter()
        .filter(|r| r["kind"] == "completion")
        .map(|r| r["item_id"].clone())
        .collect();
    items.sort_by_key(|v| v.to_string());
    assert_eq!(items, vec![json!(goal), json!(ms)]);
}

#[tokio::test]
async fn settings_update_creates_the_row_then_updates_it() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let (c1, c2) = (
        "01900000-0000-7000-8000-0000000000e1",
        "01900000-0000-7000-8000-0000000000e2",
    );
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": snap["token"], "commands": [
            cmd(c1, "settings.update", json!({ "theme": "dracula", "display_name": "Orbit User", "avatar": "a".repeat(64) }), 1000),
            cmd(c2, "settings.update", json!({ "accent": "#ff0000" }), 1001),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c1], json!({ "status": "ok" }));
    assert_eq!(res["statuses"][c2], json!({ "status": "ok" }));
    let settings: Vec<_> = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "settings")
        .collect();
    assert_eq!(settings.len(), 1);
    assert_eq!(settings[0]["id"], "00000000-0000-7000-8000-000000000002");
    assert_eq!(settings[0]["theme"], "dracula");
    assert_eq!(settings[0]["accent"], "#ff0000");
    assert_eq!(settings[0]["display_name"], "Orbit User");
    assert_eq!(settings[0]["avatar"], "a".repeat(64));
    let (_, replica) = sync(
        &app,
        json!({ "v": 1, "token": snap["token"], "since": 0, "commands": [] }),
    )
    .await;
    let profile = replica["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "settings")
        .unwrap();
    assert_eq!(profile["display_name"], "Orbit User");
    assert_eq!(profile["avatar"], "a".repeat(64));
}

#[tokio::test]
async fn label_create_is_accepted_without_a_parent() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let c = "01900000-0000-7000-8000-0000000000e3";
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": snap["token"], "commands": [
            cmd(c, "label.create", json!({ "id": TASK, "name": "errand" }), 1000),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c], json!({ "status": "ok" }));
}

fn cmd_from(device: &str, uuid: &str, r#type: &str, args: Value, wall: u64) -> Value {
    json!({ "uuid": uuid, "type": r#type, "args": args,
            "hlc": { "wall": wall, "counter": 0, "device": device } })
}

#[tokio::test]
async fn two_offline_page_edits_keep_the_second_body_as_a_conflict_copy() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let page = c(0x20);
    let (dev_a, dev_b) = (
        "a0000000-0000-7000-8000-0000000000d1",
        "b0000000-0000-7000-8000-0000000000d2",
    );
    // 2026-03-04 UTC.
    let day = 1_772_582_400_000u64;
    let setup = vec![
        cmd(
            &c(0xd1),
            "area.create",
            json!({ "id": AREA, "name": "Home", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xd2),
            "page.create",
            json!({ "id": page, "parent_id": AREA, "title": "Notes", "body": "base", "order": "a" }),
            1001,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": setup })).await;
    assert_eq!(res["statuses"][c(0xd2)], json!({ "status": "ok" }));
    let edits = vec![
        cmd_from(
            dev_a,
            &c(0xd3),
            "page.edit_body",
            json!({ "id": page, "body": "from A", "base_rev": 0 }),
            day,
        ),
        cmd_from(
            dev_b,
            &c(0xd4),
            "page.edit_body",
            json!({ "id": page, "body": "from B", "base_rev": 0 }),
            day + 1,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": edits })).await;
    assert_eq!(res["statuses"][c(0xd3)], json!({ "status": "ok" }));
    assert_eq!(res["statuses"][c(0xd4)], json!({ "status": "ok" }));
    let pages: Vec<&Value> = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "page")
        .collect();
    assert_eq!(pages.len(), 2);
    let original = pages.iter().find(|p| p["id"] == json!(page)).unwrap();
    assert_eq!(original["body"], "from A");
    assert_eq!(original["rev"], 1);
    let copy = pages.iter().find(|p| p["id"] != json!(page)).unwrap();
    assert_eq!(copy["title"], "Notes (conflict, b0000000, 2026-03-04)");
    assert_eq!(copy["body"], "from B");
    assert_eq!(copy["rev"], 0);
    assert_eq!(copy["parent_id"], AREA);
    assert!(copy["order"].as_str().unwrap() > "a");

    // A replay of the losing command makes no second copy.
    let (_, again) = sync(&app, json!({ "v": 1, "commands": [edits[1].clone()] })).await;
    let n = again["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "page")
        .count();
    assert_eq!(n, 2);
}

#[tokio::test]
async fn a_page_cannot_live_under_a_page_or_the_inbox() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (page, nested, in_inbox) = (c(0x30), c(0x31), c(0x32));
    let cmds = vec![
        cmd(
            &c(0xe1),
            "area.create",
            json!({ "id": AREA, "name": "Home", "order": "a" }),
            1000,
        ),
        cmd(
            &c(0xe2),
            "page.create",
            json!({ "id": page, "parent_id": AREA, "title": "Notes" }),
            1001,
        ),
        cmd(
            &c(0xe3),
            "page.create",
            json!({ "id": nested, "parent_id": page, "title": "Nested" }),
            1002,
        ),
        cmd(
            &c(0xe4),
            "page.create",
            json!({ "id": in_inbox, "parent_id": INBOX, "title": "Loose" }),
            1003,
        ),
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let st = &res["statuses"];
    assert_eq!(st[c(0xe2)], json!({ "status": "ok" }));
    for n in [0xe3, 0xe4] {
        assert_eq!(
            st[c(n)],
            json!({ "status": "error", "code": "bad_parent" }),
            "{n:x}"
        );
    }
}

const DAY: u64 = 24 * 60 * 60 * 1000;

fn real_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Area > Project > (Section, Task), all created at `now`; returns the sync token.
async fn tree(app: &(Router, String), now: u64) -> Value {
    let (_, res) = sync(
        app,
        json!({ "v": 1, "commands": [
            cmd(&c(0xe1), "area.create", json!({ "id": AREA, "name": "Home", "order": "a" }), now),
            cmd(&c(0xe2), "project.create", json!({ "id": PROJECT, "parent_id": AREA, "name": "P", "order": "a" }), now),
            cmd(&c(0xe3), "section.create", json!({ "id": SUB, "parent_id": PROJECT, "name": "S", "order": "a" }), now),
            cmd(&c(0xe4), "task.create", json!({ "id": TASK, "title": "T", "parent_id": PROJECT, "order": "a" }), now),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"].as_object().unwrap().len(), 4);
    res["token"].clone()
}

fn row<'a>(res: &'a Value, id: &str) -> &'a Value {
    res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == id)
        .unwrap_or_else(|| panic!("no row {id} in {res}"))
}

#[tokio::test]
async fn delete_sets_trashed_at_and_restore_clears_it() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;

    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xe5), "item.delete", json!({ "id": TASK }), now + 10),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xe5)], json!({ "status": "ok" }));
    assert_eq!(row(&res, TASK)["trashed_at"], now + 10);
    let t1 = res["token"].clone();

    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t1, "commands": [
            cmd(&c(0xe6), "item.restore", json!({ "id": TASK }), now + 20),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xe6)], json!({ "status": "ok" }));
    assert_eq!(row(&res, TASK)["trashed_at"], Value::Null);
}

#[tokio::test]
async fn deleting_a_section_reparents_unseen_children_and_preserves_newer_moves() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let stale = tree(&app, now).await;
    let unseen = c(0xf2);
    let completed = c(0xf3);
    let trashed = c(0xf4);
    let moved = c(0xf5);

    // A replica with `stale` has not seen these Section children.
    let (_, created) = sync(
        &app,
        json!({ "v": 1, "token": stale, "commands": [
            cmd(&c(0xf1), "task.create", json!({ "id": unseen, "title": "Unseen", "parent_id": SUB, "order": "a" }), now + 1),
            cmd(&c(0xf6), "task.update", json!({ "id": unseen, "description": "Keep details", "priority": 2, "due": { "date": "2026-11-25" } }), now + 2),
            cmd(&c(0xf7), "task.create", json!({ "id": completed, "title": "Completed", "parent_id": SUB, "order": "b" }), now + 3),
            cmd(&c(0xf8), "task.complete", json!({ "id": completed }), now + 4),
            cmd(&c(0xf9), "task.create", json!({ "id": trashed, "title": "Trashed", "parent_id": SUB, "order": "c" }), now + 5),
            cmd(&c(0xfa), "item.delete", json!({ "id": trashed }), now + 6),
            cmd(&c(0xfb), "task.create", json!({ "id": moved, "title": "Moved elsewhere", "parent_id": SUB, "order": "d" }), now + 7),
            cmd(&c(0xfc), "item.move", json!({ "id": moved, "parent_id": PROJECT, "order": "z" }), now + 40),
        ]}),
    )
    .await;
    for id in [
        c(0xf1),
        c(0xf6),
        c(0xf7),
        c(0xf8),
        c(0xf9),
        c(0xfa),
        c(0xfb),
        c(0xfc),
    ] {
        assert_eq!(created["statuses"][id], json!({ "status": "ok" }));
    }

    let deletion = cmd_on(
        &c(0xfd),
        "item.delete",
        json!({ "id": SUB }),
        now + 20,
        OTHER_DEVICE,
    );
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": stale, "commands": [deletion.clone()] }),
    )
    .await;
    assert_eq!(res["statuses"][c(0xfd)], json!({ "status": "ok" }));
    assert_eq!(row(&res, SUB)["trashed_at"], now + 20);

    let task = row(&res, &unseen);
    assert_eq!(task["parent_id"], PROJECT);
    assert_eq!(task["title"], "Unseen");
    assert_eq!(task["description"], "Keep details");
    assert_eq!(task["priority"], 2);
    assert_eq!(task["due"]["date"], "2026-11-25");

    let done = row(&res, &completed);
    assert_eq!(done["parent_id"], PROJECT);
    assert_eq!(done["completed_at"], now + 4);
    let deleted_task = row(&res, &trashed);
    assert_eq!(deleted_task["parent_id"], PROJECT);
    assert_eq!(deleted_task["trashed_at"], now + 6);

    let moved_row = row(&res, &moved);
    assert_eq!(moved_row["parent_id"], PROJECT);
    assert_eq!(moved_row["order"], "z");
    assert!(task["order"].as_str().unwrap() > "z");

    // A retry of the same command returns its saved status without applying duplicate moves.
    let token = res["token"].clone();
    let (_, retried) = sync(
        &app,
        json!({ "v": 1, "token": token, "commands": [deletion] }),
    )
    .await;
    assert_eq!(retried["statuses"][c(0xfd)], json!({ "status": "ok" }));
    assert_eq!(retried["delta"], json!([]));
}

#[tokio::test]
async fn an_edit_to_a_trashed_task_applies_and_it_stays_trashed() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;

    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xe5), "item.delete", json!({ "id": TASK }), now + 10),
            cmd(&c(0xe6), "task.update", json!({ "id": TASK, "title": "Renamed" }), now + 20),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xe6)], json!({ "status": "ok" }));
    let task = row(&res, TASK);
    assert_eq!(task["title"], "Renamed");
    assert_eq!(task["trashed_at"], now + 10);
}

#[tokio::test]
async fn creates_under_a_trashed_parent_are_trashed_or_captured_to_the_inbox() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let section = c(0xf1);
    let task = c(0xf2);

    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0xe5), "item.delete", json!({ "id": PROJECT }), now + 10),
        cmd(&c(0xe6), "section.create", json!({ "id": section, "parent_id": PROJECT, "name": "Late", "order": "b" }), now + 5),
        cmd(&c(0xe7), "task.create", json!({ "id": task, "title": "Late", "parent_id": PROJECT, "order": "b" }), now + 5),
    ]})).await;
    assert_eq!(res["statuses"][c(0xe6)], json!({ "status": "ok" }));
    assert_eq!(res["statuses"][c(0xe7)], json!({ "status": "ok" }));
    assert_eq!(row(&res, &section)["trashed_at"], now + 10);
    assert_eq!(row(&res, &task)["parent_id"], INBOX);
    assert_eq!(row(&res, &task)["trashed_at"], Value::Null);
}

#[tokio::test]
async fn purge_leaves_tombstones_that_expire_and_an_old_token_gets_a_full_resync() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xe5), "item.delete", json!({ "id": PROJECT }), now),
        ]}),
    )
    .await;
    let t1 = res["token"].clone();

    // Under 30 days: still in Trash.
    orbit_server::purge_at(dir.path(), (now + 29 * DAY) as i64).unwrap();
    let (_, res) = sync(&app, json!({ "v": 1, "token": t1, "commands": [] })).await;
    assert_eq!(res["delta"], json!([]));

    let purged_at = now + 31 * DAY;
    orbit_server::purge_at(dir.path(), purged_at as i64).unwrap();
    let (_, res) = sync(&app, json!({ "v": 1, "token": t1, "commands": [] })).await;
    assert_eq!(res["full"], false);
    for id in [PROJECT, SUB, TASK] {
        assert_eq!(
            row(&res, id),
            &json!({ "kind": "purged", "id": id, "purged_at": purged_at })
        );
    }
    assert_eq!(res["delta"].as_array().unwrap().len(), 3);
    // The Area was never trashed.
    let (_, all) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    assert_eq!(row(&all, AREA)["kind"], "area");
    let t2 = res["token"].clone();

    // Tombstones are dropped after 90 more days; a token from before that is stale.
    orbit_server::purge_at(dir.path(), (purged_at + 91 * DAY) as i64).unwrap();
    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [] })).await;
    assert_eq!(res["full"], true);
    let ids: Vec<&str> = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&AREA) && !ids.contains(&PROJECT));
    // A token taken after the purge still works.
    let (_, res) = sync(&app, json!({ "v": 1, "token": t2, "commands": [] })).await;
    assert_eq!(res["full"], false);
}

#[tokio::test]
async fn an_archived_project_is_read_only_until_unarchived() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;

    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0xe5), "item.archive", json!({ "id": PROJECT }), now + 10),
        cmd(&c(0xe6), "task.update", json!({ "id": TASK, "title": "Nope" }), now + 20),
        cmd(&c(0xe7), "task.create", json!({ "id": c(0xf2), "title": "New", "parent_id": PROJECT }), now + 21),
    ]})).await;
    assert_eq!(res["statuses"][c(0xe5)], json!({ "status": "ok" }));
    assert_eq!(
        res["statuses"][c(0xe6)],
        json!({ "status": "error", "code": "archived" })
    );
    assert_eq!(
        res["statuses"][c(0xe7)],
        json!({ "status": "error", "code": "archived" })
    );
    assert_eq!(row(&res, TASK)["title"], "T");
    let t1 = res["token"].clone();

    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t1, "commands": [
            cmd(&c(0xe8), "item.unarchive", json!({ "id": PROJECT }), now + 30),
            cmd(&c(0xe9), "task.update", json!({ "id": TASK, "title": "Yes" }), now + 40),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xe8)], json!({ "status": "ok" }));
    assert_eq!(res["statuses"][c(0xe9)], json!({ "status": "ok" }));
    assert_eq!(row(&res, TASK)["title"], "Yes");
    assert_eq!(row(&res, PROJECT)["archived_at"], Value::Null);
}

const UPCOMING: &str = "00000000-0000-7000-8000-000000000102";
const VIEW: &str = "01900000-0000-7000-8000-0000000000f1";

#[tokio::test]
async fn a_hidden_builtin_view_syncs_and_a_builtin_cannot_be_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let t0 = snap["token"].clone();

    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xf2), "view.update", json!({ "id": UPCOMING, "hidden": true }), 1000),
            cmd(&c(0xf3), "view.delete", json!({ "id": UPCOMING }), 1001),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xf2)], json!({ "status": "ok" }));
    assert_eq!(
        res["statuses"][c(0xf3)],
        json!({ "status": "error", "code": "bad_field" })
    );
    let view = row(&res, UPCOMING);
    assert_eq!(view["kind"], "view");
    assert_eq!(view["hidden"], true);
    assert_eq!(view["trashed_at"], Value::Null);

    // A second Device gets the same row.
    let (_, other) = sync(&app, json!({ "v": 1, "token": t0, "commands": [] })).await;
    assert_eq!(row(&other, UPCOMING)["hidden"], true);
}

#[tokio::test]
async fn deleting_a_custom_view_trashes_it() {
    let now = real_now();
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": snap["token"], "commands": [
            cmd(&c(0xf4), "view.create", json!({ "id": VIEW, "name": "Mine", "kind": "task", "order": "a" }), now),
            cmd(&c(0xf5), "view.delete", json!({ "id": VIEW }), now + 1),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xf4)], json!({ "status": "ok" }));
    assert_eq!(res["statuses"][c(0xf5)], json!({ "status": "ok" }));
    assert_eq!(row(&res, VIEW)["trashed_at"], now + 1);
}

#[tokio::test]
async fn a_project_is_touched_when_a_task_under_it_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xf6), "task.create", json!({ "id": SUBSUB, "title": "U", "parent_id": PROJECT, "order": "b" }), now + 500),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xf6)], json!({ "status": "ok" }));
    assert_eq!(row(&res, PROJECT)["touched_at"], now + 500);
}

#[tokio::test]
async fn routines_converge_on_one_check_in_and_an_archived_one_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let routine = c(0xd1);
    let checkin =
        orbit_core::routine::checkin_id(routine.parse().unwrap(), "2026-03-02").to_string();
    let other = "01900000-0000-7000-8000-0000000000d2";
    let set = |uuid: &str, device: &str, state: &str, wall: u64| {
        cmd_from(
            device,
            uuid,
            "checkin.set",
            json!({ "id": checkin, "routine_id": routine, "date": "2026-03-02", "state": state }),
            wall,
        )
    };
    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0xd3), "routine.create", json!({ "id": routine, "parent_id": PROJECT, "name": "Meds",
            "schedule": { "kind": "daily" }, "start": "2026-03-01" }), now + 1),
        set(&c(0xd4), DEVICE, "done", now + 2),
        set(&c(0xd5), other, "skipped", now + 3),
        cmd(&c(0xd6), "routine.create", json!({ "id": c(0xd7), "parent_id": PROJECT, "name": "Bad",
            "schedule": { "kind": "daily" }, "start": "2026-03-01", "goal_id": PROJECT }), now + 4),
        cmd(&c(0xd8), "item.archive", json!({ "id": routine }), now + 5),
        set(&c(0xd9), DEVICE, "done", now + 6),
        cmd(&c(0xda), "checkin.clear", json!({ "id": checkin }), now + 7),
    ]})).await;
    for n in [0xd3, 0xd4, 0xd5, 0xd8] {
        assert_eq!(res["statuses"][c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        res["statuses"][c(0xd6)],
        json!({ "status": "error", "code": "bad_parent" })
    );
    for n in [0xd9, 0xda] {
        assert_eq!(
            res["statuses"][c(n)],
            json!({ "status": "error", "code": "archived" })
        );
    }
    assert_eq!(row(&res, &checkin)["state"], "skipped");
    assert_eq!(row(&res, &checkin)["cleared"], false);
    assert_eq!(row(&res, &routine)["name"], "Meds");
    assert!(row(&res, &routine)["archived_at"].is_u64());
}

#[tokio::test]
async fn item_pin_round_trips_through_the_server() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let create = cmd(
        &c(0xe1),
        "task.create",
        json!({ "id": TASK, "title": "Buy milk" }),
        1000,
    );
    let pin = cmd(
        &c(0xe2),
        "item.pin",
        json!({ "id": TASK, "date": "2026-03-10" }),
        2000,
    );
    let (_, res) = sync(&app, json!({ "v": 1, "commands": [create, pin] })).await;
    assert_eq!(res["statuses"][c(0xe2)]["status"], "ok");
    let (_, other) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let task = other["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == TASK)
        .unwrap();
    assert_eq!(task["pinned_on"], "2026-03-10");
    let hide = cmd(
        &c(0xe3),
        "item.not_today",
        json!({ "id": TASK, "date": "2026-03-10" }),
        3000,
    );
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": other["token"], "commands": [hide] }),
    )
    .await;
    let task = res["delta"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == TASK)
        .unwrap();
    assert_eq!(
        (task["pinned_on"].is_null(), task["not_today_on"].clone()),
        (true, json!("2026-03-10"))
    );
}

#[tokio::test]
async fn a_page_body_with_a_link_gets_the_link_in_its_index() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let target = "01900000-0000-7000-8000-0000000007a1";
    let page = "01900000-0000-7000-8000-0000000007a2";
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let body = format!("see [Foo](orbit:{target}) and [Bar](orbit:nope)");
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": snap["token"], "commands": [
            cmd(&c(0x7a1), "area.create", json!({ "id": AREA, "name": "Home", "order": "a" }), 1000),
            cmd(&c(0x7a2), "page.create", json!({ "id": page, "parent_id": AREA, "title": "P", "body": "", "order": "a" }), 1001),
            cmd(&c(0x7a3), "page.edit_body", json!({ "id": page, "body": body, "base_rev": 0 }), 1002),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0x7a3)], json!({ "status": "ok" }));
    assert_eq!(row(&res, page)["links"], json!([target]));
}

#[tokio::test]
async fn reminders_add_snooze_spend_and_remove_on_a_task() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let rid = c(0xf1);
    let (_, res) = sync(&app, json!({ "v": 1, "token": snap["token"], "commands": [
        cmd(&c(0xf0), "task.create", json!({ "id": TASK, "title": "Call" }), 1000),
        cmd(&c(0xf2), "reminder.add", json!({ "task_id": TASK, "reminder":
            { "id": rid, "kind": "before", "field": "due", "minutes": 30 } }), 1001),
        cmd(&c(0xf3), "reminder.snooze", json!({ "task_id": TASK, "at": "2026-03-02T09:10" }), 1002),
        cmd(&c(0xf4), "reminder.snooze", json!({ "task_id": TASK, "at": "nope" }), 1003),
        cmd(&c(0xf5), "reminder.spend", json!({ "task_id": TASK, "id": c(0xf3) }), 1004),
    ]})).await;
    for n in [0xf0, 0xf2, 0xf3, 0xf5] {
        assert_eq!(res["statuses"][c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        res["statuses"][c(0xf4)],
        json!({ "status": "error", "code": "bad_field" })
    );
    let reminders = row(&res, TASK)["reminders"].as_array().unwrap().clone();
    assert_eq!(reminders.len(), 2);
    assert_eq!(reminders[0]["minutes"], 30);
    assert_eq!(reminders[1]["at"], "2026-03-02T09:10");
    assert_eq!(reminders[1]["spent"], true);
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": res["token"], "commands": [
            cmd(&c(0xf6), "reminder.remove", json!({ "task_id": TASK, "id": rid }), 1005),
        ]}),
    )
    .await;
    assert_eq!(res["statuses"][c(0xf6)], json!({ "status": "ok" }));
    assert_eq!(row(&res, TASK)["reminders"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn placements_converge_on_one_row_and_an_archived_roadmap_is_read_only() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (roadmap, now_ph, later_ph) = (c(0xc1), c(0xc2), c(0xc3));
    let placement =
        orbit_core::roadmap::placement_id(roadmap.parse().unwrap(), TASK.parse().unwrap())
            .to_string();
    let other = "01900000-0000-7000-8000-0000000000d2";
    let place = |uuid: &str, device: &str, phase: &str, wall: u64| {
        cmd_from(
            device,
            uuid,
            "placement.set",
            json!({ "id": placement, "roadmap_id": roadmap, "item_id": TASK, "phase_id": phase }),
            wall,
        )
    };
    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0xc4), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Plan" }), now + 1),
        cmd(&c(0xc5), "phase.create", json!({ "id": now_ph, "roadmap_id": roadmap, "name": "Now" }), now + 2),
        cmd(&c(0xc6), "phase.create", json!({ "id": later_ph, "roadmap_id": roadmap, "name": "Later" }), now + 3),
        place(&c(0xc7), DEVICE, &now_ph, now + 4),
        place(&c(0xc8), other, &later_ph, now + 5),
        place(&c(0xc9), DEVICE, &c(0xee), now + 6),
        cmd(&c(0xca), "phase.complete", json!({ "id": now_ph }), now + 7),
        cmd(&c(0xcb), "phase.uncomplete", json!({ "id": now_ph }), now + 8),
        cmd(&c(0xcc), "item.archive", json!({ "id": roadmap }), now + 9),
        place(&c(0xcd), DEVICE, &now_ph, now + 10),
        cmd(&c(0xce), "phase.complete", json!({ "id": later_ph }), now + 11),
        cmd(&c(0xcf), "page.create", json!({ "id": c(0xf3), "title": "Notes", "parent_id": roadmap }), now + 12),
    ]})).await;
    for n in [0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xca, 0xcb, 0xcc] {
        assert_eq!(res["statuses"][c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        res["statuses"][c(0xc9)],
        json!({ "status": "error", "code": "bad_parent" })
    );
    for n in [0xcd, 0xce, 0xcf] {
        assert_eq!(
            res["statuses"][c(n)],
            json!({ "status": "error", "code": "archived" }),
            "{n:x}"
        );
    }
    assert_eq!(row(&res, &placement)["phase_id"], later_ph);
    assert_eq!(row(&res, &now_ph)["completed_at"], Value::Null);
    assert!(row(&res, &roadmap)["archived_at"].is_u64());
}

#[tokio::test]
async fn an_area_rename_converges_by_last_writer_wins() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let (_, snap) = sync(&app, json!({ "v": 1, "commands": [] })).await;
    let t0 = snap["token"].clone();
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xd1), "area.create", json!({ "id": AREA, "name": "Home", "order": "a" }), 1000),
            cmd(&c(0xd2), "area.update", json!({ "id": AREA, "name": "Work" }), 3000),
            cmd(&c(0xd3), "area.update", json!({ "id": AREA, "name": "Old" }), 2000),
            cmd(&c(0xd4), "area.update", json!({ "id": TASK, "name": "Nope" }), 4000),
        ]}),
    )
    .await;
    for n in [0xd1, 0xd2, 0xd3] {
        assert_eq!(res["statuses"][c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        res["statuses"][c(0xd4)],
        json!({ "status": "error", "code": "not_found" })
    );
    assert_eq!(row(&res, AREA)["name"], "Work");
}

#[tokio::test]
async fn a_phase_gate_must_be_a_milestone_and_placement_links_converge() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (roadmap, phase, goal, ms) = (c(0x61), c(0x62), c(0x63), c(0x64));
    let placement =
        orbit_core::roadmap::placement_id(roadmap.parse().unwrap(), TASK.parse().unwrap())
            .to_string();
    let (_, res) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0x65), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Plan" }), now + 1),
        cmd(&c(0x66), "phase.create", json!({ "id": phase, "roadmap_id": roadmap, "name": "Later" }), now + 2),
        cmd(&c(0x67), "goal.create", json!({ "id": goal, "parent_id": AREA, "name": "Run", "order": "a" }), now + 3),
        cmd(&c(0x68), "milestone.create", json!({ "id": ms, "goal_id": goal, "title": "First 5k", "order": "a" }), now + 4),
        cmd(&c(0x69), "phase.update", json!({ "id": phase, "gate_id": TASK }), now + 5),
        cmd(&c(0x6a), "phase.update", json!({ "id": phase, "gate_id": ms }), now + 6),
        cmd(&c(0x6b), "placement.set", json!({ "id": placement, "roadmap_id": roadmap, "item_id": TASK, "phase_id": phase, "links": [PROJECT] }), now + 7),
    ]})).await;
    for n in [0x65, 0x66, 0x67, 0x68, 0x6a, 0x6b] {
        assert_eq!(res["statuses"][c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        res["statuses"][c(0x69)],
        json!({ "status": "error", "code": "bad_parent" })
    );
    assert_eq!(row(&res, &phase)["gate_id"], ms);
    assert_eq!(row(&res, &placement)["links"], json!([PROJECT]));
}

#[tokio::test]
async fn graph_nodes_edges_sync_across_devices_with_per_field_clocks() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (roadmap, phase, task_node, decision, edge) = (c(0x91), c(0x92), c(0x93), c(0x94), c(0x95));
    let other_device = "01900000-0000-7000-8000-0000000000d2";
    let create_request = json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0x96), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Plan" }), now + 1),
        cmd(&c(0x97), "phase.create", json!({ "id": phase, "roadmap_id": roadmap, "name": "Stage" }), now + 2),
        cmd(&c(0x98), "roadmap.node.create", json!({ "id": task_node, "roadmap_id": roadmap, "kind": "task", "ref_id": TASK, "title": "", "x": 10, "y": 20, "band_id": phase }), now + 3),
        cmd(&c(0x99), "roadmap.node.create", json!({ "id": decision, "roadmap_id": roadmap, "kind": "decision", "title": "Choose", "x": 40, "y": 20 }), now + 4),
        cmd(&c(0x9a), "roadmap.edge.create", json!({ "id": edge, "roadmap_id": roadmap, "source_id": decision, "target_id": task_node, "kind": "decides", "label": "yes" }), now + 5),
    ]});
    serde_json::from_value::<orbit_core::SyncRequest>(create_request.clone()).unwrap();
    let (create_status, created, create_cursor) = sync_graph(&app, create_request, None).await;
    assert_eq!(create_status, StatusCode::OK, "{created:?}");
    assert!(created.get("statuses").is_some(), "{created:?}");
    for n in [0x96, 0x97, 0x98, 0x99, 0x9a] {
        assert_eq!(
            created["statuses"][c(n)],
            json!({ "status": "ok" }),
            "{n:x}: {created:?}"
        );
    }
    let move_newer = cmd_from(
        other_device,
        &c(0x9b),
        "roadmap.node.move",
        json!({ "id": task_node, "x": 80, "y": 90 }),
        now + 10,
    );
    let choose = cmd(
        &c(0x9c),
        "roadmap.node.decide",
        json!({ "id": decision, "edge_id": edge, "decided_at": now + 11 }),
        now + 11,
    );
    let (_, update, update_cursor) = sync_graph(
        &app,
        json!({ "v": 1, "token": created["token"], "commands": [move_newer, choose] }),
        create_cursor.as_deref(),
    )
    .await;
    assert_eq!(update["statuses"][c(0x9b)], json!({ "status": "ok" }));
    assert_eq!(update["statuses"][c(0x9c)], json!({ "status": "ok" }));
    let stale = cmd(
        &c(0x9d),
        "roadmap.node.move",
        json!({ "id": task_node, "x": 1, "y": 2 }),
        now + 9,
    );
    let stale_decision = cmd_from(
        other_device,
        &c(0xa3),
        "roadmap.node.decide",
        json!({ "id": decision, "edge_id": null, "decided_at": null }),
        now + 8,
    );
    let (_, replica, _) = sync_graph(
        &app,
        json!({ "v": 1, "token": created["token"], "commands": [stale, stale_decision] }),
        create_cursor.as_deref(),
    )
    .await;
    assert_eq!(replica["statuses"][c(0x9d)], json!({ "status": "ok" }));
    assert_eq!(replica["statuses"][c(0xa3)], json!({ "status": "ok" }));
    let node = row(&replica, &task_node);
    assert_eq!(
        (
            node["x"].as_f64(),
            node["y"].as_f64(),
            node["band_id"].as_str()
        ),
        (Some(80.0), Some(90.0), Some(phase.as_str()))
    );
    let decision_row = row(&replica, &decision);
    assert_eq!(decision_row["taken_edge_id"], edge);
    assert_eq!(decision_row["decided_at"], now + 11);

    let (_, rejected, _) = sync_graph(
        &app,
        json!({ "v": 1, "token": replica["token"], "commands": [
            cmd(&c(0x9e), "roadmap.node.decide", json!({ "id": task_node, "edge_id": edge, "decided_at": now + 12 }), now + 12),
            cmd(&c(0x9f), "roadmap.edge.create", json!({ "id": c(0xa0), "roadmap_id": roadmap, "source_id": decision, "target_id": TASK, "kind": "then" }), now + 13),
        ]}),
        update_cursor.as_deref(),
    ).await;
    assert_eq!(
        rejected["statuses"][c(0x9e)],
        json!({ "status": "error", "code": "bad_field" })
    );
    assert_eq!(
        rejected["statuses"][c(0x9f)],
        json!({ "status": "error", "code": "bad_parent" })
    );
}

#[tokio::test]
async fn graph_edge_reroute_keeps_identity_and_a_valid_decision() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (roadmap, decision, first, second, edge) = (c(0xc1), c(0xc2), c(0xc3), c(0xc4), c(0xc5));
    let (_, created, cursor) = sync_graph(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xc6), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Plan" }), now + 1),
            cmd(&c(0xc7), "roadmap.node.create", json!({ "id": decision, "roadmap_id": roadmap, "kind": "decision", "title": "Choose", "x": 0, "y": 0 }), now + 2),
            cmd(&c(0xc8), "roadmap.node.create", json!({ "id": first, "roadmap_id": roadmap, "kind": "note", "title": "First", "x": 300, "y": 0 }), now + 3),
            cmd(&c(0xc9), "roadmap.node.create", json!({ "id": second, "roadmap_id": roadmap, "kind": "note", "title": "Second", "x": 600, "y": 0 }), now + 4),
            cmd(&c(0xca), "roadmap.edge.create", json!({ "id": edge, "roadmap_id": roadmap, "source_id": decision, "target_id": first, "kind": "decides", "label": "yes" }), now + 5),
        ]}),
        None,
    )
    .await;
    assert_eq!(created["statuses"][c(0xca)], json!({ "status": "ok" }));
    let (_, chosen, chosen_cursor) = sync_graph(
        &app,
        json!({ "v": 1, "token": created["token"], "commands": [
            cmd(&c(0xcb), "roadmap.node.decide", json!({ "id": decision, "edge_id": edge, "decided_at": now + 6 }), now + 6),
        ]}),
        cursor.as_deref(),
    )
    .await;
    assert_eq!(chosen["statuses"][c(0xcb)], json!({ "status": "ok" }));
    let (_, rerouted, _) = sync_graph(
        &app,
        json!({ "v": 1, "token": chosen["token"], "commands": [
            cmd(&c(0xcc), "roadmap.edge.update", json!({ "id": edge, "target_id": second }), now + 7),
        ]}),
        chosen_cursor.as_deref(),
    )
    .await;
    assert_eq!(rerouted["statuses"][c(0xcc)], json!({ "status": "ok" }));
    let (_, snapshot, _) =
        sync_graph(&app, json!({ "v": 1, "token": 0, "commands": [] }), None).await;
    assert_eq!(row(&snapshot, &edge)["target_id"], second);
    assert_eq!(row(&snapshot, &decision)["taken_edge_id"], edge);
    assert_eq!(row(&snapshot, &decision)["decided_at"], now + 6);
    let (_, rejected, _) = sync_graph(
        &app,
        json!({ "v": 1, "token": rerouted["token"], "commands": [
            cmd(&c(0xcd), "roadmap.edge.update", json!({ "id": edge, "target_id": decision }), now + 8),
        ]}),
        None,
    )
    .await;
    assert_eq!(
        rejected["statuses"][c(0xcd)],
        json!({ "status": "error", "code": "bad_field" })
    );
}

#[tokio::test]
async fn graph_capability_gets_full_first_pull_then_incremental_graph_delta() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let (roadmap, node) = (c(0xf1), c(0xf2));
    let (_, created) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xb3), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Plan" }), now + 1),
            cmd(&c(0xb4), "roadmap.node.create", json!({ "id": node, "roadmap_id": roadmap, "kind": "text", "title": "Draft", "x": 1, "y": 2 }), now + 2),
        ]}),
    )
    .await;
    assert_eq!(created["statuses"][c(0xb3)], json!({ "status": "ok" }));
    assert_eq!(created["statuses"][c(0xb4)], json!({ "status": "ok" }));
    let (status, first, cursor) = sync_graph(
        &app,
        json!({ "v": 1, "token": created["token"], "commands": [] }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(first["full"].as_bool().unwrap());
    assert!(
        first["delta"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["id"] == node),
        "{first:?}"
    );
    let cursor = cursor.unwrap();

    let (status, moved, next_cursor) = sync_graph(
        &app,
        json!({ "v": 1, "token": created["token"], "commands": [
            cmd(&c(0xb5), "roadmap.node.move", json!({ "id": node, "x": 8, "y": 9 }), now + 3),
        ]}),
        Some(&cursor),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(moved["full"], false);
    assert_eq!(
        moved["delta"],
        json!([{ "kind": "graph_node", "id": node, "roadmap_id": roadmap, "node_kind": "text", "ref_id": null, "title": "Draft", "x": 8.0, "y": 9.0, "band_id": null, "taken_edge_id": null, "decided_at": null, "trashed_at": null }])
    );
    let cursor_seq = cursor.strip_prefix("v1:").unwrap().parse::<u64>().unwrap();
    let next_seq = next_cursor
        .unwrap()
        .strip_prefix("v1:")
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(next_seq > cursor_seq);
}

#[tokio::test]
async fn startup_migrates_stored_placements_once_and_keeps_the_legacy_rows() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    let t0 = tree(&app, now).await;
    let roadmap = c(0xa1);
    let phase = c(0xa2);
    let second_task = c(0xa3);
    let (_, made) = sync(&app, json!({ "v": 1, "token": t0, "commands": [
        cmd(&c(0xa4), "roadmap.create", json!({ "id": roadmap, "parent_id": PROJECT, "name": "Legacy" }), now + 1),
        cmd(&c(0xa5), "phase.create", json!({ "id": phase, "roadmap_id": roadmap, "name": "Week" }), now + 2),
        cmd(&c(0xa6), "task.create", json!({ "id": second_task, "parent_id": PROJECT, "title": "Second" }), now + 3),
    ]})).await;
    let rm: orbit_core::Uuid = roadmap.parse().unwrap();
    let p: orbit_core::Uuid = phase.parse().unwrap();
    let first: orbit_core::Uuid = TASK.parse().unwrap();
    let second: orbit_core::Uuid = second_task.parse().unwrap();
    let legacy_a = orbit_core::Row::Placement(orbit_core::roadmap::Placement {
        id: orbit_core::roadmap::placement_id(rm, first),
        roadmap_id: rm,
        item_id: first,
        phase_id: Some(p),
        links: vec![second],
    });
    let legacy_b = orbit_core::Row::Placement(orbit_core::roadmap::Placement {
        id: orbit_core::roadmap::placement_id(rm, second),
        roadmap_id: rm,
        item_id: second,
        phase_id: Some(p),
        links: vec![],
    });
    let db = rusqlite::Connection::open(dir.path().join("orbit.db")).unwrap();
    for legacy in [legacy_a, legacy_b] {
        db.execute("INSERT INTO rows (id, data, clocks, seq) VALUES (?1, ?2, '{}', (SELECT MAX(seq)+1 FROM rows))",
            rusqlite::params![legacy.id().to_string(), serde_json::to_string(&legacy).unwrap()]).unwrap();
    }
    drop(db);
    let (reopened, _) = orbit_server::app(dir.path()).unwrap();
    let (status, migrated, graph_cursor) = sync_graph(
        &(reopened.clone(), app.1.clone()),
        json!({ "v": 1, "token": made["token"], "commands": [] }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(migrated["full"].as_bool().unwrap());
    assert!(graph_cursor.as_deref().unwrap().starts_with("v1:"));
    let nodes: Vec<_> = migrated["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "graph_node")
        .collect();
    let edges: Vec<_> = migrated["delta"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["kind"] == "graph_edge")
        .collect();
    assert_eq!(nodes.len(), 2);
    assert_eq!(edges.len(), 1);
    assert!(nodes.iter().all(|n| n["band_id"] == phase));
    assert!(
        migrated["delta"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "placement")
    );

    // A strict legacy tagged Row enum can still parse every row from its normal v1 pull.
    let (_, legacy) = sync(
        &(reopened.clone(), app.1.clone()),
        json!({ "v": 1, "token": made["token"], "commands": [] }),
    )
    .await;
    let old_rows: Vec<LegacyRow> = serde_json::from_value(legacy["delta"].clone()).unwrap();
    assert!(
        old_rows
            .iter()
            .any(|r| matches!(r, LegacyRow::Placement(_)))
    );
    assert!(
        legacy["delta"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| { r["kind"] != "graph_node" && r["kind"] != "graph_edge" })
    );

    // Simulate expired graph tombstones disappearing from row storage. The durable migration
    // marker must still prevent rebuilding those deterministic IDs from surviving Placements.
    let db = rusqlite::Connection::open(dir.path().join("orbit.db")).unwrap();
    for id in nodes
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .chain(edges.iter().map(|e| e["id"].as_str().unwrap()))
    {
        db.execute("DELETE FROM rows WHERE id = ?1", [id]).unwrap();
    }
    drop(db);
    drop(reopened);
    let (again, _) = orbit_server::app(dir.path()).unwrap();
    let (_, after_restart, _) = sync_graph(
        &(again, app.1.clone()),
        json!({ "v": 1, "token": made["token"], "commands": [] }),
        None,
    )
    .await;
    assert!(
        !after_restart["delta"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["kind"] == "graph_node" || r["kind"] == "graph_edge")
    );
}

#[tokio::test]
async fn purge_runs_at_most_once_per_maintenance_interval() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path()).await;
    let now = real_now();
    // The first sync runs maintenance.
    let old = now - 31 * DAY;
    let t0 = tree(&app, old).await;
    // Trashed 31 days ago (all clocks are back then), so a purge pass would tombstone it; this sync is inside the interval.
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": t0, "commands": [
            cmd(&c(0xe5), "item.delete", json!({ "id": PROJECT }), old + 10),
        ]}),
    )
    .await;
    assert_eq!(row(&res, PROJECT)["trashed_at"], old + 10);

    // Make maintenance due again (as if the interval had passed).
    let conn = rusqlite::Connection::open(dir.path().join("orbit.db")).unwrap();
    conn.execute(
        "UPDATE meta SET value = 0 WHERE key = 'last_maintenance'",
        [],
    )
    .unwrap();
    let (_, res) = sync(
        &app,
        json!({ "v": 1, "token": res["token"].clone(), "commands": [] }),
    )
    .await;
    assert_eq!(row(&res, PROJECT)["kind"], "purged");
}
