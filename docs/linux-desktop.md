# Linux desktop app

`orbit-app` is the Tauri 2 build of the Svelte UI for Linux. It lives in the tray, starts hidden at login and fires reminder notifications even when the window is closed.

## How it works

- The window keeps its own IndexedDB replica, like the web app.
- The Rust side owns the tray (libayatana-appindicator), single-instance, autostart, and a small background replica (`orbit-client`, SQLite at `$XDG_DATA_HOME/orbit/replica.db`) that syncs every 60 s and shows reminders as desktop notifications. The web notification path is off inside the app so nothing fires twice.
- After you sign in, the app creates its own Device (`Orbit desktop (reminders)`, visible and revocable in Settings) and stores its token in the Secret Service keyring. If no keyring is running, it falls back to `$XDG_DATA_HOME/orbit/token` with mode 0600.
- Closing the window hides it to the tray. Tray menu: Open, Quick add, Sync now, Quit. Left click opens the window.

## Flags

| Flag | Effect |
| --- | --- |
| `--hidden` | Start without showing the window (used by autostart). |
| `--capture` | Show the window and open quick capture. If Orbit is already running, the running instance does it. |
| `--server <url>` | Save the server address and open it (see Server address). |

## Hyprland

In `~/.config/hypr/bindings.lua`:

```lua
o.bind("SUPER + ALT + A", "Orbit quick capture", "orbit-app --capture")
```

Start it hidden at login with the same file or your autostart list:

```lua
orbit-app --hidden
```

Autostart through the app itself (`tauri-plugin-autostart`) also launches it with `--hidden`.

## Server address

The packaged app bundles only a launcher. On first start it shows a server-address page; enter your Orbit server URL (`http://` or `https://`, e.g. `https://orbit.example.com`) and the window loads the UI your server hosts (`ORBIT_WEB`). Sign in there as usual. The URL is saved in `$XDG_DATA_HOME/orbit/server`, next to `replica.db`, so later launches go straight to the server. If the saved server does not answer, the address page comes back with a Retry button.

The server-hosted page may call only the token commands, the notification plugin and the tray/capture events. The background replica and reminders sync with the saved server, whatever origin the window is on.

To change the server later, run `orbit-app --server https://other.example.com`. It saves the address and, if Orbit is already running, opens it in that window. Switching to a different server also drops the stored Device token; sign in again to create a new one. (Or delete `$XDG_DATA_HOME/orbit/server` and restart to get the address page.) In development, `devUrl` is used as before.

## Updates

The AppImage updates itself with `tauri-plugin-updater`. At start the app checks `https://github.com/moneytosms/orbit/releases/latest/download/latest.json`; a newer, signature-checked AppImage is downloaded and installed, and a notification says "Orbit updated, restart to apply". It never restarts on its own, and failures are only logged to stderr. The `.deb` is not updated this way.

Releases are signed with a minisign key kept outside the repo (`~/.config/orbit/updater.key`; the public key is in `tauri.conf.json`). CI reads it from the `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets and publishes `latest.json` and the `.sig` with the AppImage.
