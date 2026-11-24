# Desktop app

For people running Orbit on a Linux desktop, with a tray icon, a quick capture hotkey and reminders while the window is closed. `orbit-app` is the Tauri 2 build of the same UI as the web app.

## Before you start

- A running Orbit server reachable over HTTPS. See [Deploy](deploy.md).
- `webkit2gtk-4.1` and `libayatana-appindicator` on the system. On Arch: `sudo pacman -S webkit2gtk-4.1 libayatana-appindicator`.
- A notification daemon and a system tray host (on GNOME, the AppIndicator extension).

## Steps

### Install

**AppImage (GitHub Releases):**

```sh
# Download Orbit_<version>_amd64.AppImage from https://github.com/moneytosms/orbit/releases
chmod +x Orbit_<version>_amd64.AppImage
./Orbit_<version>_amd64.AppImage
```

If the AppImage fails to start with a FUSE error, install your distribution's FUSE 2 package (on Arch, `fuse2`).

### First launch

1. The app shows a server-address page. Enter your HTTPS server URL (for example `https://orbit.example.com`). It is saved in `$XDG_DATA_HOME/orbit/server`, so later launches go straight to the server.
2. Sign in. The app then creates its own Device, `Orbit desktop (reminders)`, and stores its token in the Secret Service keyring (or `$XDG_DATA_HOME/orbit/token`, mode 0600, if no keyring runs).
3. Closing the window hides it to the tray. Tray menu: Open, Quick add, Sync now, Quit.

To change the server later, see [Server address](#server-address).

### Start at login

The app starts hidden with `--hidden`. Autostart through the app (`tauri-plugin-autostart`) uses that flag. On Hyprland, or any setup with its own autostart list, start it yourself with:

```sh
orbit-app --hidden
```

### Quick capture hotkey

`orbit-app --capture` opens the quick capture overlay. On X11, Orbit registers **Alt+T** by default. Change it under Settings > Quick capture; the binding is stored on this device. The plugin's Linux backend is X11-only, so on Wayland Orbit reports the limitation in Settings and shows a Hyprland binding for the selected shortcut. The overlay saves captures directly to the local replica, so it works offline and syncs later. Press Enter to add, Ctrl+Enter to add another, or Escape to close. The tray's Quick add action opens the same overlay.

#### Hyprland

If global shortcut registration is unavailable on your Hyprland session, add this alternative in `~/.config/hypr/bindings.lua`:

```lua
hl.window_rule({
  match = { initial_title = "^Quick capture$" },
  float = true,
  center = true,
})

o.bind("ALT + T", "Orbit quick capture", "orbit-app --capture")
```

The float/center rule is needed because Hyprland may tile the overlay despite its requested size. Use the binding shown in Settings if you changed Alt+T.

### Updates

The AppImage checks `https://github.com/moneytosms/orbit/releases/latest/download/latest.json` at start. A newer, signature-checked AppImage is downloaded and installed, and a notification asks you to restart. It never restarts by itself; failures are only logged to stderr. The `.deb` is not updated this way.

Releases are signed with a minisign key kept outside the repo (`~/.config/orbit/updater.key`; the public key is in `tauri.conf.json`). CI reads it from the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets and publishes `latest.json` and the `.sig` with the AppImage.

### Build from source

```sh
just wasm                                  # builds orbit-core for the browser into app/wasm
pnpm -C app install --frozen-lockfile
cargo tauri build --bundles appimage       # output: target/release/bundle/appimage/
```

Build dependencies on Debian or Ubuntu: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`. `cargo tauri` is the Tauri CLI; CI installs it with `taiki-e/install-action` (see `.github/workflows/release.yml`). `app/package.json` has no `tauri` script, so run it from the repo root as shown.

## Verify it worked

Press the capture shortcut (or run `orbit-app --capture`), type `test capture`, and press Enter. Open the main window from the tray: the task is in the Inbox. Run `orbit sync && orbit ls` on another device signed in to the same server to see it arrive.

## How it works

- The window keeps its own IndexedDB replica, like the web app.
- The Rust side owns the tray (libayatana-appindicator), single-instance, autostart, and a small background replica (`orbit-client`, SQLite at `$XDG_DATA_HOME/orbit/replica.db`) that syncs every 60 s and shows reminders as desktop notifications. The web notification path is off inside the app so nothing fires twice.
- After you sign in, the app creates its own Device (`Orbit desktop (reminders)`, visible and revocable in Settings) and stores its token in the Secret Service keyring. If no keyring is running, it falls back to `$XDG_DATA_HOME/orbit/token` with mode 0600.

### Flags

| Flag | Effect |
| --- | --- |
| `--hidden` | Start without showing the window (used by autostart). |
| `--capture` | Open the independent quick-capture overlay. If Orbit is already running, the running instance does it. |
| `--server <url>` | Save the server address and open it (see Server address). |

### Server address

The packaged app bundles only a launcher. On first start it shows a server-address page; enter your Orbit server URL (`http://` or `https://`, e.g. `https://orbit.example.com`) and the window loads the UI your server hosts (`ORBIT_WEB`). Sign in there as usual. The URL is saved in `$XDG_DATA_HOME/orbit/server`, next to `replica.db`, so later launches go straight to the server. If the saved server does not answer, the address page comes back with a Retry button.

The server-hosted page may call only the token commands, the notification plugin and the tray/capture events. The background replica and reminders sync with the saved server, whatever origin the window is on.

To change the server later, run `orbit-app --server https://other.example.com`. It saves the address and, if Orbit is already running, opens it in that window. Switching to a different server also drops the stored Device token; sign in again to create a new one. (Or delete `$XDG_DATA_HOME/orbit/server` and restart to get the address page.) In development, `devUrl` is used as before.

## Troubleshooting

**No notifications or tray icon.** Check that a notification daemon and a system tray host are running. The tray uses libayatana-appindicator; on GNOME it needs the AppIndicator extension.

**The window cannot reach the server.** The address page returns with a Retry button. Check the server is up, then retry. The URL should start with `https://`; see [Install, Troubleshooting](install.md#troubleshooting).

**The shortcut does nothing on Wayland.** The global shortcut backend is X11-only. Use the Hyprland binding above, or your compositor's own keybinding for `orbit-app --capture`.

## Next

[Android app](android.md), or the [CLI](cli-install.md) for scripting.
