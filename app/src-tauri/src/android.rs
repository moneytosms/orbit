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

/// Commands run only for the bundled launcher pages or the saved server: the capability allows any
/// http(s) origin, so a page the WebView wandered to must not reach the token or the server setting.
fn trusted(app: &tauri::AppHandle, window: &tauri::WebviewWindow) -> Result<(), String> {
    let url = window.url().map_err(|e| e.to_string())?;
    if url.host_str() == Some("tauri.localhost") {
        return Ok(());
    }
    let saved = call(app, "getServer", json!({})).ok();
    let saved: Option<tauri::Url> = saved
        .as_ref()
        .and_then(|v| v["server"].as_str()?.parse().ok());
    match saved {
        Some(s) if s.origin() == url.origin() => Ok(()),
        _ => Err("untrusted origin".into()),
    }
}

// Async so they run off the main thread, which the Kotlin side needs to answer.
#[tauri::command]
async fn set_token(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    server: String,
    device: String,
    token: String,
) -> Result<(), String> {
    trusted(&app, &window)?;
    call(
        &app,
        "setToken",
        json!({ "server": server, "device": device, "token": token }),
    )
    .map(drop)
}

#[tauri::command]
async fn clear_token(app: tauri::AppHandle, window: tauri::WebviewWindow) {
    if trusted(&app, &window).is_err() {
        return;
    }
    let _ = call(&app, "clearToken", json!({}));
}

#[tauri::command]
async fn has_token(app: tauri::AppHandle, window: tauri::WebviewWindow) -> bool {
    trusted(&app, &window).is_ok()
        && call(&app, "hasToken", json!({})).is_ok_and(|v| v["has"] == true)
}

/// The link a widget tap left (`#/task/<id>` or `#/goals`), once; empty if none.
#[tauri::command]
async fn take_link(app: tauri::AppHandle, window: tauri::WebviewWindow) -> String {
    if trusted(&app, &window).is_err() {
        return String::new();
    }
    call(&app, "takeLink", json!({})).map_or_else(
        |_| String::new(),
        |v| v["link"].as_str().unwrap_or_default().into(),
    )
}

/// The installed Android shell version, which can differ from the server-loaded UI.
#[tauri::command]
async fn shell_version(app: tauri::AppHandle, window: tauri::WebviewWindow) -> String {
    if trusted(&app, &window).is_err() {
        return String::new();
    }
    call(&app, "shellVersion", json!({})).map_or_else(
        |_| String::new(),
        |v| v["version"].as_str().unwrap_or_default().into(),
    )
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
    crate::check_server_url(&url)?;
    trusted(&app, &window)?;
    let server = url.as_str().trim_end_matches('/').to_string();
    call(&app, "setServer", json!({ "server": server }))?;
    // Off this thread: the reachability check can block for seconds. An address that does not answer
    // opens the retry page instead of a dead WebView.
    let to = tauri::async_runtime::spawn_blocking(move || match reachable(&server) {
        true => url.to_string(),
        false => format!("http://tauri.localhost/server.html?down={server}"),
    })
    .await
    .map_err(|e| e.to_string())?;
    window
        .navigate(to.parse().map_err(|_| "not a URL".to_string())?)
        .map_err(|e| e.to_string())
}

/// Whether something accepts a connection at the server URL's host and port within 3 s.
fn reachable(server: &str) -> bool {
    use std::net::ToSocketAddrs;
    let Ok(url) = server.parse::<tauri::Url>() else {
        return false;
    };
    let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default()) else {
        return false;
    };
    (host, port).to_socket_addrs().is_ok_and(|mut a| {
        a.any(|addr| {
            std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(3)).is_ok()
        })
    })
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
            set_server,
            take_link,
            shell_version
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
                // A saved server that does not answer sends the user to the server page, not a dead WebView.
                let to = match server {
                    Some(s) if reachable(s) => s.to_string(),
                    Some(s) if !s.is_empty() => {
                        format!("http://tauri.localhost/server.html?down={s}")
                    }
                    _ => "http://tauri.localhost/server.html".to_string(),
                };
                if let Ok(to) = to.parse::<tauri::Url>() {
                    let _ = webview.navigate(to);
                }
            });
        })
}
