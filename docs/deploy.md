# Deploying orbit-server

How to run `orbit-server` for daily personal use: one process, one SQLite file, one web UI,
reachable from your laptop and phone. Commands below match `crates/orbit-server/src/main.rs`.

## Overview

`orbit-server` is the source of truth for your data. It keeps everything in SQLite under a data
directory (`orbit.db`, attachments in `blobs/`, snapshots in `backups/`) and serves:

- the sync API the apps talk to;
- the web UI (a PWA) at `/`, when `ORBIT_WEB` points at the built web app. The Docker image sets this for you.

Clients (the Linux desktop app, the Android app, the web UI and the CLI) sync against it. Nothing is
stored anywhere else.

Configuration is by environment variable:

| Variable           | Default           | Meaning                                                              |
| ------------------ | ----------------- | -------------------------------------------------------------------- |
| `ORBIT_DATA`       | `data`            | Data directory (`orbit.db`, `blobs/`, `backups/`). Docker image: `/data`. |
| `ORBIT_ADDR`       | `127.0.0.1:8080`  | Address the server listens on. Docker image: `0.0.0.0:8080`.        |
| `ORBIT_WEB`        | unset             | Directory with the built web UI, served at `/`. Docker image: `/web`. |
| `ORBIT_MAX_BLOB_MB`| `25`              | Largest attachment accepted, in MB.                                  |

## Requirements

For Docker (recommended):

- Docker with the Compose plugin (`docker compose version` must work).
- Room on disk for your data directory.
- The image is published for `linux/amd64` and `linux/arm64`.

For running from source:

- Rust stable (`rust-toolchain.toml` pins stable, plus the `wasm32-unknown-unknown` target).
- `wasm-pack` (`cargo install --locked wasm-pack`) and `just`.
- Node.js 20.19 or newer and pnpm 12.

