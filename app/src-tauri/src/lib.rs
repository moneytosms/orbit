mod reminders;
mod token;
mod tray;

use tauri::WindowEvent;

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

#[tauri::command]
fn set_token(server: String, device: String, token: String) -> Result<(), String> {
    token::save(&token::Stored {
        server,
        device,
        token,
    })
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn clear_token() {
    token::clear();
}

#[tauri::command]
fn has_token() -> bool {
    token::has()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
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
        .invoke_handler(tauri::generate_handler![set_token, clear_token, has_token])
        .setup(|app| {
            let handle = app.handle().clone();
            tray::build(&handle)?;
            reminders::start(handle.clone());
            let args: Vec<String> = std::env::args().collect();
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
        })
        .on_window_event(|window, event| {
            // Closing hides to tray; Quit in the tray menu exits.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == "main"
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
