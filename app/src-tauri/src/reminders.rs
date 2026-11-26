//! Background replica that fires reminder notifications while the app runs, even hidden to tray.
use crate::token;
use orbit_client::Client;
use orbit_core::Op;
use orbit_core::Uuid;
use orbit_core::reminder::{self, Fire};
use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

fn fire_key(f: &Fire) -> String {
    format!(
        "{}|{}|{}",
        f.task_id
            .or(f.routine_id)
            .map(|u| u.to_string())
            .unwrap_or_default(),
        f.reminder_id.map(|u| u.to_string()).unwrap_or_default(),
        f.at
    )
}

/// The reminders to show now: due at or before `now_local` and not fired before. Marks them fired.
pub fn due_now(fires: Vec<Fire>, now_local: &str, fired: &mut HashSet<String>) -> Vec<Fire> {
    fires
        .into_iter()
        .filter(|f| f.at.as_str() <= now_local)
        .filter(|f| fired.insert(fire_key(f)))
        .collect()
}

fn body(f: &Fire) -> String {
    if f.late {
        format!("Was due {}", f.at.replace('T', " "))
    } else {
        f.at.get(11..).unwrap_or_default().to_string()
    }
}

/// A tap on a notification button: complete the task, or snooze its reminder by some minutes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Debug, PartialEq)]
enum Tap {
    Complete(Uuid),
    Snooze(Uuid, i64),
}

/// When a snooze of `minutes` from `now` lands, as local `YYYY-MM-DDTHH:MM`.
fn snooze_at(now: chrono::NaiveDateTime, minutes: i64) -> String {
    (now + chrono::TimeDelta::minutes(minutes))
        .format("%Y-%m-%dT%H:%M")
        .to_string()
}

/// Shows a reminder. Task reminders carry Complete and Snooze buttons where the platform's
/// notification service supports them (Linux); a tap is sent back on `tx`.
#[cfg(target_os = "linux")]
fn show(app: &AppHandle, f: &Fire, tx: &Sender<Tap>) {
    let mut n = notify_rust::Notification::new();
    n.appname("Orbit").summary(&f.title).body(&body(f));
    if f.task_id.is_some() {
        n.action("complete", "Complete")
            .action("snooze10", "Snooze 10 min")
            .action("snooze60", "Snooze 1 h");
    }
    match (n.show(), f.task_id) {
        (Ok(handle), Some(id)) => {
            let tx = tx.clone();
            // Blocks until the notification is closed, so it gets its own thread.
            std::thread::spawn(move || {
                handle.wait_for_action(|action| {
                    let tap = match action {
                        "complete" => Tap::Complete(id),
                        "snooze10" => Tap::Snooze(id, 10),
                        "snooze60" => Tap::Snooze(id, 60),
                        _ => return,
                    };
                    let _ = tx.send(tap);
                });
            });
        }
        (Ok(_), None) => {}
        (Err(_), _) => plain(app, f),
    }
}

/// Windows and others: the notification plugin has no action buttons here.
#[cfg(not(target_os = "linux"))]
fn show(app: &AppHandle, f: &Fire, _tx: &Sender<Tap>) {
    plain(app, f);
}

fn plain(app: &AppHandle, f: &Fire) {
    let _ = app
        .notification()
        .builder()
        .title(&f.title)
        .body(body(f))
        .show();
}

/// Writes a tapped button's change and syncs it.
async fn apply(client: &mut Client, tap: Tap) {
    let now = chrono::Local::now().naive_local();
    let op = match tap {
        Tap::Complete(id) => Op::TaskComplete {
            id,
            today: Some(now.format("%Y-%m-%d").to_string()),
        },
        Tap::Snooze(task_id, minutes) => Op::ReminderSnooze {
            task_id,
            at: snooze_at(now, minutes),
        },
    };
    if client.enqueue(op, now_ms()).is_ok() {
        let _ = client.sync(now_ms()).await;
    }
}

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

