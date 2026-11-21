#[cfg(target_os = "android")]
mod android;
#[cfg(desktop)]
mod desktop;
#[cfg(desktop)]
mod reminders;
mod replica;
#[cfg(desktop)]
mod token;
#[cfg(desktop)]
mod tray;

#[cfg(desktop)]
use tauri::WindowEvent;

#[cfg(desktop)]
fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

#[cfg(desktop)]
#[tauri::command]
fn set_token(
    window: tauri::WebviewWindow,
    server: String,
    device: String,
    token: String,
) -> Result<(), String> {
    desktop::trusted(&window)?;
    // The background replica talks to the saved server, not the origin the window happens to be on.
    let server = desktop::saved().unwrap_or(server);
    token::save(&token::Stored {
        server,
        device,
        token,
    })
    .map_err(|e| e.to_string())
}

#[cfg(desktop)]
#[tauri::command]
fn clear_token(window: tauri::WebviewWindow) {
    if desktop::trusted(&window).is_ok() {
        token::clear();
    }
}

#[cfg(desktop)]
#[tauri::command]
fn has_token(window: tauri::WebviewWindow) -> bool {
    desktop::trusted(&window).is_ok() && token::has()
}

#[cfg(desktop)]
fn setup_desktop(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    tray::build(&handle)?;
    let args: Vec<String> = std::env::args().collect();
    // `--server` already chose a destination; otherwise the window still holds the bundled UI.
    let flagged = desktop::apply_flag(&handle, &args);
    reminders::start(handle.clone());
    desktop::check_update(handle.clone());
    if !flagged {
        desktop::launch(&handle);
    }
    if !has_flag(&args, "--hidden") {
        tray::show_main(&handle);
    }
    // The first `--capture` launch also needs the page to be listening, so give it a moment.
    if has_flag(&args, "--capture") {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(2));
            tray::capture(&handle);
        });
    }
    Ok(())
}

#[cfg(desktop)]
fn builder() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            desktop::apply_flag(app, &args);
            if has_flag(&args, "--capture") {
                tray::capture(app);
            } else if !has_flag(&args, "--hidden") {
                tray::show_main(app);
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            set_token,
            clear_token,
            has_token,
            desktop::set_server
        ])
        .on_page_load(|webview, payload| {
            desktop::on_page_load(webview, payload.url(), payload.event());
        })
        .setup(|app| setup_desktop(app))
        .on_window_event(|window, event| {
            // Closing hides to tray; Quit in the tray menu exits.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
}

#[cfg(target_os = "android")]
fn builder() -> tauri::Builder<tauri::Wry> {
    android::builder()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    builder()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
