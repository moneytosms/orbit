//! The one process-global replica: a single SQLite connection behind a mutex, used by Kotlin over JNI
//! (share target, WorkManager) with no window. Rust is the only SQLite writer and the only sender.
#![cfg_attr(not(target_os = "android"), allow(dead_code))] // JNI entry points are Android-only
use orbit_client::{Client, Result};
use orbit_core::Uuid;
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

/// Queues an Inbox task offline. No network, no UI.
pub fn capture(dir: &str, text: &str) -> Result<Value> {
    let text = text.trim();
    if text.is_empty() {
        return Err("empty capture".into());
    }
    let id = with(dir, "", |c| c.create_task(text, now_ms()))?;
    Ok(json!({ "id": id }))
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
}