/// One tick: sync, work out what is due, notify, spend. Returns an error only for display/logging.
async fn tick(
    app: &AppHandle,
    client: &mut Client,
    fired: &mut HashSet<String>,
    device: &str,
    taps: &Sender<Tap>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let trace = std::env::var_os("ORBIT_SYNC_TRACE")
        .is_some_and(|value| value == std::ffi::OsStr::new("1"));
    let started_at = trace.then(now_ms);
    let sync = client.sync(now_ms()).await;
    if let Some(started_at) = started_at {
        eprintln!(
            "native_sync_trace device={device} started_at={started_at} completed_at={}",
            now_ms()
        );
    }
    if let Err(e) = sync {
        if orbit_client::is_signed_out(e.as_ref()) {
            token::clear();
        }
        return Err(e);
    }
    let rows = client.rows()?;
    let cfg = reminder::Config::from_rows(&rows);
    let now_local = chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string();
    let fires = reminder::schedule(&rows, &now_local, &cfg);
    let mut spent = false;
    for f in due_now(fires, &now_local, fired) {
        show(app, &f, taps);
        if let (Some(task_id), Some(id)) = (f.task_id, f.reminder_id) {
            // Offsets re-apply per occurrence and core may refuse; `fired` already stops repeats.
            spent |= client
                .enqueue(Op::ReminderSpend { task_id, id }, now_ms())
                .is_ok();
        }
    }
    if spent {
        client.sync(now_ms()).await?;
    }
    Ok(())
}

/// Starts the 60 s loop on the async runtime.
pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut fired = HashSet::new();
        let (taps, rx) = mpsc::channel::<Tap>();
        let rx = Arc::new(Mutex::new(rx));
        let mut open: Option<(Client, String)> = None;
        loop {
            match token::load() {
                None => open = None,
                Some(t) => {
                    if open.as_ref().is_none_or(|(_, key)| *key != t.token) {
                        open = open_client(&t).map(|c| (c, t.token.clone()));
                    }
                    if let Some((client, _)) = open.as_mut()
                        && let Err(e) = tick(&app, client, &mut fired, &t.device, &taps).await
                    {
                        eprintln!("orbit reminders: {e}");
                    }
                }
            }
            // Waits a minute, or less when a notification button is tapped.
            if let Some(tap) = next_tap(&rx).await
                && let Some((client, _)) = open.as_mut()
            {
                apply(client, tap).await;
            }
        }
    });
}

async fn next_tap(rx: &Arc<Mutex<Receiver<Tap>>>) -> Option<Tap> {
    // Tauri's runtime is tokio; wait on a blocking thread to avoid a direct tokio dependency.
    let rx = Arc::clone(rx);
    tauri::async_runtime::spawn_blocking(move || {
        let rx = rx.lock().ok()?;
        rx.recv_timeout(Duration::from_secs(60)).ok()
    })
    .await
    .ok()
    .flatten()
}

fn open_client(t: &token::Stored) -> Option<Client> {
    let dir = token::data_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let db = rusqlite::Connection::open(dir.join("replica.db")).ok()?;
    let mut c = Client::open(db, &t.server).ok()?;
    c.set_auth(t.device.parse().ok()?, &t.token).ok()?;
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_core::Uuid;

    fn fire(n: u128, at: &str) -> Fire {
        Fire {
            task_id: Some(Uuid::from_u128(n)),
            routine_id: None,
            reminder_id: None,
            at: at.into(),
            late: false,
            title: "T".into(),
        }
    }

    #[test]
    fn snooze_lands_in_local_time_across_midnight() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 11, 30)
            .unwrap()
            .and_hms_opt(23, 55, 0)
            .unwrap();
        assert_eq!(snooze_at(now, 10), "2026-12-01T00:05");
        assert_eq!(snooze_at(now, 60), "2026-12-01T00:55");
    }

    #[test]
    fn fires_due_ones_once() {
        let mut fired = HashSet::new();
        let fires = || vec![fire(1, "2026-01-01T09:00"), fire(2, "2026-01-01T09:30")];
        let got = due_now(fires(), "2026-01-01T09:00", &mut fired);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].at, "2026-01-01T09:00");
        assert!(due_now(fires(), "2026-01-01T09:00", &mut fired).is_empty());
        let later = due_now(fires(), "2026-01-01T09:31", &mut fired);
        assert_eq!(later.len(), 1);
        assert_eq!(later[0].task_id, Some(Uuid::from_u128(2)));
    }

    #[test]
    fn same_task_at_a_new_time_fires_again() {
        let mut fired = HashSet::new();
        assert_eq!(
            due_now(
                vec![fire(1, "2026-01-01T09:00")],
                "2026-01-01T09:00",
                &mut fired
            )
            .len(),
            1
        );
        assert_eq!(
            due_now(
                vec![fire(1, "2026-01-02T09:00")],
                "2026-01-02T09:00",
                &mut fired
            )
            .len(),
            1
        );
    }
}
