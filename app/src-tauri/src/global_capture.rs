//! Desktop-only global shortcut and its offline capture window.
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

const DEFAULT_SHORTCUT: &str = "ALT+T";
const SHORTCUT_FILE: &str = "capture-shortcut";
pub const SHOW_EVENT: &str = "orbit://show-capture";
static SHORTCUT: Mutex<String> = Mutex::new(String::new());
static ERROR: Mutex<Option<String>> = Mutex::new(None);
static REGISTERED: AtomicBool = AtomicBool::new(false);

fn file() -> PathBuf {
    crate::token::data_dir().join(SHORTCUT_FILE)
}

pub fn shortcut() -> String {
    SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn set_error(error: Option<String>) {
    *ERROR.lock().unwrap_or_else(|e| e.into_inner()) = error;
}

pub fn error() -> Option<String> {
    ERROR.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn initialize(app: &AppHandle) {
    let requested = std::fs::read_to_string(file())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SHORTCUT.into());
    if wayland_session() {
        *SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()) = requested.clone();
        REGISTERED.store(false, Ordering::Relaxed);
        set_error(Some(wayland_message(&requested)));
        refresh_tray(app);
        return;
    }
    match app.global_shortcut().register(requested.as_str()) {
        Ok(()) => {
            REGISTERED.store(true, Ordering::Relaxed);
            *SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()) = requested;
            set_error(None);
        }
        Err(e) => {
            REGISTERED.store(false, Ordering::Relaxed);
            *SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()) = requested;
            set_error(Some(fallback_message(&e.to_string())));
        }
    }
    refresh_tray(app);
}

fn fallback_message(cause: &str) -> String {
    format!(
        "Global shortcut unavailable ({cause}). Choose another binding or use `orbit-app --capture`."
    )
}

fn wayland_session() -> bool {
    std::env::var("XDG_SESSION_TYPE").is_ok_and(|kind| kind == "wayland")
        || std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn wayland_message(shortcut: &str) -> String {
    let binding = shortcut
        .split('+')
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" + ");
    format!(
        "Native Wayland global shortcuts are unavailable with this X11 backend. On Hyprland, add `o.bind(\"{binding}\", \"Orbit quick capture\", \"orbit-app --capture\")` to your bindings."
    )
}

fn refresh_tray(app: &AppHandle) {
    crate::tray::refresh_capture_label(app, &shortcut());
}

#[tauri::command]
pub fn get_capture_shortcut(
    window: tauri::WebviewWindow,
) -> Result<(String, Option<String>), String> {
    crate::desktop::trusted(&window)?;
    Ok((shortcut(), error()))
}

#[tauri::command]
pub fn set_capture_shortcut(window: tauri::WebviewWindow, shortcut: String) -> Result<(), String> {
    crate::desktop::trusted(&window)?;
    let shortcut = shortcut.trim();
    if shortcut.is_empty() {
        return Err("Choose a key combination".into());
    }
    let app = window.app_handle();
    let old = self::shortcut();
    if wayland_session() {
        std::fs::create_dir_all(crate::token::data_dir()).map_err(|e| e.to_string())?;
        std::fs::write(file(), shortcut).map_err(|e| e.to_string())?;
        if REGISTERED.swap(false, Ordering::Relaxed) {
            let _ = app.global_shortcut().unregister(old.as_str());
        }
        *SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()) = shortcut.to_string();
        set_error(Some(wayland_message(shortcut)));
        refresh_tray(app);
        return Ok(());
    }
    let was_registered = REGISTERED.load(Ordering::Relaxed);
    if old == shortcut && was_registered {
        set_error(None);
        return Ok(());
    }
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| fallback_message(&e.to_string()))?;
    std::fs::create_dir_all(crate::token::data_dir()).map_err(|e| {
        let _ = app.global_shortcut().unregister(shortcut);
        e.to_string()
    })?;
    std::fs::write(file(), shortcut).map_err(|e| {
        let _ = app.global_shortcut().unregister(shortcut);
        e.to_string()
    })?;
    if was_registered && old != shortcut {
        let _ = app.global_shortcut().unregister(old.as_str());
    }
    REGISTERED.store(true, Ordering::Relaxed);
    *SHORTCUT.lock().unwrap_or_else(|e| e.into_inner()) = shortcut.to_string();
    set_error(None);
    refresh_tray(app);
    Ok(())
}

#[tauri::command]
pub fn capture_task(window: tauri::WebviewWindow, text: String) -> Result<(), String> {
    crate::desktop::trusted(&window)?;
    crate::replica::capture(&crate::token::data_dir().to_string_lossy(), &text)
        .map(drop)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn hide_capture(window: tauri::WebviewWindow) -> Result<(), String> {
    crate::desktop::trusted(&window)?;
    if window.label() != "capture" {
        return Err("capture window required".into());
    }
    window.hide().map_err(|e| e.to_string())
}

pub fn show(app: &AppHandle) {
    let Some(window) = app.get_webview_window("capture") else {
        return;
    };
    if let Ok(cursor) = app.cursor_position()
        && let Ok(Some(monitor)) = app.monitor_from_point(cursor.x, cursor.y)
    {
        let position = monitor.position();
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let width = (640.0 * scale) as u32;
        let height = (180.0 * scale) as u32;
        let x = position.x + (size.width.saturating_sub(width) / 2) as i32;
        let y = position.y + (size.height.saturating_sub(height) / 2) as i32;
        let _ = window.set_position(PhysicalPosition::new(x, y));
    } else {
        let _ = window.center();
    }
    let _ = window.show();
    let _ = window.set_focus();
    let _ = window.emit(SHOW_EVENT, ());
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("capture") {
        let _ = window.hide();
    }
}

pub fn on_shortcut(app: &AppHandle) {
    show(app);
}
