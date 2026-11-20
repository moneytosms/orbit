//! The one process-global replica: a single SQLite connection behind a mutex, used by Kotlin over JNI
//! (share target, WorkManager) with no window. Rust is the only SQLite writer and the only sender.
#![cfg_attr(not(target_os = "android"), allow(dead_code))] // JNI entry points are Android-only
use orbit_client::{Client, Result};
use orbit_core::{Op, Row, Uuid, goal, next, parse, reminder, routine, view};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

/// Longest a sync may hold the connection, so a stalled network never blocks a capture for long.
const SYNC_TIMEOUT: Duration = Duration::from_secs(20);

/// (dir, server url, client). A different dir or url swaps it, dropping the old connection first.
static CLIENT: Mutex<Option<(String, String, Client)>> = Mutex::new(None);

fn with<T>(dir: &str, url: &str, f: impl FnOnce(&mut Client) -> Result<T>) -> Result<T> {
    let mut g = CLIENT.lock().unwrap_or_else(|e| e.into_inner());
    if g.as_ref().is_none_or(|(d, u, _)| d != dir || u != url) {
        *g = None;
        std::fs::create_dir_all(dir)?;
        let db = Connection::open(Path::new(dir).join("replica.db"))?;
        *g = Some((dir.into(), url.into(), Client::open(db, url)?));
    }
    f(&mut g.as_mut().expect("just opened").2)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The device's local "now" as `YYYY-MM-DDTHH:MM`.
fn now_local() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M").to_string()
}

/// Days since 1970-01-01 for a `YYYY-MM-DD` date.
fn day_number(day: &str) -> Result<i64> {
    let d = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d")?;
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).expect("valid epoch");
    Ok((d - epoch).num_days())
}

/// An empty id means no filter.
fn area_filter(area: &str) -> Result<Option<Uuid>> {
    Ok(if area.is_empty() {
        None
    } else {
        Some(area.parse()?)
    })
}

/// The names quick-add can resolve against.
fn known(rows: &[Row]) -> parse::Known {
    let mut k = parse::Known::default();
    for r in rows {
        match r {
            Row::Area(a) => k.areas.push(a.name.clone()),
            Row::Project(p) => k.areas.push(p.name.clone()),
            Row::Goal(g) => k.goals.push(g.name.clone()),
            Row::Page(p) => k.pages.push(p.title.clone()),
            Row::Label(l) => k.labels.push(l.name.clone()),
            _ => {}
        }
    }
    k
}

/// Queues an Inbox task offline, with quick-add syntax applied to its title, due, priority and
/// labels. No network, no UI. Place, goal and page tokens are parsed but not applied yet.
pub fn capture(dir: &str, text: &str) -> Result<Value> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty capture".into());
    }
    with(dir, "", |c| {
        let rows = c.rows()?;
        let parsed = parse::parse(text, &now_local(), &known(&rows));
        if parsed.title.is_empty() {
            return Err("empty capture".into());
        }
        let label_ids: Vec<Uuid> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Label(l) if parsed.labels.contains(&l.name) => Some(l.id),
                _ => None,
            })
            .collect();
        let id = c.create_task(&parsed.title, now_ms())?;
        let mut patch = serde_json::Map::new();
        if let Some(d) = &parsed.due {
            patch.insert("due".into(), json!({ "date": d.date, "time": d.time }));
        }
        if let Some(p) = parsed.priority {
            patch.insert("priority".into(), json!(p));
        }
        if !label_ids.is_empty() {
            patch.insert("label_ids".into(), json!(label_ids));
        }
        if !patch.is_empty() {
            c.update_task(id, Value::Object(patch), now_ms())?;
        }
        Ok(json!({ "id": id }))
    })
}

