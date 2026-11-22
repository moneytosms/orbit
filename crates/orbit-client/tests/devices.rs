mod common;
use axum::{
    Json, Router,
    http::{StatusCode, header::HeaderValue},
    response::IntoResponse,
    routing::post,
};
use common::{PASSWORD, device, replica, server};
use orbit_core::{
    Due, ErrorCode, MIN_CLIENT_PROTOCOL, Op, PROTOCOL_V, Status, SyncRequest, SyncResponse,
    UpgradeRequired, Uuid, completion_id,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

const HOUR: u64 = 3_600_000;

#[test]
fn native_undo_does_not_restore_due_after_a_newer_same_value_edit() {
    let mut c = replica("http://127.0.0.1:1");
    let id = c.create_task("Recurring task", 1000).unwrap();
    let prior_due = Due {
        date: "2026-11-22".into(),
        time: None,
    };
    c.update_task(id, serde_json::json!({ "due": prior_due }), 1100)
        .unwrap();
    let complete = c
        .enqueue(Op::TaskComplete { id, today: None }, 1200)
        .unwrap();
    let completed = c.task(id).unwrap().unwrap();
    let expected_due = completed.due.clone();
    let next_due = Due {
        date: "2026-11-23".into(),
        time: None,
    };
    c.update_task(id, serde_json::json!({ "due": next_due }), 1300)
        .unwrap();
    let undo = c
        .enqueue(
            Op::TaskUncomplete {
                id,
                completion_id: Some(completion_id(id, complete.uuid)),
                completed_at: completed.completed_at,
                completion_hlc: Some(complete.hlc),
                expected_due: Some(expected_due),
                restore_due: Some(Some(prior_due)),
            },
            1400,
        )
        .unwrap();

    assert!(matches!(
        undo.op,
        Op::TaskUncomplete {
            restore_due: None,
            ..
        }
    ));
    assert_eq!(c.task(id).unwrap().unwrap().due.unwrap().date, "2026-11-23");
}

#[test]
fn native_prediction_reverses_only_exact_cascade_completions() {
    let mut c = replica("http://127.0.0.1:1");
    let root = c.create_task("Root", 1000).unwrap();
    let parse = |s: &str| s.parse::<Uuid>().unwrap();
    let (mid, deep, leaf, independent, moved, recurring) = (
        parse("01900000-0000-7000-8000-000000000101"),
        parse("01900000-0000-7000-8000-000000000102"),
        parse("01900000-0000-7000-8000-000000000103"),
        parse("01900000-0000-7000-8000-000000000104"),
        parse("01900000-0000-7000-8000-000000000105"),
        parse("01900000-0000-7000-8000-000000000106"),
    );
    for (id, title, parent_id, now_ms) in [
        (mid, "Mid", root, 1100),
        (independent, "Independent", root, 1400),
        (moved, "Moved", root, 1500),
        (recurring, "Recurring", root, 1600),
    ] {
        c.enqueue(
            Op::TaskCreate {
                id,
                title: title.into(),
                parent_id: Some(parent_id),
                order: "a".into(),
            },
            now_ms,
        )
        .unwrap();
    }
    c.update_task(
        recurring,
        serde_json::json!({
            "due": { "date": "2026-11-22" },
            "recurrence": { "rule": "day", "from_completion": false }
        }),
        1700,
    )
    .unwrap();
    c.complete_task(mid, 1800).unwrap();
    for (id, title, parent_id, now_ms) in [(deep, "Deep", mid, 1850), (leaf, "Leaf", deep, 1860)] {
        c.enqueue(
            Op::TaskCreate {
                id,
                title: title.into(),
                parent_id: Some(parent_id),
                order: "a".into(),
            },
            now_ms,
        )
        .unwrap();
    }
    c.complete_task(independent, 2000).unwrap();
    let root_done = c
        .enqueue(
            Op::TaskComplete {
                id: root,
                today: Some("2026-11-22".into()),
            },
            2000,
        )
        .unwrap();
    assert_eq!(c.task(deep).unwrap().unwrap().completed_at, Some(2000));
    assert_eq!(c.task(leaf).unwrap().unwrap().completed_at, Some(2000));
    assert_eq!(
        c.task(recurring).unwrap().unwrap().due.unwrap().date,
        "2026-11-23"
    );
    c.enqueue(
        Op::ItemMove {
            id: moved,
            parent_id: None,
            order: "z".into(),
        },
        3000,
    )
    .unwrap();
    c.update_task(
        recurring,
        serde_json::json!({ "due": { "date": "2026-11-23" } }),
        3000,
    )
    .unwrap();
    let root_task = c.task(root).unwrap().unwrap();
    c.enqueue(
        Op::TaskUncomplete {
            id: root,
            completion_id: Some(completion_id(root, root_done.uuid)),
            completed_at: root_task.completed_at,
            completion_hlc: Some(root_done.hlc),
            expected_due: Some(root_task.due.clone()),
            restore_due: Some(None),
        },
        4000,
    )
    .unwrap();

    assert_eq!(c.task(root).unwrap().unwrap().completed_at, None);
    assert_eq!(c.task(mid).unwrap().unwrap().completed_at, Some(1800));
    assert_eq!(
        c.task(independent).unwrap().unwrap().completed_at,
        Some(2000)
    );
    assert_eq!(c.task(deep).unwrap().unwrap().completed_at, None);
    assert_eq!(c.task(leaf).unwrap().unwrap().completed_at, None);
    assert_eq!(c.task(moved).unwrap().unwrap().completed_at, None);
    assert_eq!(
        c.task(recurring).unwrap().unwrap().due.unwrap().date,
        "2026-11-23"
    );
}

#[tokio::test]
async fn graph_outbox_waits_for_capability_and_survives_a_plain_v1_server() {
    let requests = Arc::new(Mutex::new(Vec::<Vec<bool>>::new()));
    let graph_enabled = Arc::new(AtomicBool::new(false));
    let app = {
        let requests = requests.clone();
        let graph_enabled = graph_enabled.clone();
        Router::new().route(
            "/sync",
            post(move |Json(req): Json<SyncRequest>| {
                let requests = requests.clone();
                let graph_enabled = graph_enabled.clone();
                async move {
                    let graph_flags = req
                        .commands
                        .iter()
                        .map(|cmd| {
                            matches!(
                                cmd.op,
                                Op::RoadmapNodeCreate { .. }
                                    | Op::RoadmapNodeMove { .. }
                                    | Op::RoadmapEdgeCreate { .. }
                            )
                        })
                        .collect();
                    requests.lock().unwrap().push(graph_flags);
                    let statuses = req
                        .commands
                        .into_iter()
                        .map(|cmd| (cmd.uuid, Status::Ok))
                        .collect();
                    let mut res = (
                        StatusCode::OK,
                        Json(SyncResponse {
                            statuses,
                            delta: vec![],
                            token: 0,
                            clock: None,
                            full: false,
                        }),
                    )
                        .into_response();
                    if graph_enabled.load(Ordering::SeqCst) {
                        res.headers_mut()
                            .insert("x-orbit-graph-cursor", HeaderValue::from_static("v1:0"));
                    }
                    res
                }
            }),
        )
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let mut client = replica(&url);
    let task = client.create_task("Legacy compatible", 1000).unwrap();
    client
        .enqueue(
            Op::RoadmapNodeCreate {
                id: "01900000-0000-7000-8000-0000000000e1".parse().unwrap(),
                roadmap_id: "01900000-0000-7000-8000-0000000000e2".parse().unwrap(),
                kind: orbit_core::roadmap::NodeKind::Task,
                ref_id: Some(task),
                title: String::new(),
                x: 0.0,
                y: 0.0,
                band_id: None,
            },
            1001,
        )
        .unwrap();

    client.sync(2000).await.unwrap();
    assert_eq!(*requests.lock().unwrap(), vec![vec![false]]);

    graph_enabled.store(true, Ordering::SeqCst);
    client.sync(3000).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[1], Vec::<bool>::new());
    assert_eq!(requests[2], vec![true]);
}

#[tokio::test]
async fn offline_edits_to_different_fields_both_survive() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    let id = a.create_task("Buy milk", 1000).unwrap();
    a.sync(1000).await.unwrap();
    b.sync(1000).await.unwrap();

    // Both offline: A renames, B completes.
    a.rename_task(id, "Buy oat milk", 2000).unwrap();
    b.complete_task(id, 2100).unwrap();
    a.sync(3000).await.unwrap();
    b.sync(3000).await.unwrap();
    a.sync(3000).await.unwrap();

    for d in [&a, &b] {
        let t = d.task(id).unwrap().unwrap();
        assert_eq!(
            (t.title.as_str(), t.completed_at),
            ("Buy oat milk", Some(2100))
        );
    }
}

