//! Background replica that fires reminder notifications while the app runs, even hidden to tray.
use crate::token;
use orbit_client::Client;
use orbit_core::reminder::{self, Fire};
use orbit_core::{Op, Row};
use std::collections::HashSet;
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

fn now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis().max(0) as u64
}

/// One tick: sync, work out what is due, notify, spend. Returns an error only for display/logging.
async fn tick(
    app: &AppHandle,
    client: &mut Client,
    fired: &mut HashSet<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Err(e) = client.sync(now_ms()).await {
        if orbit_client::is_signed_out(e.as_ref()) {
            token::clear();
        }
        return Err(e);
    }
    let rows = client.rows()?;
    let cfg = reminder::Config {
        default_reminder: rows
            .iter()
            .find_map(|r| match r {
                Row::Settings(s) => s.default_reminder,
                _ => None,
            })
            .unwrap_or(true),
    };
    let now_local = chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string();
    let fires = reminder::schedule(&rows, &now_local, &cfg);
    let mut spent = false;
    for f in due_now(fires, &now_local, fired) {
        let _ = app
            .notification()
            .builder()
            .title(&f.title)
            .body(body(&f))
            .show();
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
        let mut open: Option<(Client, String)> = None;
        loop {
            match token::load() {
                None => open = None,
                Some(t) => {
                    if open.as_ref().is_none_or(|(_, key)| *key != t.token) {
                        open = open_client(&t).map(|c| (c, t.token.clone()));
                    }
                    if let Some((client, _)) = open.as_mut()
                        && let Err(e) = tick(&app, client, &mut fired).await
                    {
                        eprintln!("orbit reminders: {e}");
                    }
                }
            }
            tokio_sleep().await;
        }
    });
}

async fn tokio_sleep() {
    // Tauri's runtime is tokio; sleep on a blocking thread to avoid a direct tokio dependency.
    let _ =
        tauri::async_runtime::spawn_blocking(|| std::thread::sleep(Duration::from_secs(60))).await;
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