/// Flushes the outbox in HLC (enqueue) order and merges the server's delta. The token comes from
/// the Keystore on every call and is dropped from SQLite again, so it never rests there.
pub fn sync(dir: &str, server: &str, device: &str, token: &str) -> Result<Value> {
    let device: Uuid = device.parse()?;
    with(dir, server, |c| {
        c.set_auth(device, token)?;
        let before = c.outbox_len()?;
        let res = tauri::async_runtime::block_on(async {
            tokio::time::timeout(SYNC_TIMEOUT, c.sync(now_ms())).await
        });
        c.logout()?;
        Ok(match res {
            Err(_) => json!({ "error": "timeout" }),
            Ok(Err(e)) => {
                json!({ "error": e.to_string(), "signed_out": orbit_client::is_signed_out(e.as_ref()) })
            }
            Ok(Ok(rejected)) => {
                json!({ "flushed": before.saturating_sub(c.outbox_len()?), "rejected": rejected.len() })
            }
        })
    })
}

/// Reminders to show at `now_local`, soonest first, as `orbit_core::reminder::Fire`s.
pub fn reminders(dir: &str, now_local: &str) -> Result<Value> {
    with(dir, "", |c| {
        let rows = c.rows()?;
        let cfg = reminder::Config {
            default_reminder: rows
                .iter()
                .find_map(|r| match r {
                    Row::Settings(s) => s.default_reminder,
                    _ => None,
                })
                .unwrap_or(true),
        };
        Ok(serde_json::to_value(reminder::schedule(
            &rows, now_local, &cfg,
        ))?)
    })
}

/// The groups of the view `view_id` (stored, else built-in) evaluated for `today`.
pub fn view(dir: &str, view_id: &str, today: &str) -> Result<Value> {
    let id: Uuid = view_id.parse()?;
    with(dir, "", |c| {
        let rows = c.rows()?;
        let v = view::view_or_builtin(id, &rows).ok_or("unknown view")?;
        Ok(serde_json::to_value(view::evaluate(&v, &rows, today))?)
    })
}

/// Every view a widget can show: the stored ones, then the built-ins.
pub fn view_sources(dir: &str) -> Result<Value> {
    with(dir, "", |c| {
        let rows = c.rows()?;
        let all: Vec<view::View> = rows
            .iter()
            .filter_map(|r| match r {
                Row::View(v) => Some(v.clone()),
                _ => None,
            })
            .chain(view::builtins())
            .collect();
        Ok(serde_json::to_value(all)?)
    })
}

/// What's next at `now_time` (`HH:MM`) on `today`.
pub fn whats_next(dir: &str, today: &str, now_time: &str) -> Result<Value> {
    with(dir, "", |c| {
        let rows = c.rows()?;
        let items = next::whats_next(&rows, today, now_time, &next::Cfg::default());
        Ok(serde_json::to_value(items)?)
    })
}

/// The Routines under `area` (all when empty), with whether each is due, paused or ended on
/// `today` and its streaks.
pub fn routines(dir: &str, today: &str, area: &str) -> Result<Value> {
    let area = area_filter(area)?;
    let day = day_number(today)?;
    with(dir, "", |c| {
        let rows = c.rows()?;
        let checkins: Vec<routine::Checkin> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Checkin(k) => Some(k.clone()),
                _ => None,
            })
            .collect();
        let mut out = vec![];
        for r in &rows {
            let Row::Routine(r) = r else { continue };
            if r.trashed_at.is_some()
                || r.archived_at.is_some()
                || area.is_some_and(|a| a != r.parent_id)
            {
                continue;
            }
            out.push(json!({
                "id": r.id,
                "name": r.name,
                "parent_id": r.parent_id,
                "due_today": routine::due_on(r, day),
                "paused": routine::is_paused(r, today),
                "ended": routine::is_ended(r, today),
                "stats": routine::stats(r, &checkins, today),
            }));
        }
        Ok(json!(out))
    })
}

/// The Goals under `area` (all when empty), with progress and whether they are behind pace.
pub fn goals(dir: &str, today: &str, area: &str) -> Result<Value> {
    let area = area_filter(area)?;
    with(dir, "", |c| {
        let rows = c.rows()?;
        let mut out = vec![];
        for r in &rows {
            let Row::Goal(g) = r else { continue };
            if area.is_some_and(|a| a != g.parent_id) {
                continue;
            }
            let progress = goal::progress(g, &rows);
            out.push(json!({
                "id": g.id,
                "name": g.name,
                "parent_id": g.parent_id,
                "progress": progress,
                "behind": goal::behind_pace(g, progress, today),
            }));
        }
        Ok(json!(out))
    })
}