#[tokio::test]
async fn offline_edits_to_priority_and_due_both_survive() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    let id = a.create_task("Buy milk", 1000).unwrap();
    a.sync(1000).await.unwrap();
    b.sync(1000).await.unwrap();

    a.update_task(id, serde_json::json!({ "priority": 1 }), 2000)
        .unwrap();
    b.update_task(
        id,
        serde_json::json!({ "due": { "date": "2026-03-04" } }),
        2100,
    )
    .unwrap();
    a.sync(3000).await.unwrap();
    b.sync(3000).await.unwrap();
    a.sync(3000).await.unwrap();

    for d in [&a, &b] {
        let t = d.task(id).unwrap().unwrap();
        assert_eq!(t.priority, 1);
        assert_eq!(t.due.unwrap().date, "2026-03-04");
    }
}

#[tokio::test]
async fn a_late_offline_edit_never_overwrites_a_newer_one() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    let id = a.create_task("Buy milk", 1000).unwrap();
    a.sync(1000).await.unwrap();
    b.sync(1000).await.unwrap();

    a.rename_task(id, "Older, from A offline", 2000).unwrap();
    b.rename_task(id, "Newer, from B", 3000).unwrap();
    b.sync(3000).await.unwrap();
    a.sync(4000).await.unwrap(); // A's older edit arrives last
    b.sync(4000).await.unwrap();

    for d in [&a, &b] {
        assert_eq!(d.task(id).unwrap().unwrap().title, "Newer, from B");
    }
}

