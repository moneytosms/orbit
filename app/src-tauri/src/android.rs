//! Android shell: no tray or reminders here. The Device token lives in the Keystore (Kotlin `OrbitPlugin`),
//! the window loads the user's Orbit server, and background work reaches Rust through `replica` over JNI.
use serde_json::{Value, json};
use tauri::plugin::PluginHandle;
use tauri::{Manager, Wry};

struct Native(PluginHandle<Wry>);

fn call(app: &tauri::AppHandle, cmd: &str, args: Value) -> Result<Value, String> {
    app.state::<Native>()
        .0
        .run_mobile_plugin(cmd, args)
        .map_err(|e| e.to_string())
}

// Async so they run off the main thread, which the Kotlin side needs to answer.
#[tauri::command]
async fn set_token(
    app: tauri::AppHandle,
    server: String,
    device: String,
    token: String,
) -> Result<(), String> {
    call(
        &app,
        "setToken",
        json!({ "server": server, "device": device, "token": token }),
    )
    .map(drop)
}

#[tauri::command]
async fn clear_token(app: tauri::AppHandle) {
    let _ = call(&app, "clearToken", json!({}));
}

#[tauri::command]
async fn has_token(app: tauri::AppHandle) -> bool {
    call(&app, "hasToken", json!({})).is_ok_and(|v| v["has"] == true)
}

/// Saves the server URL (from the first-run page) and opens it in the window.
#[tauri::command]
async fn set_server(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    url: String,
) -> Result<(), String> {
    let url: tauri::Url = url.trim().parse().map_err(|_| "not a URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("use an http or https URL".into());
    }
    call(
        &app,
        "setServer",
        json!({ "server": url.as_str().trim_end_matches('/') }),
    )?;
    window.navigate(url).map_err(|e| e.to_string())
}

pub fn builder() -> tauri::Builder<Wry> {
    let native = tauri::plugin::Builder::<Wry>::new("orbit")
        .setup(|app, api| {
            app.manage(Native(
                api.register_android_plugin("dev.orbit.app", "OrbitPlugin")?,
            ));
            Ok(())
        })
        .build();
    tauri::Builder::default()
        .plugin(native)
        .invoke_handler(tauri::generate_handler![
            set_token,
            clear_token,
            has_token,
            set_server
        ])
        // The bundled index.html is only a launcher: once it has loaded, send the window to the saved
        // server, or to the server-URL page. A navigation sooner than this is dropped.
        .on_page_load(|webview, payload| {
            let u = payload.url();
            if payload.event() != tauri::webview::PageLoadEvent::Finished
                || u.host_str() != Some("tauri.localhost")
                || u.path() != "/"
            {
                return;
            }
            let webview = webview.clone();
            // Off this thread: the Kotlin plugin answers on the main thread.
            tauri::async_runtime::spawn_blocking(move || {
                let server = call(webview.app_handle(), "getServer", json!({})).ok();
                let server = server.as_ref().and_then(|v| v["server"].as_str());
                let to = match server {
                    Some(s) if !s.is_empty() => s,
                    _ => "http://tauri.localhost/server.html",
                };
                if let Ok(to) = to.parse() {
                    let _ = webview.navigate(to);
                }
            });
        })
}
