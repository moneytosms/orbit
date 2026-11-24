# Deploying orbit-server

For people hosting Orbit for themselves on a laptop, home server or VPS. Run `orbit-server` for daily personal use: one process, one SQLite file, one web UI, reachable from
your laptop and phone. Commands match `crates/orbit-server/src/main.rs`.

## Before you start

- A machine that stays on: Docker with the Compose plugin (`docker compose version` prints a version), or a Rust toolchain for a source build. 1 GB of RAM and 2 vCPUs are enough.
- For access from other devices, one of: a Tailscale account, a domain pointing at the machine, or a Cloudflare account.

Pick your path:

1. [Run it](#1-run-it) on one machine (five minutes, Docker).
2. [Reach it from other devices](#2-reach-it-from-other-devices) over HTTPS, with [Tailscale](#3-https-with-tailscale), [your own domain](#4-https-with-a-domain-caddy) or a [Cloudflare Tunnel](cloudflare-tunnel.md).
3. [Connect each client](#5-connect-your-clients) using the one server address.
4. [Keep it running](#6-keep-it-running) across reboots, then [back up, update and recover](#7-backups-updates-and-recovery).

## What you are running

`orbit-server` is the source of truth for your data. It keeps everything in SQLite under a data
directory (`orbit.db`, attachments in `blobs/`, snapshots in `backups/`) and serves the sync API the
apps talk to, plus the web UI (a PWA) at `/` when `ORBIT_WEB` points at the built web app. The Docker
image sets that for you. Nothing is stored anywhere else.

The database runs in SQLite WAL mode, so `orbit.db-wal` and `orbit.db-shm` appear next to `orbit.db`.
Copy snapshots (below), not the bare `orbit.db`, when backing up or snapshotting a volume.

| Variable            | Default          | Meaning                                                                   |
| ------------------- | ---------------- | ------------------------------------------------------------------------- |
| `ORBIT_DATA`        | `data`           | Data directory (`orbit.db`, `blobs/`, `backups/`). Docker image: `/data`. |
| `ORBIT_ADDR`        | `127.0.0.1:8080` | Address the server listens on. Docker image: `0.0.0.0:8080`.              |
| `ORBIT_WEB`         | unset            | Directory with the built web UI, served at `/`. Docker image: `/web`.     |
| `ORBIT_MAX_BLOB_MB` | `25`             | Largest attachment accepted, in MB.                                       |
| `ORBIT_SECURE_COOKIE` | unset          | `1` or `true` adds `Secure` to the web session cookie. Set it when the UI is served over HTTPS (proxy or tunnel); leave unset for plain-HTTP LAN use, where browsers would drop the cookie. |
| `ORBIT_SYNC_TRACE`  | unset            | Set to `1` for a debug latency trace of sync requests. See [sync-latency.md](sync-latency.md). |

## 1. Run it

Needs Docker with the Compose plugin (`docker compose version` must work). The image is published
for `linux/amd64` and `linux/arm64`. No checkout and no Rust toolchain are needed:

```sh
mkdir orbit && cd orbit
curl -fsSLO https://raw.githubusercontent.com/moneytosms/orbit/main/deploy/compose.yaml
docker compose up -d
docker compose logs orbit | grep "setup code"
```

The data directory `./data` is created and handed to the server's user for you. The server listens on
`127.0.0.1:8080` only, so nothing else on your network can reach it yet.

### Create your account

On first start, with no user yet, the server prints a one-time setup code:

```text
First run: open the app and enter setup code <code>
```

1. Open <http://127.0.0.1:8080> in a browser on the same machine and enter the setup code.
2. Create your account. The password must be at least 8 characters.
3. Optionally turn on TOTP (two-factor codes) in Settings.

Once the account exists the setup step is gone for good. Settings, Account and devices lists signed-in
Devices and has a Revoke button for each.

To build the image from a checkout instead of pulling it, edit `deploy/compose.yaml`: comment out the
`image:` line, uncomment `build: ..`, then run `docker compose up -d --build`.

## 2. Reach it from other devices

Plain HTTP is fine on the machine running the server. Any other device needs **HTTPS**: the Android
release app refuses `http://` addresses entirely. Every option below keeps the server itself on plain
HTTP inside the host or container and adds HTTPS in front.

| | Tailscale | Domain with Caddy |
| --- | --- | --- |
| Reachable from | devices on your tailnet only | anywhere on the internet |
| You need | a free Tailscale account on every device | a domain, a public IP, ports 80 and 443 open |
| Certificate | `*.ts.net`, handled by Tailscale | Let's Encrypt, handled by Caddy |
| Address looks like | `https://<machine>.<tailnet>.ts.net` | `https://orbit.example.com` |

A third path, a Cloudflare Tunnel, needs no public IP and no open ports, and one tunnel can carry
several apps with a hostname each. It has its own page: [cloudflare-tunnel.md](cloudflare-tunnel.md).

Not sure? Pick Tailscale: nothing is exposed to the internet.

## 3. HTTPS with Tailscale

1. Keep the server on localhost. `compose.yaml` already publishes `127.0.0.1:8080` only.
2. On the server machine, install Tailscale and sign in (`tailscale up`). In the Tailscale admin
   console, enable MagicDNS and HTTPS certificates if you have not already.
3. Share the local port over HTTPS in the background:

   ```sh
   tailscale serve --bg 8080
   tailscale serve status
   ```

   `status` prints your address, of the form `https://<machine>.<tailnet>.ts.net`. That is your
   [server address](#5-connect-your-clients).
4. Install Tailscale on every device you use and sign in to the same tailnet.

To stop sharing: `tailscale serve reset`.

## 4. HTTPS with a domain (Caddy)

1. Create a DNS `A` record for your domain (for example `orbit.example.com`) pointing at the
   server's public IP.
2. Open ports 80 and 443 on the server and its firewall. Caddy uses port 80 for certificate
   challenges and 443 for HTTPS.
3. In a checkout (or by downloading `compose.caddy.yaml` and `Caddyfile` from `deploy/` into one
   directory), edit `Caddyfile` and put in your domain:

   ```caddyfile
   orbit.example.com {
   	reverse_proxy orbit:8080
   }
   ```

4. Start the Caddy variant from that directory:

   ```sh
   docker compose -f compose.caddy.yaml up -d
   ```

Caddy gets and renews certificates itself and keeps them in the `caddy_data` volume. In this variant
the `orbit` service is not published on the host at all; only Caddy listens, on 80 and 443. Your
[server address](#5-connect-your-clients) is `https://orbit.example.com`.

Use one variant at a time. Both use `./data` and the same service name, so do not run them against the
same directory together. To move from the plain file to the Caddy one, run `docker compose down`
first.

## 5. Connect your clients

Every client needs one thing: the **server address**, the `https://` URL from section 3 or 4 (or
`http://127.0.0.1:8080` on the server machine itself). Use the bare origin: no trailing path, and
the same address on every device. Replace `https://orbit.example.com` below with yours.

| Client | How to give it the address | What happens |
| --- | --- | --- |
| Web / PWA | Open the address in a browser. | You land on the login. Install it as an app from the browser menu. |
| Linux desktop | First launch shows an address page: enter it. Or run `orbit-app --server https://orbit.example.com`. | The address is saved to `~/.local/share/orbit/server` and the login opens. |
| Android | First launch shows an address page: enter it. | The address is saved on the device, then the login opens. Must be `https://`. |
| CLI | `orbit login https://orbit.example.com` | Reads your password from stdin or `ORBIT_PASSWORD`, signs in and saves the address. |

Each sign-in becomes a Device on your account. Install steps per client are in
[install.md](install.md); flags and files for the desktop app are in
[linux-desktop.md](linux-desktop.md); the CLI is in [cli.md](cli.md).

## 6. Keep it running

### Docker

`compose.yaml` sets `restart: unless-stopped`, so the container comes back after a crash or a reboot
as long as Docker itself starts at boot. On most Linux systems enable that once:

```sh
sudo systemctl enable --now docker
```

Check after a reboot with `docker compose ps` in the directory holding `compose.yaml`.

### Without Docker (systemd)

Build the web app and the server from the repository root:

```sh
just wasm && pnpm -C app install && pnpm -C app build
cargo build --release -p orbit-server
```

Run it by hand to try it:

```sh
ORBIT_WEB=app/dist ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server
```

To keep it running, install `deploy/orbit-server.service` as a user service. It expects the repo at
`~/orbit` and keeps data in `~/orbit-data`; edit the three paths in the file if yours differ:

```sh
mkdir -p ~/.config/systemd/user
cp deploy/orbit-server.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now orbit-server
loginctl enable-linger "$USER"
```

`enable-linger` starts the service at boot instead of only after you log in. Look at it with
`systemctl --user status orbit-server` and `journalctl --user -u orbit-server -f`. The setup code
appears in that journal on first start. Put [Tailscale](#3-https-with-tailscale) or
[Caddy](#4-https-with-a-domain-caddy) in front as before; Caddy needs the Docker variant, or your own
Caddy install pointing at `127.0.0.1:8080`.

Subcommands work the same way, with `ORBIT_DATA` set:

```sh
ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server backup "$HOME/orbit-manual"
ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server reset-password
```

## 7. Backups, updates and recovery

### Automatic snapshots

The server checks once an hour. If the newest snapshot is older than 24 hours (or none exists),
it writes one. Snapshots go to `$ORBIT_DATA/backups/orbit-<timestamp-ms>.db`, the seven newest
are kept, and older ones are deleted. Attachments are copied into `backups/blobs/` alongside them;
that shared blob folder is never pruned.

With the Compose file that is `deploy/data/backups/`.

### Manual backup

A snapshot is a consistent copy taken with SQLite `VACUUM INTO`, so it is safe while the server
runs:

```sh
docker compose exec orbit orbit-server backup /data/manual-backup
```

This writes `./data/manual-backup/orbit-<timestamp-ms>.db` plus blobs. Copy the directory off the
machine to keep it safe.

### Copy backups off the machine

The automatic snapshots sit on the same disk as the data, so they are not a backup on their own. A daily
job takes a manual snapshot and copies both kinds off the machine, keeping the ten newest manual ones:

```sh
cat > "$HOME/orbit-backup.sh" <<'SH'
#!/bin/sh
set -eu
DIR="$HOME/orbit"
OFFBOX="$HOME/orbit-offbox"
cd "$DIR"
docker compose exec -T orbit orbit-server backup /data/manual-backup
mkdir -p "$OFFBOX/manual" "$OFFBOX/auto"
rsync -a --delete "$DIR/data/manual-backup/" "$OFFBOX/manual/"
rsync -a "$DIR/data/backups/" "$OFFBOX/auto/"
find "$OFFBOX/manual" -name 'orbit-*.db' | sort | head -n -10 | xargs -r rm -f
SH
chmod +x "$HOME/orbit-backup.sh"
( crontab -l 2>/dev/null; echo "15 4 * * * $HOME/orbit-backup.sh >> $HOME/orbit-backup.log 2>&1" ) | crontab -
```

Then pull `~/orbit-offbox` to another machine with whichever tool you already use (rsync, rclone,
restic).

### Restore

Restore replaces the live database, so the server must be stopped:

```sh
docker compose stop orbit
docker compose run --rm orbit restore /data/backups/orbit-<timestamp-ms>.db
docker compose up -d orbit
```

`restore` copies the snapshot to `orbit.db` (removing any stale WAL and SHM files) and copies blobs
from the snapshot's folder into the data directory. The `orbit` service's entrypoint is
`orbit-server`, so the arguments are the subcommand and its path.

### Export

Settings has an export action that downloads `orbit-export.zip`: `items.json` with every row, each
live page as `pages/<name>.md`, and the attachments it references under `blobs/`. From the CLI:
`orbit export orbit-export.zip`.

The export is a readable copy of your data. To restore, use a snapshot (see above), not the export.

### Reset password

If you forget the password, reset it on the host while the server runs. The new password is read
from stdin, one line:

```sh
printf '%s\n' 'your-new-password' | docker compose exec -i orbit orbit-server reset-password
```

The reset also turns TOTP off and signs out every Device. The new password must be at least 8
characters.

### Updating

In the directory holding `compose.yaml`:

```sh
docker compose pull
docker compose up -d
```

Your data stays in `./data`, so an update never touches it. Optionally take a manual backup
first (see above). To pin a version, replace `:latest` in the Compose file with a release tag such
as `:0.1.0`.

## Verify it worked

```sh
curl -sS http://127.0.0.1:8080/auth; echo
```

It prints `{"setup":false}` once your account exists (`{"setup":true}` means the server is up and still waiting for the setup code). From another device, open the same address over HTTPS and sign in.

## Troubleshooting

**The setup code is lost.** While no user exists, the server generates a new setup code on every
start and prints it. Read the latest one:

```sh
docker compose logs orbit | grep "setup code"
```

Or restart the container (`docker compose restart orbit`) and read the log again. Only the most
recently printed code works.

**Port already in use.** The server exits with `bind ORBIT_ADDR`. Either stop the other process, or
change the host side of the port mapping in `compose.yaml` (for example `127.0.0.1:8081:8080`), or
set `ORBIT_ADDR` for a source build.

**`permission denied` on `./data`.** The `init` service normally fixes this on every start. If you
removed it or use your own compose file, give the directory to uid 10001:

```sh
sudo chown -R 10001:10001 data
docker compose up -d
```

**Phone says "cleartext not permitted" or cannot connect.** The Android release app refuses plain
`http://` URLs. Use the HTTPS address from [HTTPS](#2-reach-it-from-other-devices) (Tailscale or Caddy), not
`http://<ip>:8080`.

**Locked out after failed sign-ins (`rate_limited`, HTTP 429).** After five failed attempts from one
address, that address is blocked for a minute, and each further failure doubles the wait. Wait it
out, or use the reset above if you have really forgotten the password.

## Next

[Install your clients](install.md): the web app, desktop app, Android and CLI. Using a Cloudflare Tunnel instead? See [Deploy: Cloudflare Tunnel](cloudflare-tunnel.md).