#[tokio::test]
async fn a_device_with_a_slow_clock_still_wins_with_an_edit_made_after_it_synced() {
    let s = server().await;
    let (mut a, mut slow) = (device(&s).await, device(&s).await);
    let now = 10 * HOUR;
    let id = a.create_task("Buy milk", now).unwrap();
    a.rename_task(id, "From A", now + 1000).unwrap();
    a.sync(now + 1000).await.unwrap();

    // The slow Device's wall clock is an hour behind, but it edits after seeing A's rename.
    slow.sync(now - HOUR).await.unwrap();
    slow.rename_task(id, "From slow, later", now - HOUR + 2000)
        .unwrap();
    slow.sync(now - HOUR + 2000).await.unwrap();
    a.sync(now + 3000).await.unwrap();

    for d in [&a, &slow] {
        assert_eq!(d.task(id).unwrap().unwrap().title, "From slow, later");
    }
}

#[tokio::test]
async fn a_rejected_command_is_dropped_reported_and_its_row_restored() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    // Both Devices create the same id offline; B gets there first.
    let id = orbit_core::new_id(1000, &[9; 10]);
    b.enqueue(
        Op::TaskCreate {
            id,
            title: "From B".into(),
            parent_id: None,
            order: String::new(),
        },
        1000,
    )
    .unwrap();
    a.enqueue(
        Op::TaskCreate {
            id,
            title: "From A".into(),
            parent_id: None,
            order: String::new(),
        },
        1000,
    )
    .unwrap();
    b.sync(2000).await.unwrap();

    let rejected = a.sync(2000).await.unwrap();
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].code, ErrorCode::Exists);
    assert_eq!(a.outbox_len().unwrap(), 0);
    assert_eq!(a.task(id).unwrap().unwrap().title, "From B");
}

#[tokio::test]
async fn a_network_error_keeps_the_outbox_for_the_next_sync() {
    let s = server().await;
    let mut offline = replica("http://127.0.0.1:1");
    offline.create_task("Buy milk", 1000).unwrap();
    assert!(offline.sync(1000).await.is_err());
    assert_eq!(offline.outbox_len().unwrap(), 1);

    let mut online = device(&s).await;
    online.create_task("Buy bread", 1000).unwrap();
    online.sync(1000).await.unwrap();
    assert_eq!(online.outbox_len().unwrap(), 0);
}