fn queue(dir: &str, op: Op) -> Result<Value> {
    with(dir, "", |c| {
        c.enqueue(op, now_ms()).map(|_| json!({ "ok": true }))
    })
}

/// Completes a Task; `today` is the completer's local `YYYY-MM-DD`, which `every!` counts from.
pub fn complete(dir: &str, task_id: &str, today: &str) -> Result<Value> {
    let id: Uuid = task_id.parse()?;
    queue(
        dir,
        Op::TaskComplete {
            id,
            today: Some(today.into()),
        },
    )
}

/// Ticks a Routine's day as `state` (`done` or `skipped`).
pub fn checkin(dir: &str, routine_id: &str, date: &str, state: &str) -> Result<Value> {
    let routine_id: Uuid = routine_id.parse()?;
    let state: routine::CheckState = serde_json::from_value(json!(state))?;
    queue(
        dir,
        Op::CheckinSet {
            id: routine::checkin_id(routine_id, date),
            routine_id,
            date: date.into(),
            state,
        },
    )
}

/// Clears a Routine's tick for `date`.
pub fn checkin_clear(dir: &str, routine_id: &str, date: &str) -> Result<Value> {
    let routine_id: Uuid = routine_id.parse()?;
    queue(
        dir,
        Op::CheckinClear {
            id: routine::checkin_id(routine_id, date),
        },
    )
}

/// Adds a one-off reminder on a Task at `until_local` (`YYYY-MM-DDTHH:MM`).
pub fn reminder_snooze(dir: &str, task_id: &str, until_local: &str) -> Result<Value> {
    let task_id: Uuid = task_id.parse()?;
    queue(
        dir,
        Op::ReminderSnooze {
            task_id,
            at: until_local.into(),
        },
    )
}

/// Queues any op, given as its JSON form (`{"type": ..., ...}`).
pub fn enqueue(dir: &str, op_json: &str) -> Result<Value> {
    let op: Op = serde_json::from_str(op_json)?;
    queue(dir, op)
}

#[cfg(target_os = "android")]
pub mod jni_api {
    use jni::JNIEnv;
    use jni::objects::{JClass, JString};
    use jni::sys::jstring;

    fn s(env: &mut JNIEnv, j: &JString) -> String {
        env.get_string(j).map(Into::into).unwrap_or_default()
    }