For phones and other devices you need HTTPS. See [HTTPS](#https).

## Quick start (Docker Compose)

The Compose file is `deploy/compose.yaml`. It runs one `orbit` service, publishes it on
`127.0.0.1:8080` only, and keeps data in `./data` next to the file.

```sh
git clone https://github.com/moneytosms/orbit.git
cd orbit/deploy
docker compose up -d
docker compose logs orbit
```

Open <http://127.0.0.1:8080> in a browser on the same machine. For any other device, use HTTPS
(see below); the server is deliberately bound to localhost.

To build the image from your checkout instead of pulling it, edit `compose.yaml`: comment out the
`image:` line and uncomment `build: ..`, then run `docker compose up -d --build`.

## First run

On first start, with no user yet, the server prints a one-time setup code to its log:

```text
First run: open the app and enter setup code <code>
```

Read it with:

```sh
docker compose logs orbit | grep "setup code"
```

Then:

1. Open the web UI and enter the setup code.
2. Create your account. The password must be at least 8 characters.
3. Optionally turn on TOTP (two-factor codes) in Settings.

Once the account exists, the setup code is no longer shown and the setup step is gone for good.
Use Settings → Devices to see signed-in devices and revoke any you no longer use.

## HTTPS

Plain HTTP works on the machine running the server. Other devices should use HTTPS: the Android
release APK refuses cleartext (`http://`) connections entirely. Pick one of the two options below.
Both keep the server itself on plain HTTP inside the host or the container.

### A) Tailscale (no public exposure)

Your server and devices join a private tailnet; nothing is exposed to the internet.

1. Keep the server on localhost. The Compose file already publishes `127.0.0.1:8080` only.
2. On the server machine, install Tailscale and sign in (`tailscale up`). In the Tailscale admin
   console, enable MagicDNS and HTTPS certificates if you have not already.
3. Expose the local port over HTTPS in the background:

   ```sh
   tailscale serve --bg 8080
   tailscale serve status
   ```

   `status` shows your address, of the form `https://<machine>.<tailnet>.ts.net`.
4. Install Tailscale on every device you use (phone, laptop), sign in to the same tailnet, and open
   that `https://` address, and use that address as the server in the apps.

To stop sharing: `tailscale serve reset`.

### B) Caddy with a domain (public HTTPS)

Use this if you want the server reachable from anywhere without Tailscale. It needs a domain you
control.

1. Create a DNS `A` record for your domain (for example `orbit.example.com`) pointing at the
   server's public IP.
2. Open ports 80 and 443 on the server and its firewall. Caddy uses port 80 for certificate
   challenges and port 443 for HTTPS.
3. Edit `deploy/Caddyfile` and replace the domain:

   ```caddyfile
   orbit.example.com {
   	reverse_proxy orbit:8080
   }
   ```

4. Start the Caddy variant from `deploy/`:

   ```sh
   docker compose -f compose.caddy.yaml up -d
   ```

Caddy obtains and renews certificates automatically; they are stored in the `caddy_data` volume.
In this variant the `orbit` service is not published on the host at all; only Caddy listens, on
80 and 443. Use `https://orbit.example.com` on every device.

Use one variant at a time. Both use `./data` and the same service name, so do not run them side by
side against the same directory.

## Backups and restore

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

## Export

Settings has an export action that downloads `orbit-export.zip`: `items.json` with every row, each
live page as `pages/<name>.md`, and the attachments it references under `blobs/`.

The export is a readable copy of your data. To restore, use a snapshot (see above), not the export.

## Reset password

If you forget the password, reset it on the host while the server runs. The new password is read
from stdin, one line:

```sh
printf '%s\n' 'your-new-password' | docker compose exec -i orbit orbit-server reset-password
```

The reset also turns TOTP off and signs out every Device. The new password must be at least 8
characters.

## Updating

```sh
cd deploy
docker compose pull
docker compose up -d
```

Your data stays in `./data`, so an update never touches it. Optionally take a manual backup
first (see above). To pin a version, replace `:latest` in the Compose file with a release tag such
as `:0.1.0`.

## Running without Docker

Build the web app and the server from the repository root:

```sh
just wasm && pnpm -C app install && pnpm -C app build
cargo build --release -p orbit-server
```

Run it, with the built web app and a data directory:

```sh
ORBIT_WEB=app/dist ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server
```

Defaults are `ORBIT_ADDR=127.0.0.1:8080` and `ORBIT_DATA=data`. Subcommands work the same way:

```sh
ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server backup "$HOME/orbit-manual"
ORBIT_DATA="$HOME/orbit-data" ./target/release/orbit-server reset-password
```

Stop the server before `restore`, as above.

### systemd user unit

Save as `~/.config/systemd/user/orbit-server.service` and replace the paths with absolute ones
for your checkout:

```ini
[Unit]
Description=orbit-server
After=network-online.target

[Service]
ExecStart=/home/you/orbit/target/release/orbit-server
Environment=ORBIT_WEB=/home/you/orbit/app/dist
Environment=ORBIT_DATA=/home/you/orbit-data
Environment=ORBIT_ADDR=127.0.0.1:8080
Restart=on-failure

[Install]
WantedBy=default.target
```

Then:

```sh
systemctl --user daemon-reload
systemctl --user enable --now orbit-server
systemctl --user status orbit-server
journalctl --user -u orbit-server -f
loginctl enable-linger "$USER"
```

`enable-linger` keeps the user service running after you log out. Put the Tailscale or Caddy
setup from [HTTPS](#https) in front of it as before.

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

**`permission denied` on `./data`.** The container runs as uid 10001 and needs to write `./data`.
Create the directory and give it to that uid:

```sh
sudo chown -R 10001:10001 data
docker compose up -d
```

**Phone says "cleartext not permitted" or cannot connect.** The Android release app refuses plain
`http://` URLs. Use the HTTPS address from [HTTPS](#https) (Tailscale or Caddy), not
`http://<ip>:8080`.

**Locked out after failed sign-ins (`rate_limited`, HTTP 429).** After five failed attempts from one
address, that address is blocked for a minute, and each further failure doubles the wait. Wait it
out, or use the reset above if you have really forgotten the password.
