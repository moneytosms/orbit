# Install Orbit on your devices

For people who already run an Orbit server and want to put a client on each device. Every client talks to the same server over HTTPS, and each install you sign in becomes a **Device** on your account.

## Before you start

- You need a running Orbit server that your devices can reach over **HTTPS**. Set one up with [Deploy](deploy.md) (Tailscale serve, Caddy with a domain, or a [Cloudflare Tunnel](cloudflare-tunnel.md)). The address you end up with is the one every client below asks for: [Connect your clients](deploy.md#5-connect-your-clients).
- You have one account, created with the setup code the server prints on first run (see [Deploy](deploy.md)).
- Each client you sign in becomes its own Device: web browser, Linux desktop app, Android app, CLI. Settings > Account and devices lists them and has a **Revoke** button per Device.

## Steps

Pick the clients you want:

| Client | Platforms | Where |
| --- | --- | --- |
| Web / PWA | any browser; installable on desktop and phone | below |
| Desktop app | Linux AppImage | [Desktop app](linux-desktop.md) |
| Android | Android 12+ (minSdk 31), sideloaded APK | [Android app](android.md) |
| CLI and MCP | Linux prebuilt; anywhere else from source | [CLI: install and sign in](cli-install.md) |

### Web / PWA (any desktop or phone browser)

1. Open your server's address (for example `https://orbit.example.com`) and log in. Use your password and, if enabled, the TOTP code.
2. Install it as an app:
   - **Chrome or Edge (desktop):** click the install icon in the address bar, or menu > **Install Orbit**.
   - **Android Chrome:** menu > **Add to Home screen** (or **Install app**).
   - **iOS Safari:** Share > **Add to Home Screen**.
3. Reminders in the browser: open Settings > Reminders and turn on **Notify on this device**, then allow notifications when the browser asks. Browser reminders only fire while an Orbit tab is open. For reminders on a phone, use the Android app.
4. Offline: the app shell, the wasm core and the fonts are precached by a service worker (`vite-plugin-pwa`, see `app/vite.config.ts`). Open the app once while online; after that it opens offline and queues changes until the connection returns.

### Updating each client

- **Server:** see [Deploy](deploy.md).
- **Web / PWA:** the service worker updates itself in the background (`registerType: 'autoUpdate'`). Close and reopen the app to load the new version.
- **Linux AppImage:** the in-app updater installs new releases; restart when it asks.
- **Android:** install the new APK over the old one, or let Obtainium do it. Android only updates an app when the new APK has the same signature, so always use the release APK and never a debug build.
- **CLI:** run the install command again (see [cli-install.md](cli-install.md#4-update)).

Run `orbit sync` before a CLI update so you do not leave queued changes behind.

### Uninstall and revoke a device

1. Revoke the Device first: Settings > **Account and devices** > **Revoke** on that Device. The device is signed out on the server and can no longer sync.
2. Remove the local copy:
   - **Web:** remove the installed app (browser's app menu, or Settings on the phone), and clear the site data if you want the offline copy gone.
   - **Linux:** delete the AppImage, disable autostart, and remove the local data with `rm -r ~/.local/share/orbit` (or `$XDG_DATA_HOME/orbit`). This deletes the local replica and the saved token.
   - **Android:** uninstall Orbit from the launcher or Settings > Apps.
   - **CLI:** run `orbit sync` first, then `orbit logout`. Logout refuses while changes are unsynced unless you pass `--yes`. Then delete the `orbit` binary (`cargo uninstall orbit-cli`).

Changing the server password signs out every other Device. `orbit-server reset-password` does the same (see [deploy.md](deploy.md)).

## Verify it worked

Open Settings > Account and devices in the web app. The browser you just signed in with is listed as a Device. Add a task there, then run `orbit sync && orbit ls` on any other signed-in client to see it arrive.

## Troubleshooting

**Can't connect to the server**

- The URL must start with `https://`. The Android release APK refuses plain `http://`.
- Check the certificate in a browser on the same device. A certificate error means the server's TLS is not trusted yet. Fix it on the server side (see [Deploy](deploy.md): Caddy with a domain gets a certificate automatically; Tailscale serve uses a `*.ts.net` certificate).
- **Tailscale:** the phone or laptop must have the Tailscale app running and connected to the same tailnet. Turn it on, then retry. Use the server's Tailscale name in the URL.
- `orbit status` (CLI) shows the server address, sign-in state and how many changes are waiting.

**Changes sit in the queue**

Orbit keeps working offline and sends queued changes when the connection returns. Open the app or run `orbit sync` once online.

**Browser reminders do not fire**

Browser reminders only fire while an Orbit tab is open. Use the [Android app](android.md) or the [Desktop app](linux-desktop.md) for reminders while the browser is closed.

## Next

[Your first ten minutes](first-ten-minutes.md) if you have not added a task yet, otherwise [Quick add and tasks](tasks.md).