    fn ret(env: &mut JNIEnv, v: super::Result<serde_json::Value>) -> jstring {
        let v = v.unwrap_or_else(|e| serde_json::json!({ "error": e.to_string() }));
        env.new_string(v.to_string())
            .map_or(std::ptr::null_mut(), |j| j.into_raw())
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_capture<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        text: JString<'l>,
    ) -> jstring {
        let (d, t) = (s(&mut env, &dir), s(&mut env, &text));
        ret(&mut env, super::capture(&d, &t))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_sync<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        server: JString<'l>,
        device: JString<'l>,
        token: JString<'l>,
    ) -> jstring {
        let (d, u, v, t) = (
            s(&mut env, &dir),
            s(&mut env, &server),
            s(&mut env, &device),
            s(&mut env, &token),
        );
        ret(&mut env, super::sync(&d, &u, &v, &t))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_reminders<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        now_local: JString<'l>,
    ) -> jstring {
        let (d, n) = (s(&mut env, &dir), s(&mut env, &now_local));
        ret(&mut env, super::reminders(&d, &n))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_view<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        view_id: JString<'l>,
        today: JString<'l>,
    ) -> jstring {
        let (d, v, t) = (
            s(&mut env, &dir),
            s(&mut env, &view_id),
            s(&mut env, &today),
        );
        ret(&mut env, super::view(&d, &v, &t))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_view_1sources<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
    ) -> jstring {
        let d = s(&mut env, &dir);
        ret(&mut env, super::view_sources(&d))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_whats_1next<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        today: JString<'l>,
        now_time: JString<'l>,
    ) -> jstring {
        let (d, t, n) = (
            s(&mut env, &dir),
            s(&mut env, &today),
            s(&mut env, &now_time),
        );
        ret(&mut env, super::whats_next(&d, &t, &n))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_routines<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        today: JString<'l>,
        area: JString<'l>,
    ) -> jstring {
        let (d, t, a) = (s(&mut env, &dir), s(&mut env, &today), s(&mut env, &area));
        ret(&mut env, super::routines(&d, &t, &a))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_goals<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        today: JString<'l>,
        area: JString<'l>,
    ) -> jstring {
        let (d, t, a) = (s(&mut env, &dir), s(&mut env, &today), s(&mut env, &area));
        ret(&mut env, super::goals(&d, &t, &a))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_complete<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        task_id: JString<'l>,
        today: JString<'l>,
    ) -> jstring {
        let (d, i, t) = (
            s(&mut env, &dir),
            s(&mut env, &task_id),
            s(&mut env, &today),
        );
        ret(&mut env, super::complete(&d, &i, &t))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_checkin<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        routine_id: JString<'l>,
        date: JString<'l>,
        state: JString<'l>,
    ) -> jstring {
        let (d, r, dt, st) = (
            s(&mut env, &dir),
            s(&mut env, &routine_id),
            s(&mut env, &date),
            s(&mut env, &state),
        );
        ret(&mut env, super::checkin(&d, &r, &dt, &st))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_checkin_1clear<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        routine_id: JString<'l>,
        date: JString<'l>,
    ) -> jstring {
        let (d, r, dt) = (
            s(&mut env, &dir),
            s(&mut env, &routine_id),
            s(&mut env, &date),
        );
        ret(&mut env, super::checkin_clear(&d, &r, &dt))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_reminder_1snooze<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        task_id: JString<'l>,
        until_local: JString<'l>,
    ) -> jstring {
        let (d, i, u) = (
            s(&mut env, &dir),
            s(&mut env, &task_id),
            s(&mut env, &until_local),
        );
        ret(&mut env, super::reminder_snooze(&d, &i, &u))
    }

    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_orbit_app_Core_enqueue<'l>(
        mut env: JNIEnv<'l>,
        _: JClass<'l>,
        dir: JString<'l>,
        op_json: JString<'l>,
    ) -> jstring {
        let (d, o) = (s(&mut env, &dir), s(&mut env, &op_json));
        ret(&mut env, super::enqueue(&d, &o))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_queue_in_order_and_survive_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_str().unwrap();
        capture(d, " one ").unwrap();
        capture(d, "two").unwrap();
        assert_eq!(with(d, "", |c| c.outbox_len()).unwrap(), 2);
        // A new url swaps the connection; the queue is on disk.
        assert_eq!(with(d, "http://x", |c| c.outbox_len()).unwrap(), 2);
    }

    #[test]
    fn capture_applies_quick_add_syntax() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_str().unwrap();
        with(d, "", |c| {
            c.enqueue(
                Op::LabelCreate {
                    id: Uuid::from_u128(7),
                    name: "errands".into(),
                },
                now_ms(),
            )
            .map(drop)
        })
        .unwrap();
        let id: Uuid = capture(d, "buy milk #errands p1").unwrap()["id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let task = with(d, "", |c| c.task(id)).unwrap().unwrap();
        assert_eq!(task.title, "buy milk");
        assert_eq!(task.priority, 1);
        assert_eq!(task.label_ids, vec![Uuid::from_u128(7)]);
    }

    #[test]
    fn capture_rejects_blank_and_token_only_input() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path().to_str().unwrap();
        assert!(capture(d, "   ").is_err());
        assert!(capture(d, "p1").is_err());
    }

    #[test]
    fn day_number_counts_from_epoch() {
        assert_eq!(day_number("1970-01-01").unwrap(), 0);
        assert_eq!(day_number("1970-01-02").unwrap(), 1);
    }
}
