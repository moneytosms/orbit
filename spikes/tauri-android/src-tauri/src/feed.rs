// PROTOTYPE (issue #29): Rust-owned replica + outbox, callable from Kotlin over JNI with no
// Tauri window. One process-global connection, so Tauri commands and the widget share it.
use chrono::{Duration, Local, NaiveDate};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{io::{Read, Write}, net::TcpStream, sync::{Mutex, MutexGuard, OnceLock}, time::Instant};

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();
const SERVER: &str = "10.0.2.2:8787";

pub fn db(dir: &str) -> MutexGuard<'static, Connection> {
    DB.get_or_init(|| {
        let c = Connection::open(format!("{dir}/PROTOTYPE-wipe-me.db")).unwrap();
        c.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS task(id TEXT PRIMARY KEY, title TEXT, due TEXT, project TEXT, recurrence TEXT, done INT DEFAULT 0);
            CREATE TABLE IF NOT EXISTS outbox(uuid TEXT PRIMARY KEY, body TEXT);").unwrap();
        Mutex::new(c)
    }).lock().unwrap()
}

fn http(method: &str, path: &str, body: &str) -> std::io::Result<String> {
    let mut s = TcpStream::connect(SERVER)?;
    write!(s, "{method} {path} HTTP/1.0\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len())?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    if !out.starts_with("HTTP/1.0 200") { return Err(std::io::Error::other(out)); }
    Ok(out.split_once("\r\n\r\n").map(|x| x.1.to_string()).unwrap_or_default())
}

/// Source evaluation, standing in for orbit-core View evaluation.
pub fn rows(dir: &str, source: &str) -> Value {
    let c = db(dir);
    let sql = match source {
        "inbox" => "SELECT id,title,due,project,recurrence FROM task WHERE done=0 AND project='Inbox' ORDER BY title",
        _ => "SELECT id,title,due,project,recurrence FROM task WHERE done=0 AND due IS NOT NULL ORDER BY due,title",
    };
    let mut st = c.prepare(sql).unwrap();
    let today = Local::now().date_naive().to_string();
    let v: Vec<Value> = st.query_map([], |r| Ok(json!({
        "id": r.get::<_, String>(0)?, "title": r.get::<_, String>(1)?, "due": r.get::<_, Option<String>>(2)?,
        "project": r.get::<_, String>(3)?, "recurring": r.get::<_, Option<String>>(4)?.is_some(),
    }))).unwrap().flatten().map(|mut x| { x["overdue"] = json!(x["due"].as_str().is_some_and(|d| d < today.as_str())); x }).collect();
    json!(v)
}

/// Widget tick: local optimistic apply + outbox command. Recurring = next occurrence (orbit-core stand-in).
pub fn complete(dir: &str, id: &str) -> Value {
    let c = db(dir);
    let (due, rec): (Option<String>, Option<String>) =
        c.query_row("SELECT due,recurrence FROM task WHERE id=?", [id], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    match (rec.as_deref(), due) {
        (Some("daily"), Some(d)) => {
            let next = NaiveDate::parse_from_str(&d, "%Y-%m-%d").unwrap() + Duration::days(1);
            c.execute("UPDATE task SET due=? WHERE id=?", params![next.to_string(), id]).unwrap();
        }
        _ => { c.execute("UPDATE task SET done=1 WHERE id=?", [id]).unwrap(); }
    }
    let uuid = uuid::Uuid::new_v4().to_string();
    let body = json!({"uuid": uuid, "type": "task_complete", "args": {"id": id}}).to_string();
    c.execute("INSERT INTO outbox VALUES(?,?)", params![uuid, body]).unwrap();
    json!({"ok": true})
}

/// Background sync: flush outbox, then pull server state (server-authoritative, pending commands re-applied by server first).
pub fn refresh(dir: &str) -> Value {
    let t = Instant::now();
    let c = db(dir);
    let pending: Vec<(String, String)> = c.prepare("SELECT uuid,body FROM outbox").unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().flatten().collect();
    for (uuid, body) in &pending {
        if let Err(e) = http("POST", "/sync", body) { return json!({"error": e.to_string(), "stage": "flush"}); }
        c.execute("DELETE FROM outbox WHERE uuid=?", [uuid]).unwrap();
    }
    let tasks: Vec<Value> = match http("GET", "/tasks", "") {
        Ok(b) => serde_json::from_str(&b).unwrap(),
        Err(e) => return json!({"error": e.to_string(), "stage": "pull"}),
    };
    c.execute("DELETE FROM task", []).unwrap();
    for t in &tasks {
        c.execute("INSERT INTO task VALUES(?,?,?,?,?,?)", params![t["id"].as_str(), t["title"].as_str(), t["due"].as_str(),
            t["project"].as_str(), t["recurrence"].as_str(), t["done"].as_bool().unwrap_or(false) as i32]).unwrap();
    }
    json!({"flushed": pending.len(), "pulled": tasks.len(), "ms": t.elapsed().as_millis()})
}

pub fn outbox_len(dir: &str) -> i64 { db(dir).query_row("SELECT count(*) FROM outbox", [], |r| r.get(0)).unwrap() }

#[cfg(target_os = "android")]
mod jni_api {
    use jni::{objects::{JClass, JString}, sys::jstring, JNIEnv};
    fn s(env: &mut JNIEnv, j: &JString) -> String { env.get_string(j).unwrap().into() }
    fn ret(env: &mut JNIEnv, v: serde_json::Value) -> jstring { env.new_string(v.to_string()).unwrap().into_raw() }

    #[no_mangle]
    pub extern "system" fn Java_dev_orbit_spike_Core_rows<'l>(mut env: JNIEnv<'l>, _: JClass<'l>, dir: JString<'l>, src: JString<'l>) -> jstring {
        let (d, x) = (s(&mut env, &dir), s(&mut env, &src)); ret(&mut env, super::rows(&d, &x))
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_orbit_spike_Core_complete<'l>(mut env: JNIEnv<'l>, _: JClass<'l>, dir: JString<'l>, id: JString<'l>) -> jstring {
        let (d, x) = (s(&mut env, &dir), s(&mut env, &id)); ret(&mut env, super::complete(&d, &x))
    }
    #[no_mangle]
    pub extern "system" fn Java_dev_orbit_spike_Core_refresh<'l>(mut env: JNIEnv<'l>, _: JClass<'l>, dir: JString<'l>) -> jstring {
        let d = s(&mut env, &dir); ret(&mut env, super::refresh(&d))
    }
}