#[tokio::test]
async fn a_protocol_upgrade_error_is_typed_and_keeps_the_outbox() {
    let app = axum::Router::new().route(
        "/sync",
        post(|| async {
            (
                StatusCode::UPGRADE_REQUIRED,
                Json(UpgradeRequired {
                    error: "upgrade_required".into(),
                    client_protocol: PROTOCOL_V,
                    min_client_protocol: MIN_CLIENT_PROTOCOL + 1,
                    upgrade: "https://example.test/releases/latest".into(),
                }),
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let mut client = replica(&url);
    let task = client.create_task("Queued change", 1000).unwrap();
    let err = client.sync(1000).await.unwrap_err();

    let upgrade = err.downcast_ref::<UpgradeRequired>().unwrap();
    assert_eq!(upgrade.error, "upgrade_required");
    assert_eq!(upgrade.min_client_protocol, MIN_CLIENT_PROTOCOL + 1);
    assert_eq!(upgrade.upgrade, "https://example.test/releases/latest");
    assert_eq!(client.outbox_len().unwrap(), 1);
    assert_eq!(client.task(task).unwrap().unwrap().title, "Queued change");
}

#[tokio::test]
async fn a_bodyless_protocol_upgrade_keeps_the_http_error_and_outbox() {
    let app = axum::Router::new().route("/sync", post(|| async { StatusCode::UPGRADE_REQUIRED }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let mut client = replica(&url);
    let task = client.create_task("Queued change", 1000).unwrap();
    let err = client.sync(1000).await.unwrap_err();

    let http_error = err.downcast_ref::<reqwest::Error>().unwrap();
    assert_eq!(http_error.status(), Some(StatusCode::UPGRADE_REQUIRED));
    assert_eq!(client.outbox_len().unwrap(), 1);
    assert_eq!(client.task(task).unwrap().unwrap().title, "Queued change");
}

#[tokio::test]
async fn server_version_is_typed_and_does_not_require_login() {
    let s = server().await;
    let client = replica(&s.url);
    let version = client.server_version().await.unwrap();
    assert_eq!(version.server, env!("CARGO_PKG_VERSION"));
    assert_eq!(version.protocol, PROTOCOL_V);
    assert_eq!(version.min_client_protocol, MIN_CLIENT_PROTOCOL);
}

#[tokio::test]
async fn login_makes_the_device_id_the_hlc_device() {
    let s = server().await;
    let mut a = replica(&s.url);
    let id = a.login(PASSWORD, None, "Laptop", "cli").await.unwrap();
    let cmd = a
        .enqueue(
            Op::TaskCreate {
                id: orbit_core::new_id(1000, &[1; 10]),
                title: "Buy milk".into(),
                parent_id: None,
                order: String::new(),
            },
            1000,
        )
        .unwrap();
    assert_eq!(cmd.hlc.device, id);
    let err = a
        .login("wrong password", None, "Laptop", "cli")
        .await
        .unwrap_err();
    assert_eq!(err.to_string(), "credentials");
}

#[tokio::test]
async fn a_signed_out_device_keeps_its_outbox_and_replays_it_after_login() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    orbit_server::reset_password(s.data.path(), "a new long password").unwrap();
    let id = b.create_task("Written while signed out", 1000).unwrap();

    let err = b.sync(1000).await.unwrap_err();
    assert!(orbit_client::is_signed_out(&*err), "{err}");
    assert_eq!(b.outbox_len().unwrap(), 1);

    b.login("a new long password", None, "Test", "cli")
        .await
        .unwrap();
    b.sync(2000).await.unwrap();
    assert_eq!(b.outbox_len().unwrap(), 0);
    a.login("a new long password", None, "Test", "cli")
        .await
        .unwrap();
    a.sync(2000).await.unwrap();
    assert_eq!(
        a.task(id).unwrap().unwrap().title,
        "Written while signed out"
    );
}

#[tokio::test]
async fn two_devices_reordering_the_same_task_converge_on_the_later_move() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    let area = a.create_area("Home", "a", 1000).unwrap();
    let id = a.create_task("Buy milk", 1000).unwrap();
    a.sync(1000).await.unwrap();
    b.sync(1000).await.unwrap();

    a.move_item(id, Some(area), "b", 2000).unwrap();
    b.move_item(id, Some(area), "c", 3000).unwrap();
    b.sync(3000).await.unwrap();
    a.sync(4000).await.unwrap();
    b.sync(4000).await.unwrap();

    for d in [&a, &b] {
        let t = d.task(id).unwrap().unwrap();
        assert_eq!((t.parent_id, t.order.as_str()), (area, "c"));
    }
}

#[tokio::test]
async fn offline_creates_under_a_project_trashed_meanwhile_follow_it_into_trash() {
    let s = server().await;
    let (mut a, mut b) = (device(&s).await, device(&s).await);
    // Real time: the server purges anything trashed 30 days before its own clock.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let area = a.create_area("Home", "a", now).unwrap();
    let project = orbit_core::new_id(now, &[1; 10]);
    a.enqueue(
        Op::ProjectCreate {
            color: String::new(),
            id: project,
            parent_id: area,
            name: "P".into(),
            order: "a".into(),
        },
        now,
    )
    .unwrap();
    a.sync(now).await.unwrap();
    b.sync(now).await.unwrap();

    // A trashes the project while B, offline, adds a task and a section to it.
    a.enqueue(Op::ItemDelete { id: project }, now + 1000)
        .unwrap();
    let task = orbit_core::new_id(now, &[2; 10]);
    let section = orbit_core::new_id(now, &[3; 10]);
    b.enqueue(
        Op::TaskCreate {
            id: task,
            title: "Late".into(),
            parent_id: Some(project),
            order: "a".into(),
        },
        now + 500,
    )
    .unwrap();
    b.enqueue(
        Op::SectionCreate {
            id: section,
            parent_id: project,
            name: "Late".into(),
            order: "a".into(),
        },
        now + 600,
    )
    .unwrap();
    a.sync(now + 2000).await.unwrap();
    assert!(b.sync(now + 2000).await.unwrap().is_empty());
    b.sync(now + 3000).await.unwrap();

    {
        let d = &b;
        assert_eq!(d.task(task).unwrap().unwrap().parent_id, orbit_core::INBOX);
        let Some(orbit_core::Row::Section(sec)) = d.get_row(section).unwrap() else {
            panic!("no section")
        };
        assert_eq!(sec.trashed_at, Some(now + 1000));
    }
}
