# Linux tray, autostart and global hotkey in Tauri 2

Issue: moneytosms/orbit#23. Date: 2026-11-19.
Scope: what Tauri 2 (core + official tray, autostart, global-shortcut plugins) supports on Linux for a tray icon, autostart, and a global hotkey that opens a capture window. Wayland (Hyprland, GNOME, KDE) vs X11. Portal requirements, SNI/libappindicator, open issues, fallback.
Method: primary sources only. Tauri and plugin source read at tags `tauri-v2.12.7`, `global-shortcut-v2.4.1`, `autostart-v2.7.0` (via `gh api`). `tray-icon` 0.25.0 and `global-hotkey` 0.8.0 read from crates.io tarballs. Portal spec at flatpak.github.io. Backend portal configs at github.com/flatpak/xdg-desktop-portal-kde, github.com/hyprwm/xdg-desktop-portal-hyprland, gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome. Issues via `gh search`/`gh issue view`. Nothing built or run on any compositor.

## Verdict

**Tray and autostart work on Linux. The global hotkey does not work on native Wayland with Tauri's plugin.** Tauri's global-shortcut plugin is X11-only and fails silently on Wayland. Ship the hotkey as a compositor keybind running `orbit --capture`, forwarded to the running instance via `tauri-plugin-single-instance`. Revisit the XDG GlobalShortcuts portal (via `ashpd`) later for KDE/GNOME/Hyprland.

## Findings

### Global hotkey (tauri-plugin-global-shortcut)

- Plugin README lists Linux as supported with no Wayland caveat. Source: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/global-shortcut/README.md (read at `global-shortcut-v2.4.1`).
- The plugin depends on `global-hotkey = "0.8"`, not on any portal crate (no `ashpd`, no `zbus`). Source: `plugins/global-shortcut/Cargo.toml` at `global-shortcut-v2.4.1`.
- `global-hotkey` 0.8.0 README and `src/lib.rs` say: "Linux (X11 Only)". Source: https://github.com/tauri-apps/global-hotkey (`src/lib.rs` header, 0.8.0 crate tarball).
- Backend connects with `RustConnection::connect(None)` (uses `DISPLAY`) and runs `XGrabKey` on the root window. Source: `src/platform_impl/x11/mod.rs` (0.8.0).
- **Silent failure on pure Wayland (verified by reading source, not run):**
  - `GlobalHotKeyManager::new()` always returns `Ok`. It spawns a thread that does the X connect.
  - If the X connect fails, the thread only logs under the `tracing` feature, then exits.
  - `register()` sends to the dead thread, ignores the send error, and `if let Ok(result) = rx.recv()` falls through to `Ok(())`.
  - So `app.global_shortcut().register(...)` returns success and nothing is bound. The plugin's `setup` `?` does not catch it.
  - Source: `src/platform_impl/x11/mod.rs` lines 31-56 and 227-232 (0.8.0).
  - Caveat: `global-hotkey` 0.10.0 (published 2026-11-09) exists. **Unverified**: whether it changes this behaviour. Not read.
- Under XWayland (`DISPLAY` set), grabs may succeed but only fire while an X11 client is focused. **Unverified**: not tested on Hyprland, GNOME or KDE.
- Plugin docs show `CmdOrCtrl`-style accelerators. Source: https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/plugin/global-shortcut.mdx (`v2`).

### XDG GlobalShortcuts portal (the Wayland path)

- Spec: `org.freedesktop.portal.GlobalShortcuts`, interface version 2. Methods `CreateSession`, `BindShortcuts`, `ListShortcuts`. Signals `Activated` and `Deactivated`. Shortcuts are bound per session and are active regardless of app focus. Source: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html
- The app must create a session and call `BindShortcuts`. The compositor/desktop shows the binding UI. Source: same spec.
- Backends that implement `org.freedesktop.impl.portal.GlobalShortcuts`:
  - **KDE Plasma**: yes. `data/kde.portal` in xdg-desktop-portal-kde lists it. Source: https://github.com/KDE/xdg-desktop-portal-kde (`data/kde.portal`, `master`).
  - **Hyprland**: yes. `hyprland.portal` lists it, with `UseIn=wlroots;Hyprland;sway;Wayfire;river`. Source: https://github.com/hyprwm/xdg-desktop-portal-hyprland (`hyprland.portal`, `master`).
  - **GNOME**: yes. `data/meson.build` in xdg-desktop-portal-gnome lists `org.freedesktop.impl.portal.GlobalShortcuts`. `src/globalshortcuts.c` calls `org.gnome.Shell` `GrabAccelerator`/`UngrabAccelerators`. Source: https://gitlab.gnome.org/GNOME/xdg-desktop-portal-gnome (`data/meson.build`, `src/globalshortcuts.c`, `main`). Minimum GNOME/portal version: **unverified**.
