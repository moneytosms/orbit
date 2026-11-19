mod common;
use common::{PASSWORD, device, replica, server};
use orbit_core::{ErrorCode, Op};

const HOUR: u64 = 3_600_000;

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
