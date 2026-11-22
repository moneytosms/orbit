//! The tray icon (libayatana-appindicator) and the window actions it and `--capture` share.
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager};

pub const CAPTURE_EVENT: &str = "orbit://capture";
pub const SYNC_EVENT: &str = "orbit://sync";
const TRAY_ID: &str = "orbit";
static UPDATE_STATE: std::sync::Mutex<(Option<String>, Option<u32>)> =
    std::sync::Mutex::new((None, None));

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Shows the window and asks the UI to open quick capture.
pub fn capture(app: &AppHandle) {
    show_main(app);
    let _ = app.emit(CAPTURE_EVENT, ());
}

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu(app, None, None)?;
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Orbit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id.as_ref() {
            "open" => show_main(app),
            "quick_add" => capture(app),
            "sync" => {
                let _ = app.emit(SYNC_EVENT, ());
            }
            "restart_update" => app.restart(),
            "update_required" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let tauri::tray::TrayIconEvent::Click {
                button: tauri::tray::MouseButton::Left,
                button_state: tauri::tray::MouseButtonState::Up,
                ..
            } = e
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Replaces the tray action with a restart prompt after a signed update is staged.
pub fn update_ready(app: &AppHandle, version: &str) {
    UPDATE_STATE.lock().unwrap_or_else(|e| e.into_inner()).0 = Some(version.to_string());
    refresh(app);
}

pub fn upgrade_required(app: &AppHandle, protocol: u32) {
    UPDATE_STATE.lock().unwrap_or_else(|e| e.into_inner()).1 = Some(protocol);
    refresh(app);
}

pub fn clear_upgrade_required(app: &AppHandle) {
    UPDATE_STATE.lock().unwrap_or_else(|e| e.into_inner()).1 = None;
    refresh(app);
}

fn refresh(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let state = UPDATE_STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Ok(menu) = menu(app, state.0.as_deref(), state.1) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn menu(
    app: &AppHandle,
    update: Option<&str>,
    upgrade: Option<u32>,
) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Open", true, None::<&str>)?;
    let add = MenuItem::with_id(app, "quick_add", "Quick add", true, None::<&str>)?;
    let sync = MenuItem::with_id(app, "sync", "Sync now", true, None::<&str>)?;
    let restart = update
        .map(|version| {
            MenuItem::with_id(
                app,
                "restart_update",
                format!("Restart to update {version}"),
                true,
                None::<&str>,
            )
        })
        .transpose()?;
    let update_required = upgrade
        .map(|protocol| {
            MenuItem::with_id(
                app,
                "update_required",
                format!("Update Orbit (protocol {protocol} required)"),
                true,
                None::<&str>,
            )
        })
        .transpose()?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    match (restart, update_required) {
        (Some(restart), Some(required)) => {
            Menu::with_items(app, &[&open, &add, &sync, &restart, &required, &quit])
        }
        (Some(restart), None) => Menu::with_items(app, &[&open, &add, &sync, &restart, &quit]),
        (None, Some(required)) => Menu::with_items(app, &[&open, &add, &sync, &required, &quit]),
        (None, None) => Menu::with_items(app, &[&open, &add, &sync, &quit]),
    }
}