- Tauri does not use the portal. Using it means custom Rust code. `ashpd` 0.14.0 exposes `desktop::global_shortcuts` (`GlobalShortcuts::bind_shortcuts`). Source: crates.io tarball `ashpd-0.14.0`. Not yet built against Tauri.
- Portal requirement for the app: `xdg-desktop-portal` (1.22.1 is the latest release per GitHub API) plus the matching backend running. **Unverified**: whether a non-sandboxed (non-Flatpak) app can use the portal as-is. Spec does not say.

### Tray icon (SNI / libappindicator)

- Tauri 2.12.7 enables `tray-icon` 0.25 with features `serde` and `libappindicator`. Source: `crates/tauri/Cargo.toml` at `tauri-v2.12.7`.
- `tray-icon` 0.25.0 default features are `libappindicator` (and `muda-libxdo`). The `ksni` backend is an optional feature that Tauri does not enable. Source: `Cargo.toml` of tray-icon 0.25.0 (crates.io tarball).
- `libappindicator` needs a system library at build and run time: `libappindicator-gtk3` or `libayatana-appindicator` (Arch), `libappindicator3-dev` or `libayatana-appindicator3-dev` (Debian/Ubuntu), plus `gtk3` and `xdotool`. Source: tray-icon 0.25.0 `src/lib.rs` docs.
- The Ayatana project describes itself as "a GTK implementation of the StatusNotifierItem Specification (SNI)". Source: https://github.com/AyatanaIndicators/libayatana-appindicator (README).
- The SNI spec defines the DBus protocol: `org.freedesktop.StatusNotifierItem-PID-ID` service, `StatusNotifierWatcher`, and `ContextMenu()` instead of `Activate()` for right-click. Source: https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierItem/
- Hosts:
  - **GNOME Shell**: no built-in tray. Needs the AppIndicator/KStatusNotifierItem extension (`appindicatorsupport@rgcjonas.gmail.com`, extensions.gnome.org id 615). Source: https://github.com/ubuntu/gnome-shell-extension-appindicator (README). Under Wayland, a logout/login is needed after install (same README).
  - **KDE Plasma**: native SNI host. **Unverified**: not checked against a Plasma source file. Common knowledge, flag before relying on it.
  - **Hyprland**: no built-in tray. Use a bar with a tray module (e.g. Waybar's `tray` module). Source: https://github.com/Alexays/Waybar (`man/waybar-tray.5.scd`, marked beta). SNI support in Waybar's tray: **unverified** (man page does not say).
- Tauri tray limits, from the Tauri docs and `tray-icon` source:
  - Left-click, double-click, enter, move, leave events are **not emitted on Linux**. Right-click shows the menu. Source: https://github.com/tauri-apps/tauri-docs/blob/v2/src/content/docs/learn/system-tray.mdx (`v2`) and `crates/tauri/src/tray/mod.rs` (tauri-v2.12.7).
  - Tooltip: unsupported on Linux. Source: `crates/tauri/src/tray/mod.rs` `set_tooltip` docs (tauri-v2.12.7).
  - Icon may not appear unless a menu is set. Menu cannot be removed once set. Source: `crates/tauri/src/tray/mod.rs` (tauri-v2.12.7).
  - The icon needs a temp dir on Linux (written to disk). Source: `crates/tauri/src/tray/mod.rs` (tauri-v2.12.7).
  - Implication for Orbit: the capture UI cannot be opened by clicking the tray icon on Linux. Use a menu item, or rely on the hotkey/CLI.

### Autostart (tauri-plugin-autostart)

- Plugin README: Linux supported. Source: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/autostart/README.md (`autostart-v2.7.0`).
- Plugin uses `auto-launch` 0.6 and the default `LinuxLaunchMode::XdgAutostart`. Writes `~/.config/autostart/<app_name>.desktop`. Source: `plugins/autostart/Cargo.toml` and `plugins/autostart/src/lib.rs` (`autostart-v2.7.0`); `auto-launch` 0.6.0 `src/linux.rs` and `src/lib.rs` (crates.io tarball).
- Under AppImage, the plugin uses the AppImage path. Source: `plugins/autostart/src/lib.rs` (`autostart-v2.7.0`).
- XDG Autostart spec: `.desktop` files in `$XDG_CONFIG_HOME/autostart` are launched by the session manager. Source: https://specifications.freedesktop.org/autostart-spec/latest/
- **Unverified**: whether Hyprland runs XDG autostart by default. The Hyprland wiki autostart page did not render. Assume it does not without a systemd user target or `exec-once`. Check on the machine before relying on it.

### Single-instance forwarding (for the CLI fallback)

- `tauri-plugin-single-instance` README: Linux supported. The callback receives `argv` and `cwd` of the second launch. Source: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/single-instance/README.md (`v2`).
- Deep-link plugin README: on Windows and Linux the OS spawns a new instance with the URL as CLI argument, unless `single-instance` with the `deep-link` feature is used. Source: https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/deep-link/README.md (`v2`).

### Open upstream issues

- tauri-apps/global-hotkey #28 "Global Shortcut support on Wayland": open. Root tracker. https://github.com/tauri-apps/global-hotkey/issues/28
- tauri-apps/global-hotkey #219 "Wayland support (using Evdev)": closed as duplicate (target not checked). Proposed evdev approach was not adopted here. https://github.com/tauri-apps/global-hotkey/issues/219
- tauri-apps/tauri #3578 "[feat] Global Shortcut support on Wayland": open. Blocked on `wayland-protocols` RFC (MR 56). https://github.com/tauri-apps/tauri/issues/3578
- tauri-apps/tauri #11293 "[feat] Use ksni crate for tray icons on Linux": open. Says libappindicator is near-dead and lacks left-click and tooltip. https://github.com/tauri-apps/tauri/issues/11293
- tauri-apps/global-hotkey #198 "Expose Linux/X11 activation timestamp": open. Not blocking. https://github.com/tauri-apps/global-hotkey/issues/198
- tauri-apps/tauri #13599 (tray missing under Flatpak): closed. Relevant only if Orbit ships as Flatpak. https://github.com/tauri-apps/tauri/issues/13599
- No open Tauri issue found for tray/autostart failures specific to Wayland or GNOME (searched `wayland tray`, `gnome tray icon`, `tray not visible kde`, `wayland autostart`, `appindicator`). Searches cover only the issues API; open PR state not checked.

## Fallback: compositor keybind to `orbit --capture`

- Recommended path for v1 on Wayland:
  1. Tray via Tauri `TrayIconBuilder` with a menu (Capture, Quit). Left-click is not available on Linux.
  2. Single instance via `tauri-plugin-single-instance`. A second launch of `orbit --capture` sends `argv` to the running instance, which opens the capture window.
  3. User binds the hotkey in their compositor to `orbit --capture` (Hyprland `exec` bind, GNOME/KDE custom shortcut). **Unverified**: exact config syntax for all three, not checked against their docs.
  4. Autostart via `tauri-plugin-autostart` (XDG `.desktop`). Confirm it launches on Hyprland (see Unverified).
- Optional later: `ashpd` GlobalShortcuts portal for KDE, GNOME and Hyprland. Gives the user a native binding UI without editing compositor config. Not yet built or tested.
- Keep `register()` out of the Wayland path. Since `register()` returns `Ok` on Wayland, do not use its success as proof the hotkey is bound. Detect the session with `XDG_SESSION_TYPE` / `WAYLAND_DISPLAY` if the in-app hotkey is ever enabled.

## Unverified (summary)

- `global-hotkey` 0.10.0 behaviour on Wayland.
- Behaviour of XWayland grabs under each compositor (fires only when X11 window focused: expected, not tested).
- GNOME and KDE: exact portal version minimums, and KDE's native SNI host (not checked in a Plasma source file).
- Waybar tray SNI support.
- Hyprland XDG autostart behaviour (wiki page did not render).
- Portal use from a non-sandboxed app; no `ashpd` prototype built.
- GNOME/KDE shortcut config commands for the CLI fallback.
- Nothing built or run. No Tauri code exists in this repo yet (`src-tauri/` absent on `main`).

Next: decide v1 hotkey = compositor keybind + single-instance CLI. Then spike the tray and single-instance on GNOME (extension installed) and Hyprland (Waybar) with `rtk cargo` once a Tauri shell exists.
