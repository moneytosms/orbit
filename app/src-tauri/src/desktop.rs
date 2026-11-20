//! Desktop server address and updates. The packaged window loads the bundled UI only long enough to
//! send itself to the saved Orbit server (or the bundled `server.html` page); the server-hosted UI then
//! reaches the few commands in `capabilities/desktop.json`.
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Manager, Url, Webview};
use tauri_plugin_notification::NotificationExt;

const BUNDLED: &str = "tauri://localhost/server.html";

fn file() -> PathBuf {
    crate::token::data_dir().join("server")
}

/// The saved server URL, without a trailing slash.
pub fn saved() -> Option<String> {
    let s = std::fs::read_to_string(file()).ok()?;
    let s = s.trim().trim_end_matches('/');
    (!s.is_empty()).then(|| s.to_string())
}

fn save(url: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(crate::token::data_dir())?;
    std::fs::write(file(), url)
}

fn is_bundled(url: &Url) -> bool {
    url.scheme() == "tauri" && url.host_str() == Some("localhost")
}

/// Commands run only for the bundled pages or the saved server: a page the window wandered to must not
/// reach the token or the server setting. Debug builds also trust the dev server.
pub fn trusted(window: &tauri::WebviewWindow) -> Result<(), String> {
    let url = window.url().map_err(|e| e.to_string())?;
    if is_bundled(&url) || (cfg!(debug_assertions) && url.host_str() == Some("localhost")) {
        return Ok(());
    }
    match saved().and_then(|s| s.parse::<Url>().ok()) {
        Some(s) if s.origin() == url.origin() => Ok(()),
        _ => Err("untrusted origin".into()),
    }
}

fn reachable(server: &str) -> bool {
    let Ok(url) = server.parse::<Url>() else {
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

/// Saves the server URL (from the first-run page) and opens it in the window.
#[tauri::command]
pub fn set_server(window: tauri::WebviewWindow, url: String) -> Result<(), String> {
    let url: Url = url.trim().parse().map_err(|_| "not a URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("use an http or https URL".into());
    }
    trusted(&window)?;
    store(&url)?;
    window.navigate(url).map_err(|e| e.to_string())
}

fn store(url: &Url) -> Result<String, String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err("use an http or https URL".into());
    }
    let server = url.as_str().trim_end_matches('/').to_string();
    // A token for another server is useless here; the new UI provisions its own.
    if crate::token::load().is_some_and(|t| t.server.trim_end_matches('/') != server) {
        crate::token::clear();
    }
    save(&server).map_err(|e| e.to_string())?;
    Ok(server)
}

/// `--server <url>`: saves the server and, if the window is up, opens it there.
pub fn apply_flag(app: &AppHandle, args: &[String]) {
    let Some(raw) = args
        .iter()
        .position(|a| a == "--server")
        .and_then(|i| args.get(i + 1))
    else {
        return;
    };
    match raw
        .trim()
        .parse::<Url>()
        .map_err(|e| e.to_string())
        .and_then(|u| store(&u).map(|_| u))
    {
        Ok(url) => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.navigate(url);
            }
        }
        Err(e) => eprintln!("orbit: --server {raw}: {e}"),
    }
}

/// Once the bundled launcher page has loaded, send the window to the saved server or the server page.
/// A navigation sooner than this is dropped.
pub fn on_page_load(webview: &Webview, url: &Url, event: PageLoadEvent) {
    if event != PageLoadEvent::Finished || !is_bundled(url) || url.path() != "/" {
        return;
    }
    let webview = webview.clone();
    // Off this thread: the reachability check can block for seconds.
    tauri::async_runtime::spawn_blocking(move || {
        let to = match saved() {
            Some(s) if reachable(&s) => s.parse::<Url>().ok(),
            Some(s) => Url::parse_with_params(BUNDLED, [("down", s)]).ok(),
            None => BUNDLED.parse().ok(),
        };
        if let Some(to) = to {
            let _ = webview.navigate(to);
        }
    });
}

/// Checks for a new AppImage at start, installs it and says so; the user restarts to apply it.
pub fn check_update(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_updater::UpdaterExt;
        let result: Result<(), Box<dyn std::error::Error>> = async {
            let Some(update) = app.updater()?.check().await? else {
                return Ok(());
            };
            update.download_and_install(|_, _| {}, || {}).await?;
            app.notification()
                .builder()
                .title("Orbit updated")
                .body("Orbit updated, restart to apply")
                .show()?;
            Ok(())
        }
        .await;
        if let Err(e) = result {
            eprintln!("orbit: update check failed: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_origin() {
        assert!(is_bundled(&"tauri://localhost/".parse().unwrap()));
        assert!(!is_bundled(&"https://orbit.example.com/".parse().unwrap()));
        assert!(!is_bundled(&"http://localhost:5173/".parse().unwrap()));
    }
}
