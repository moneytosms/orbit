fn main() {
    // Read the identifier here so `run()` can forward `--capture` before `generate_context!()` runs.
    let config_path =
        std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("tauri.conf.json");
    println!("cargo:rerun-if-changed={}", config_path.display());
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    println!(
        "cargo:rustc-env=ORBIT_APP_IDENTIFIER={}",
        config["identifier"]
            .as_str()
            .expect("tauri.conf.json identifier")
    );

    // Listed so capabilities can grant them to the remote server origin (Android loads the UI from the server).
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_token",
            "clear_token",
            "has_token",
            "set_server",
            "get_auto_updates",
            "set_auto_updates",
            "notify_upgrade_required",
            "clear_upgrade_required",
            "take_link",
            "shell_version",
            "set_bar_theme",
            "capture_task",
            "get_capture_shortcut",
            "set_capture_shortcut",
            "hide_capture",
        ]),
    ))
    .unwrap();
}
