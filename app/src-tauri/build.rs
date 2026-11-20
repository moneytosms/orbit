fn main() {
    // Listed so capabilities can grant them to the remote server origin (Android loads the UI from the server).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_token",
            "clear_token",
            "has_token",
            "set_server",
            "take_link",
        ]),
    ))
    .unwrap();
}
