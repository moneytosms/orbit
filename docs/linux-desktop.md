# Linux desktop app

`orbit-app` is the Tauri 2 build of the Svelte UI for Linux. It lives in the tray, starts hidden at login and fires reminder notifications even when the window is closed.

## How it works

- The window keeps its own IndexedDB replica, like the web app.
- The Rust side owns the tray (libayatana-appindicator), single-instance, autostart, and a small background replica (`orbit-client`, SQLite at `$XDG_DATA_HOME/orbit/replica.db`) that syncs every 60 s and shows reminders as desktop notifications. The web notification path is off inside the app so nothing fires twice.
- After you sign in, the app creates its own Device (`Orbit desktop (reminders)`, visible and revocable in Settings) and stores its token in the Secret Service keyring. If no keyring is running, it falls back to `$XDG_DATA_HOME/orbit/token` with mode 0600.
- Closing the main window hides it to the tray. Tray menu: Open, Quick add (with its shortcut), Sync now, Quit. Left click opens the main window.

## Flags

| Flag | Effect |
| --- | --- |
| `--hidden` | Start without showing the window (used by autostart). |
| `--capture` | Open the independent quick-capture overlay. If Orbit is already running, the running instance does it. |
| `--server <url>` | Save the server address and open it (see Server address). |

## Global shortcut

On X11, Orbit registers **Alt+T** by default. Change it under Settings → Quick capture; the binding is stored on this device. The plugin's Linux backend is X11-only, so on Wayland Orbit reports the limitation in Settings and shows a Hyprland binding for the selected shortcut. The overlay saves captures directly to the local replica, so it works offline and syncs later. Press Enter to add, Ctrl+Enter to add another, or Escape to close. `orbit-app --capture` and the tray's Quick add action open the same overlay.

## Hyprland

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

Start it hidden at login as described in [Install, Start at login](install.md#start-at-login).

## Server address

The packaged app bundles only a launcher. On first start it shows a server-address page; enter your Orbit server URL (`http://` or `https://`, e.g. `https://orbit.example.com`) and the window loads the UI your server hosts (`ORBIT_WEB`). Sign in there as usual. The URL is saved in `$XDG_DATA_HOME/orbit/server`, next to `replica.db`, so later launches go straight to the server. If the saved server does not answer, the address page comes back with a Retry button.

The server-hosted page may call only the token commands, the notification plugin and the tray/capture events. The background replica and reminders sync with the saved server, whatever origin the window is on.

To change the server later, run `orbit-app --server https://other.example.com`. It saves the address and, if Orbit is already running, opens it in that window. Switching to a different server also drops the stored Device token; sign in again to create a new one. (Or delete `$XDG_DATA_HOME/orbit/server` and restart to get the address page.) In development, `devUrl` is used as before.

## Updates

What users see when an update arrives is in [Install, Updates](install.md#updates). The AppImage uses `tauri-plugin-updater`; failures are only logged to stderr.

Releases are signed with a minisign key kept outside the repo (`~/.config/orbit/updater.key`; the public key is in `tauri.conf.json`). CI reads it from the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets and publishes `latest.json` and the `.sig` with the AppImage.
