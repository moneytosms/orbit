//! Desktop server address and updates. The packaged window loads the bundled UI only long enough to
//! send itself to the saved Orbit server (or the bundled `server.html` page); the server-hosted UI then
//! reaches the few commands in `capabilities/desktop.json`.
use std::net::ToSocketAddrs;
use std::path::PathBuf;
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, Emitter, Manager, Url, Webview};
use tauri_plugin_notification::NotificationExt;

const BUNDLED: &str = "tauri://localhost/server.html";
const AUTO_UPDATES: &str = "auto-updates";
const UPDATE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
static LAST_REQUIRED_PROTOCOL: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

fn file() -> PathBuf {
    crate::token::data_dir().join("server")
}

/// The saved server URL, without a trailing slash.
pub fn saved() -> Option<String> {
    let s = std::fs::read_to_string(file()).ok()?;
    let s = s.trim().trim_end_matches('/');
    (!s.is_empty()).then(|| s.to_string())
}

/// The bundled launcher documents, whichever way WebKit names the root of the packaged assets:
/// `tauri://localhost/`, `/index.html` or an empty path all mean "the app's own start page".
fn is_launcher(url: &Url) -> bool {
    is_bundled(url) && matches!(url.path(), "/" | "" | "/index.html")
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

/// Saves the server URL (from the first-run page) and opens it in the window. An address that does
/// not answer opens the retry page instead of a dead WebView.
#[tauri::command]
pub async fn set_server(window: tauri::WebviewWindow, url: String) -> Result<(), String> {
    let url: Url = url.trim().parse().map_err(|_| "not a URL".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("use an http or https URL".into());
    }
    trusted(&window)?;
    let server = store(&url)?;
    // Off the main thread: the reachability check can block for seconds.
    let to = tauri::async_runtime::spawn_blocking(move || match reachable(&server) {
        true => server,
        false => format!("{BUNDLED}?down={server}"),
    })
    .await
    .map_err(|e| e.to_string())?;
    window
        .navigate(to.parse().map_err(|_| "not a URL".to_string())?)
        .map_err(|e| e.to_string())
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
/// Returns whether it navigated, so startup does not ask for a destination twice.
pub fn apply_flag(app: &AppHandle, args: &[String]) -> bool {
    let Some(raw) = args
        .iter()
        .position(|a| a == "--server")
        .and_then(|i| args.get(i + 1))
    else {
        return false;
    };
    match raw
        .trim()
        .parse::<Url>()
        .map_err(|e| e.to_string())
        .and_then(|u| store(&u).map(|_| u))
    {
        Ok(url) => app
            .get_webview_window("main")
            .is_some_and(|w| w.navigate(url).is_ok()),
        Err(e) => {
            eprintln!("orbit: --server {raw}: {e}");
            false
        }
    }
}

/// Where the window goes first: the saved server, the server page when none is saved, or the server
/// page with `?down=` when the saved server does not answer.
fn first_stop() -> String {
    match saved() {
        Some(s) if reachable(&s) => s,
        Some(s) => format!("{BUNDLED}?down={s}"),
        None => BUNDLED.to_string(),
    }
}

/// Sends the window to its first stop as soon as it exists. The window is created with the bundled
/// Svelte UI, which would otherwise land the user on a login screen that has no server to talk to.
pub fn launch(app: &AppHandle) {
    let app = app.clone();
    // Off this thread: the reachability check can block for seconds, and the window may not exist yet.
    std::thread::spawn(move || {
        let window = std::iter::repeat_with(|| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            app.get_webview_window("main")
        })
        .take(60)
        .flatten()
        .next();
        let Some(window) = window else { return };
        if let Ok(to) = first_stop().parse::<Url>() {
            let _ = window.navigate(to);
        }
    });
}

/// Once the bundled launcher page has loaded, send the window to the saved server or the server page.
/// A navigation issued before the page loads is dropped, so this is the second try at the same thing.
pub fn on_page_load(webview: &Webview, url: &Url, event: PageLoadEvent) {
    if event != PageLoadEvent::Finished || !is_launcher(url) {
        return;
    }
    let webview = webview.clone();
    // Off this thread: the reachability check can block for seconds.
    tauri::async_runtime::spawn_blocking(move || {
        if let Ok(to) = first_stop().parse::<Url>() {
            let _ = webview.navigate(to);
        }
    });
}

fn auto_updates_enabled() -> bool {
    auto_updates_at(&crate::token::data_dir().join(AUTO_UPDATES))
}

fn auto_updates_at(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .map(|value| value.trim() != "false")
        .unwrap_or(true)
}

fn save_auto_updates(enabled: bool) -> std::io::Result<()> {
    std::fs::create_dir_all(crate::token::data_dir())?;
    std::fs::write(
        crate::token::data_dir().join(AUTO_UPDATES),
        enabled.to_string(),
    )
}

#[tauri::command]
pub fn get_auto_updates(window: tauri::WebviewWindow) -> Result<bool, String> {
    trusted(&window)?;
    Ok(auto_updates_enabled())
}

#[tauri::command]
pub fn set_auto_updates(window: tauri::WebviewWindow, enabled: bool) -> Result<(), String> {
    trusted(&window)?;
    save_auto_updates(enabled).map_err(|e| e.to_string())?;
    if enabled {
        let app = window.app_handle().clone();
        tauri::async_runtime::spawn(async move { check_once(&app).await });
    }
    Ok(())
}

#[tauri::command]
pub fn notify_upgrade_required(
    window: tauri::WebviewWindow,
    min_client_protocol: u32,
) -> Result<(), String> {
    trusted(&window)?;
    crate::tray::upgrade_required(window.app_handle(), min_client_protocol);
    let mut last = LAST_REQUIRED_PROTOCOL
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if *last == Some(min_client_protocol) {
        return Ok(());
    }
    *last = Some(min_client_protocol);
    window
        .app_handle()
        .notification()
        .builder()
        .title("Orbit update required")
        .body(format!(
            "This server requires sync protocol {min_client_protocol}. Queued changes remain on this device."
        ))
        .show()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_upgrade_required(window: tauri::WebviewWindow) -> Result<(), String> {
    trusted(&window)?;
    *LAST_REQUIRED_PROTOCOL
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = None;
    crate::tray::clear_upgrade_required(window.app_handle());
    Ok(())
}

/// Checks on startup and every six hours while automatic checks are enabled.
pub fn check_update(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if auto_updates_enabled() {
                check_once(&app).await;
            }
            tokio::time::sleep(UPDATE_INTERVAL).await;
        }
    });
}

