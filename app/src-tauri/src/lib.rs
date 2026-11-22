#[cfg(target_os = "android")]
mod android;
#[cfg(desktop)]
mod desktop;
#[cfg(desktop)]
mod global_capture;
#[cfg(desktop)]
mod reminders;
mod replica;
#[cfg(desktop)]
mod token;
#[cfg(desktop)]
mod tray;

#[cfg(desktop)]
use tauri::{Manager, WindowEvent};

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
    if !has_flag(&args, "--hidden") && !has_flag(&args, "--capture") {
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
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        global_capture::on_shortcut(app);
                    }
                })
                .build(),
        )
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
            desktop::set_server,
            desktop::get_auto_updates,
            desktop::set_auto_updates,
            desktop::notify_upgrade_required,
            desktop::clear_upgrade_required,
            global_capture::capture_task,
            global_capture::get_capture_shortcut,
            global_capture::set_capture_shortcut,
            global_capture::hide_capture,
            desktop::shell_version
        ])
        .on_page_load(|webview, payload| {
            desktop::on_page_load(webview, payload.url(), payload.event());
        })
        .setup(|app| {
            global_capture::initialize(app.handle());
            setup_desktop(app)
        })
        .on_window_event(|window, event| {
            // Closing hides to tray; Quit in the tray menu exits.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
            if let WindowEvent::Focused(false) = event
                && window.label() == "capture"
            {
                let app = window.app_handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if app
                        .get_webview_window("capture")
                        .is_some_and(|window| window.is_focused().is_ok_and(|focused| !focused))
                    {
                        global_capture::hide(&app);
                    }
                });
            }
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "capture"
            {
                api.prevent_close();
                global_capture::hide(window.app_handle());
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
