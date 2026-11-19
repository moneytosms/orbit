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
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "order": "" }])
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
    let done = json!([{ "kind": "task", "id": TASK, "parent_id": INBOX, "title": "Buy milk", "completed_at": 1002, "order": "", "description": "", "due": null, "deadline": null, "duration_min": null, "priority": 4, "label_ids": [], "recurrence": null, "goal_id": null }]);
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
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "order": "" }])
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
        json!([{ "kind": "task", "id": TASK, "parent_id": INBOX, "title": "Buy milk", "completed_at": null, "order": "", "description": "", "due": null, "deadline": null, "duration_min": null, "priority": 4, "label_ids": [], "recurrence": null, "goal_id": null }])
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
    ];
    let (_, res) = sync(&app, json!({ "v": 1, "commands": cmds })).await;
    let st = &res["statuses"];
    for n in [0xc1, 0xc2, 0xc3, 0xc5, 0xc6, 0xc7] {
        assert_eq!(st[c(n)], json!({ "status": "ok" }), "{n:x}");
    }
    assert_eq!(
        st[c(0xc4)],
        json!({ "status": "error", "code": "bad_parent" })
    );
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
            cmd(c1, "settings.update", json!({ "theme": "dracula" }), 1000),
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
