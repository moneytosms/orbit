mod common;
use common::{device, server};
use orbit_core::{ErrorCode, Op};

const HOUR: u64 = 3_600_000;

#[tokio::test]
async fn offline_edits_to_different_fields_both_survive() {
    let s = server().await;
    let (mut a, mut b) = (device(&s.url), device(&s.url));
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
async fn a_late_offline_edit_never_overwrites_a_newer_one() {
    let s = server().await;
    let (mut a, mut b) = (device(&s.url), device(&s.url));
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
    let (mut a, mut slow) = (device(&s.url), device(&s.url));
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
    let (mut a, mut b) = (device(&s.url), device(&s.url));
    // Both Devices create the same id offline; B gets there first.
    let id = orbit_core::new_id(1000, &[9; 10]);
    b.enqueue(
        Op::TaskCreate {
            id,
            title: "From B".into(),
        },
        1000,
    )
    .unwrap();
    a.enqueue(
        Op::TaskCreate {
            id,
            title: "From A".into(),
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
    let mut offline = device("http://127.0.0.1:1");
    offline.create_task("Buy milk", 1000).unwrap();
    assert!(offline.sync(1000).await.is_err());
    assert_eq!(offline.outbox_len().unwrap(), 1);

    let mut online = device(&s.url);
    online.create_task("Buy bread", 1000).unwrap();
    online.sync(1000).await.unwrap();
    assert_eq!(online.outbox_len().unwrap(), 0);
}
