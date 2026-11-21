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
}
