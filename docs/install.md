# Install Orbit on your devices

Orbit has one server and several clients. Every client talks to the same server over HTTPS, and each install you sign in becomes a **Device** on your account.

## 1. Before you start

- You need a running Orbit server that your devices can reach over **HTTPS**. Set one up with [deploy.md](deploy.md) (Tailscale serve or Caddy with a domain). The address you end up with is the one every client below asks for: [deploy.md, Connect your clients](deploy.md#5-connect-your-clients).
- You have one account, created with the setup code the server prints on first run (see [deploy.md](deploy.md)).
- Each client you sign in becomes its own Device: web browser, Linux desktop app, Android app, CLI. Settings > Account and devices lists them and has a **Revoke** button per Device.

Clients:

| Client | Platforms | Section |
| --- | --- | --- |
| Web / PWA | any browser; installable on desktop and phone | [2](#2-web--pwa-any-desktop-or-phone-browser) |
| Linux desktop | AppImage, or AUR `orbit-bin` | [3](#3-linux-desktop) |
| Android | Android 12+ (minSdk 31), sideloaded APK | [4](#4-android) |
| CLI and MCP | Linux prebuilt; anywhere else from source | [5](#5-cli-and-mcp) |

## 2. Web / PWA (any desktop or phone browser)

1. Open your server's address (for example `https://orbit.example.com`) and log in. Use your password and, if enabled, the TOTP code.
2. Install it as an app:
   - **Chrome or Edge (desktop):** click the install icon in the address bar, or menu > **Install Orbit**.
   - **Android Chrome:** menu > **Add to Home screen** (or **Install app**).
   - **iOS Safari:** Share > **Add to Home Screen**.
3. Reminders in the browser: open Settings > Reminders and turn on **Notify on this device**, then allow notifications when the browser asks. Browser reminders only fire while an Orbit tab is open. For reminders on a phone, use the Android app.
4. Offline: the app shell, the wasm core and the fonts are precached by a service worker (`vite-plugin-pwa`, see `app/vite.config.ts`). Open the app once while online; after that it opens offline and queues changes until the connection returns.

## 3. Linux desktop

The desktop app is `orbit-app`: tray icon, quick capture hotkey, reminder notifications while the window is closed. Full details and flags are in [linux-desktop.md](linux-desktop.md).

### Install

**AppImage (GitHub Releases):**

```sh
# Download Orbit_<version>_amd64.AppImage from https://github.com/moneytosms/orbit/releases
chmod +x Orbit_<version>_amd64.AppImage
./Orbit_<version>_amd64.AppImage
```

The AppImage needs `webkit2gtk-4.1` and `libayatana-appindicator` on the system. On Arch: `sudo pacman -S webkit2gtk-4.1 libayatana-appindicator`. If the AppImage fails to start with a FUSE error, install your distribution's FUSE 2 package (on Arch, `fuse2`).

**AUR (`orbit-bin`)**, once published:

```sh
yay -S orbit-bin
```

The PKGBUILD is in `packaging/aur/`. It installs the same AppImage as `/usr/bin/orbit-app`.

### First launch

1. The app shows a server-address page. Enter your HTTPS server URL (for example `https://orbit.example.com`). It is saved in `$XDG_DATA_HOME/orbit/server`, so later launches go straight to the server.
2. Sign in. The app then creates its own Device, `Orbit desktop (reminders)`, and stores its token in the Secret Service keyring (or `$XDG_DATA_HOME/orbit/token`, mode 0600, if no keyring runs).
3. Closing the window hides it to the tray. Tray menu: Open, Quick add, Sync now, Quit.

To change the server later: `orbit-app --server https://other.example.com`. Switching servers drops the stored Device token, so sign in again.

### Start at login

The app starts hidden with `--hidden`. Autostart through the app (`tauri-plugin-autostart`) uses that flag. On Hyprland, or any setup with its own autostart list, start it yourself with:

```sh
orbit-app --hidden
```

### Quick capture hotkey

`orbit-app --capture` shows the window and opens quick capture. If Orbit is already running, the running instance does it. Hyprland example from [linux-desktop.md](linux-desktop.md), in `~/.config/hypr/bindings.lua`:

```lua
o.bind("SUPER + ALT + A", "Orbit quick capture", "orbit-app --capture")
```

### Updates

The AppImage checks `https://github.com/moneytosms/orbit/releases/latest/download/latest.json` at start. A newer, signature-checked AppImage is downloaded and installed, and a notification asks you to restart. It never restarts by itself. The `.deb` is not updated this way. For an AUR install, update with the AUR helper: `yay -Syu orbit-bin`.

### Build from source

```sh
just wasm                                  # builds orbit-core for the browser into app/wasm
pnpm -C app install --frozen-lockfile
cargo tauri build --bundles appimage       # output: target/release/bundle/appimage/
```

Build dependencies on Debian or Ubuntu: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`. `cargo tauri` is the Tauri CLI; CI installs it with `taiki-e/install-action` (see `.github/workflows/release.yml`). `app/package.json` has no `tauri` script, so run it from the repo root as shown.

## 4. Android

Orbit runs on Android 12 and newer (minSdk 31). The release APK is signed with the project's release key and does not allow cleartext HTTP, so your server must use HTTPS.

### Install the APK

Either:

- **Sideload:** download `Orbit-<version>-android.apk` from the [GitHub Releases](https://github.com/moneytosms/orbit/releases) page. Open it on the phone and allow installing from that browser or file manager when Android asks (Settings > Apps > Special app access > Install unknown apps).
- **Obtainium:** add the app with the repo URL `https://github.com/moneytosms/orbit` and set the APK filter to `Orbit-.*-android\.apk`, as in [releasing.md](releasing.md#obtainium). Obtainium then installs and updates the APK from the releases.

### First run

1. Open Orbit. The first page asks for your server URL. Enter the HTTPS address and continue. The app checks the server is reachable, saves the URL and opens it.
2. Log in. The phone becomes a Device (listed in Settings > Account and devices).
3. Grant **notifications** when asked (`POST_NOTIFICATIONS`).
4. Reminders use exact alarms. The manifest declares `USE_EXACT_ALARM`, so there is no separate prompt.
5. Battery: on first launch Orbit shows a one-time dialog about battery savers. Tap **Open** to reach Orbit's app settings, then open **Battery** and set it to **Unrestricted**. Some phone makers hide this under a different name; see [Troubleshooting](#8-troubleshooting).

### Widgets

Long-press the home screen, open **Widgets**, find Orbit, and drag a widget out. Available widgets (labels from `res/values/strings.xml`):

- **Quick add**: tap to add a task to Orbit.
- **Task list**: shows a view, area, project or label as a list. Pick what it shows when you add it (the widget configuration screen opens).
- **Routines**: today's Routines; tap one to check it in.
- **Goal**: one Goal as a card, or a list of Goals; tap to open it in Orbit. Pick the Goal when you add it.

### Share to Orbit

Share any text from another app (browser, notes, a chat) and choose **Orbit**. The text becomes an Inbox task. This works offline; the task syncs when you are back online.

## 5. CLI and MCP

The CLI runs on your own device and needs no checkout or Rust toolchain. On Linux (x86_64 and aarch64):

```sh
curl -fsSL https://raw.githubusercontent.com/moneytosms/orbit/main/scripts/install-cli.sh | sh
orbit login https://orbit.example.com
orbit ls
```

Install options, sign-in details (`--name`, two-factor codes), `orbit sync`, `orbit logout` and updates are on one page: [cli-install.md](cli-install.md). The full reference is in [cli.md](cli.md) and the man pages are in `docs/man/`.

### MCP (AI tools)

`orbit mcp` runs the same commands as an MCP server over stdio. Sign in with `orbit login` first. To add it to Claude Code:

```sh
claude mcp add orbit -- orbit mcp
```

Or in a project's `.mcp.json`:

```json
{
  "mcpServers": {
    "orbit": {
      "command": "orbit",
      "args": ["mcp"]
    }
  }
}
```

## 6. Updating each client

- **Server:** see [deploy.md](deploy.md).
- **Web / PWA:** the service worker updates itself in the background (`registerType: 'autoUpdate'`). Close and reopen the app to load the new version.
- **Linux AppImage:** the in-app updater installs new releases; restart when it asks. AUR installs: `yay -Syu orbit-bin`.
- **Android:** install the new APK over the old one, or let Obtainium do it. Android only updates an app when the new APK has the same signature, so always use the release APK and never a debug build.
- **CLI:** run the install command again (see [cli-install.md](cli-install.md#4-update)).

Run `orbit sync` before a CLI update so you do not leave queued changes behind.

## 7. Uninstall and revoke a device

1. Revoke the Device first: Settings > **Account and devices** > **Revoke** on that Device. The device is signed out on the server and can no longer sync.
2. Remove the local copy:
   - **Web:** remove the installed app (browser's app menu, or Settings on the phone), and clear the site data if you want the offline copy gone.
   - **Linux:** delete the AppImage (or `yay -R orbit-bin`), disable autostart, and remove the local data with `rm -r ~/.local/share/orbit` (or `$XDG_DATA_HOME/orbit`). This deletes the local replica and the saved token.
   - **Android:** uninstall Orbit from the launcher or Settings > Apps.
   - **CLI:** run `orbit sync` first, then `orbit logout`. Logout refuses while changes are unsynced unless you pass `--yes`. Then delete the `orbit` binary (`cargo uninstall orbit-cli`).

Changing the server password signs out every other Device. `orbit-server reset-password` does the same (see [deploy.md](deploy.md)).

## 8. Troubleshooting

**Can't connect to the server**

- The URL must start with `https://`. The Android release APK refuses plain `http://`.
- Check the certificate in a browser on the same device. A certificate error means the server's TLS is not trusted yet. Fix it on the server side (see [deploy.md](deploy.md): Caddy with a domain gets a certificate automatically; Tailscale serve uses a `*.ts.net` certificate).
- **Tailscale:** the phone or laptop must have the Tailscale app running and connected to the same tailnet. Turn it on, then retry. Use the server's Tailscale name in the URL.
- `orbit status` (CLI) shows the server address, sign-in state and how many changes are waiting.
- On the Linux app, if the saved server does not answer, the address page returns with a Retry button. Check the server is up, then retry.

**Changes sit in the queue**

Orbit keeps working offline and sends queued changes when the connection returns. Open the app or run `orbit sync` once online.

**Reminders are late or missing on Android**

- Set Orbit to **Unrestricted** in battery settings (step 5 of [First run](#first-run)). Battery savers can delay or kill alarms.
- Check notifications are allowed for Orbit.
- Some phone makers add their own "auto-start" or "background activity" switch; enable it for Orbit.
- Alarms do not survive a reboot; Orbit arms the next reminder again on boot (`BootReceiver`).
- The Android checklist in [android-checklist.md](android-checklist.md) describes how to test reminders on a real device.

**Browser reminders do not fire**

Browser reminders only fire while an Orbit tab is open. Use the Android app or the Linux desktop app for reminders while the browser is closed.

**Linux: no notifications or tray icon**

Check that a notification daemon and a system tray host are running. The tray uses libayatana-appindicator; on GNOME it needs the AppIndicator extension.