async fn check_once(app: &AppHandle) {
    let result: Result<(), Box<dyn std::error::Error>> = async {
        let updater = updater(app)?;
        let Some(update) = updater.check().await? else {
            return Ok(());
        };
        let version = update.version.clone();
        let notes = update
            .body
            .clone()
            .unwrap_or_else(|| "A new Orbit version is ready.".into());
        update.download_and_install(|_, _| {}, || {}).await?;
        crate::tray::update_ready(app, &version);
        app.notification()
            .builder()
            .title(format!("Orbit {version} is ready"))
            .body(format!(
                "{notes}\nRestart Orbit from the tray menu to apply."
            ))
            .show()?;
        let _ = app.emit("orbit://update-ready", (version, notes));
        Ok(())
    }
    .await;
    if let Err(e) = result {
        eprintln!("orbit: update check failed: {e}");
    }
}

fn updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, Box<dyn std::error::Error>> {
    use tauri_plugin_updater::UpdaterExt;

    let mut builder = app.updater_builder();
    // Disposable local updater feeds are only accepted by debug builds. Release builds always
    // use the pinned, signed endpoint and public key from tauri.conf.json.
    if cfg!(debug_assertions) {
        if let Ok(endpoint) = std::env::var("ORBIT_UPDATE_ENDPOINT") {
            builder = builder.endpoints(vec![endpoint.parse()?])?;
        }
        if let Ok(pubkey) = std::env::var("ORBIT_UPDATE_PUBKEY") {
            builder = builder.pubkey(pubkey);
        }
    }
    Ok(builder.build()?)
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

    #[test]
    fn launcher_documents() {
        for ok in ["/", "", "/index.html"] {
            let url = format!("tauri://localhost{ok}").parse().unwrap();
            assert!(is_launcher(&url), "{ok:?} is the bundled UI");
        }
        // The server page is a destination, not a launcher: redirecting from it would loop.
        assert!(!is_launcher(
            &"tauri://localhost/server.html?down=http://x"
                .parse()
                .unwrap()
        ));
        assert!(!is_launcher(&"https://orbit.example.com/".parse().unwrap()));
    }

    #[test]
    fn auto_update_preference_defaults_on_and_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(AUTO_UPDATES);
        assert!(auto_updates_at(&path));
        std::fs::write(&path, "false").unwrap();
        assert!(!auto_updates_at(&path));
        std::fs::write(&path, "true").unwrap();
        assert!(auto_updates_at(&path));
    }
}
