// PROTOTYPE: Rust side of the spike. Schedules reminders through the Kotlin plugin and
// reads the JSON outbox files Kotlin writes (Rust as importer, per issue #18).
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tauri::{plugin::PluginHandle, Manager, Runtime, Wry};

struct Native(PluginHandle<Wry>);

fn outbox_dir<R: Runtime>(app: &tauri::AppHandle<R>) -> PathBuf {
    // Kotlin writes to Context.filesDir/outbox; Tauri app_data_dir = Context.dataDir.
    app.path().app_data_dir().unwrap().join("files")
}

fn read_dir(dir: PathBuf) -> Vec<Value> {
    let Ok(rd) = fs::read_dir(dir) else { return vec![] };
    rd.flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice(&fs::read(e.path()).ok()?).ok())
        .collect()
}

#[tauri::command]
fn state(app: tauri::AppHandle) -> Value {
    let base = outbox_dir(&app);
    let alarms = app.state::<Native>().0.run_mobile_plugin::<Value>("alarms", json!({})).unwrap_or(Value::Null);
    json!({ "pending": read_dir(base.join("outbox")), "sent": read_dir(base.join("sent")), "alarms": alarms })
}

#[tauri::command]
fn schedule_reminder(app: tauri::AppHandle, secs: u64) -> Result<Value, String> {
    app.state::<Native>().0
        .run_mobile_plugin("schedule", json!({ "secs": secs, "title": format!("Spike reminder (+{secs}s)") }))
        .map_err(|e| e.to_string())
}

fn native_plugin() -> tauri::plugin::TauriPlugin<Wry> {
    tauri::plugin::Builder::new("orbit")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            app.manage(Native(api.register_android_plugin("dev.orbit.spike", "OrbitPlugin")?));
            #[cfg(not(target_os = "android"))]
            let _ = (app, api);
            Ok(())
        })
        .build()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(native_plugin())
        .invoke_handler(tauri::generate_handler![state, schedule_reminder])
        .run(tauri::generate_context!())
        .expect("run");
}
