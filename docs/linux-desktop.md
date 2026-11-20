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

The Rust side talks to the origin the window was loaded from, so run the app against your Orbit server (`devUrl` in development).
