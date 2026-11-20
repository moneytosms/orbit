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
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "order": "", "trashed_at": null, "archived_at": null }])
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
        json!([{ "kind": "area", "id": INBOX, "name": "Inbox", "order": "", "trashed_at": null, "archived_at": null }])
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
