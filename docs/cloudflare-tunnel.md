# HTTPS with a Cloudflare Tunnel

A Cloudflare Tunnel gives your server a real HTTPS address on your own domain without opening a single
inbound port and without a public IP. `cloudflared` runs on the machine, dials out to Cloudflare, and
Cloudflare forwards requests for a hostname to the server on `127.0.0.1:8080`.

The useful part for a self-hoster: **one tunnel carries every app on the machine**. Each app gets its
own hostname, all of them served by the same `cloudflared` process, so you set the tunnel up once and
add routes to it later. Orbit is simply one of those routes.

Pick this path instead of [Caddy](deploy.md#4-https-with-a-domain-caddy) when the machine is behind
NAT, when you do not want to open ports 80 and 443, or when you already run other services behind one
tunnel. Pick [Tailscale](deploy.md#3-https-with-tailscale) instead when the server must not be reachable
from the open internet at all.

What you need:

- A domain whose DNS is managed by Cloudflare (the zone has to be in the account that creates the
  tunnel).
- A machine that can reach Cloudflare on the outbound side, which every VPS can.
- About ten minutes, once.

## What the server expects from the proxy

These come from the server and client code, so they decide how the route has to look.

| Behaviour | What it means for the setup |
| --- | --- |
| The web UI uses absolute paths (`/sync`, `/auth`, `/blobs/...`) and ships a root-scoped PWA and service worker | Use a **subdomain root** such as `https://orbit.example.com`. A subpath such as `example.com/orbit` does not work without rebuilding the app. |
| The Linux and Android apps load the UI **from the server** (`app/src-tauri/src/desktop.rs`, `android.rs`), and the release APK has `usesCleartextTraffic="false"` | The server must serve the built web app (`ORBIT_WEB`, already `/web` in the Docker image) and must be reached over real HTTPS. The Docker image covers the first part. |
| Login rate limiting keys on the socket peer address (`ConnectInfo<SocketAddr>` in `crates/orbit-server/src/auth.rs`), and the server does not read `X-Forwarded-For` | Behind a tunnel every request arrives from `127.0.0.1`, so the limit becomes one bucket shared by all clients. See [The tunnel and the login lockout](#the-tunnel-and-the-login-lockout). |
| The session cookie is `HttpOnly; SameSite=Strict` with no `Secure` flag | Fine behind an HTTPS tunnel. Keep the UI and the API on the same origin, which a single hostname already does. |
| The clients sync with `POST /sync` every 60 seconds while visible, plus a 2 second debounce after an edit | No websockets and no long polling, so the Cloudflare free plan is enough and the 100 second origin timeout is not a risk. |
| Attachment uploads are limited by `ORBIT_MAX_BLOB_MB`, default 25 | Below Cloudflare's 100 MB request ceiling. Leave the default. |
| Every client stores the server address without a trailing slash and expects a bare origin | Hand out `https://orbit.example.com` and nothing longer. |

## Request volume and tunnel cost

For one continuously visible client, the normal sync timer yields 1,440 `/sync` requests/day at
60 seconds, 17,280/day at 5 seconds, and 21,600/day at 4 seconds. These are interval calculations
(`86,400 seconds/day ÷ polling interval`), not Cloudflare billing-meter readings; multiply by the
number of visible clients and add requests caused by edits. A 4 second timer is 15 times the current
poll request volume.

The tunnel itself adds an estimated $0 plan charge for an included tunnel within Cloudflare's first
1,000 tunnels. This does not price the origin machine, its network transfer, or other Cloudflare
products. Check the current [Tunnel overview](https://developers.cloudflare.com/tunnel/) and
[tunnel pricing](https://developers.cloudflare.com/tunnel/features/tunnels-for-platforms/) before
budgeting a deployment; account terms can change.

## 1. Run the server on the VPS

Follow [Run it](deploy.md#1-run-it), then make the data mount absolute so a wrong working directory
can never create a second, empty database:

```sh
export ORBIT_DIR="$HOME/orbit"
mkdir -p "$ORBIT_DIR/data" && cd "$ORBIT_DIR"
curl -fsSLO https://raw.githubusercontent.com/moneytosms/orbit/main/deploy/compose.yaml
sed -i "s|- ./data:/data|- $ORBIT_DIR/data:/data|g" compose.yaml
docker compose up -d
sudo systemctl enable --now docker
docker compose logs orbit | grep -i "setup code"
```

The published port stays `127.0.0.1:8080`, which is exactly what `cloudflared` on the same machine
needs. Nothing is reachable from the network yet.

## 2. Create your account before the server is public

The first run is protected by a code printed to the logs, so finish it over localhost rather than over
the internet:

```sh
# on your laptop, leave the session open while you sign up
ssh -L 8080:127.0.0.1:8080 you@your-server
```

Create the account as in [Create your account](deploy.md#create-your-account), using a long password,
because this one will face the internet. Then confirm setup is closed:

```sh
curl -sS http://127.0.0.1:8080/auth
```

`{"setup":false}` means the setup step is gone for good. Close the SSH session afterwards.

## 3. Create one tunnel and install it as a service

Install `cloudflared` from Cloudflare's package repository:

```sh
sudo mkdir -p --mode=0755 /usr/share/keyrings
curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared any main" | sudo tee /etc/apt/sources.list.d/cloudflared.list
sudo apt update && sudo apt install -y cloudflared
cloudflared --version
```

Then in the Cloudflare dashboard:

1. Zero Trust, **Networks**, **Tunnels**, **Create a tunnel**. Choose *Cloudflared* and name it after
   the machine, for example `vps`.
2. Copy the token on the install page (`eyJhIjoi...`).
3. Install it as a systemd service, which registers the tunnel and starts it at boot:

```sh
export TUNNEL_TOKEN="paste-the-token-here"
sudo cloudflared service install "$TUNNEL_TOKEN"
sudo systemctl status cloudflared --no-pager
sudo journalctl -u cloudflared -n 20 --no-pager | grep -i "registered\|connected\|error"
```

`active (running)` and a `Registered tunnel connection` line mean the tunnel is up. Do not also run
`cloudflared tunnel run` by hand, one process per tunnel.

A tunnel created this way is managed from the dashboard: routes live in Cloudflare, not on disk. If you
would rather keep routes in a file you version, create the tunnel with `cloudflared tunnel login` and
`cloudflared tunnel create vps` instead, and put the ingress rules below in
`/etc/cloudflared/config.yml`.

## 4. Route a hostname to Orbit

In the tunnel, **Public Hostname**, **Add a public hostname**:

| Field | Value |
| --- | --- |
| Subdomain | `orbit` |
| Domain | `example.com` |
| Path | leave empty |
| Type | `HTTP` |
| URL | `localhost:8080` |

Cloudflare creates the proxied `CNAME` to `<tunnel-id>.cfargotunnel.com` for you, and traffic for
`orbit.example.com` now goes to the server. Terminate TLS at Cloudflare: `cloudflared` talks plain
HTTP to `localhost:8080`, and clients see HTTPS.

With a locally managed tunnel the same route is a `config.yml` entry:

```yaml
tunnel: vps
credentials-file: /etc/cloudflared/<tunnel-id>.json

ingress:
  - hostname: orbit.example.com
    service: http://localhost:8080
  - service: http_status:404
```

Check it before restarting:

```sh
cloudflared tunnel ingress validate
cloudflared tunnel ingress rule https://orbit.example.com/sync
sudo systemctl restart cloudflared
```

## 5. Check it from outside

```sh
curl -sS -o /dev/null -w 'root %{http_code}\n' https://orbit.example.com/
curl -sS https://orbit.example.com/auth; echo
dig +short orbit.example.com | head -3
```

Expect `root 200`, `{"setup":false}`, and Cloudflare anycast addresses rather than your server's IP.
A `502` or a Cloudflare `1033` here means Cloudflare cannot reach the origin, so check
`docker compose ps` and `curl -sS http://127.0.0.1:8080/auth` on the VPS.

Now open the hostname in a browser and turn on TOTP in Settings, Account, before you sign any more
clients in.

## 6. Use the tunnel for more than one app

The tunnel is a property of the machine, not of Orbit. To add another service, add another public
hostname that points at that service's port, for example `notes.example.com` to `localhost:9090`. Both
hostnames share the one `cloudflared` process and its connection to Cloudflare.

In a locally managed `config.yml` the same thing is one more ingress entry, with the catch-all last:

```yaml
tunnel: vps
credentials-file: /etc/cloudflared/<tunnel-id>.json

ingress:
  - hostname: orbit.example.com
    service: http://localhost:8080
  - hostname: notes.example.com
    service: http://localhost:9090
  - hostname: files.example.com
    service: http://localhost:3000
  - service: http_status:404
```

Two habits keep this tidy:

- One subdomain per app, never a subpath. Orbit cannot do subpaths at all, and most other self-hosted
  apps have the same trouble with cookie scopes and absolute asset paths.
- Keep the catch-all `http_status:404` last. Without it `cloudflared` refuses to start.

If you would rather run `cloudflared` in Docker next to the apps, put it on the same network as the
service and use the service name as the origin (`http://orbit:8080`), then drop the host port mapping
from `deploy/compose.yaml` and keep `127.0.0.1:8080` only for debugging. In that case the ingress
entries or dashboard hostnames point at `orbit:8080`, not `localhost:8080`.

## 7. Edge settings that matter

- **Rate limit `/auth/login` and `/auth/setup`.** In **Security**, **WAF**, **Rate limiting rules**,
  match `(http.request.uri.path eq "/auth/login") or (http.request.uri.path eq "/auth/setup")`, count by
  IP, allow 10 requests a minute, then block for 10 minutes. Cloudflare sees the real client address,
  the origin does not (see the next section).
- **Leave Bot Fight Mode off** for this hostname, or expect native clients' API calls to be challenged.
  If it is on for the whole zone, add a WAF skip rule for `/sync`, `/auth/*`, `/blobs/*` and `/devices`.
- **Do not put Cloudflare Access in front of it.** The CLI, the Android background sync and the desktop
  replica authenticate with `Authorization: Bearer`, not a browser SSO session, and they cannot send
  Access service token headers. Orbit's password and TOTP are the access control here.
- **Do not enable Cache Everything.** Static assets are content hashed, but `/sync` and `/auth` must
  never be cached. The default Cloudflare behaviour is already correct.
- **Universal SSL covers one level of subdomain.** `orbit.example.com` is fine. A name like
  `orbit.apps.example.com` needs Total TLS or Advanced Certificate Manager, or clients get a
  certificate error even though the tunnel works.

## The tunnel and the login lockout

The server counts failed sign-ins per peer address, in memory (`failed` and `limited` in
`crates/orbit-server/src/auth.rs`). Five failures block that address for a minute, and each further
failure doubles the wait up to about 64 minutes. Behind a tunnel every request arrives from
`127.0.0.1`, so that block applies to everyone at once: a stranger guessing passwords can lock you out of
your own server.

Three things fix that in practice:

1. Keep TOTP on. A wrong password plus a missing code is still a failure, but the password alone no
   longer gets anyone in.
2. Keep the WAF rate limiting rule above in place, so guessing is stopped at the edge.
3. If you do get locked out, the counters are in memory, so a restart clears them:

```sh
cd "$HOME/orbit" && docker compose restart orbit
```

`rate_limited` with HTTP 429 is that lockout, not a broken tunnel.

## Backups, and what `docker compose down` does

`deploy/compose.yaml` mounts the data directory as a bind mount (`./data:/data`), not a named volume.
So `docker compose down`, `down -v`, `pull`, `up -d`, `restart`, reboots and a deleted image all leave
`~/orbit/data` alone. `-v` only removes named volumes, and this compose file has none.

What does lose data, and what to do about each:

| Risk | Guard |
| --- | --- |
| Running `docker compose` from a different directory, which creates a fresh `./data` and a new empty database | Always `cd "$HOME/orbit"` first, or make the mount absolute as in step 1. |
| Deleting or moving `~/orbit/data` | Snapshot before any reorganisation, and keep a copy off the machine. |
| `orbit-server restore <snapshot>`, which replaces the live database | Take a fresh snapshot immediately before restoring. |
| Disk failure, provider reclamation, a full disk | Copies stored off the machine. The seven automatic snapshots sit on the same disk, so they are not a backup. |

Backups, the off-machine copy script and restores are the same behind a tunnel: see
[Backups, updates and recovery](deploy.md#7-backups-updates-and-recovery).

## Connect the clients

Every client asks for the same thing, the bare origin `https://orbit.example.com`. Connect each one as in
[Connect your clients](deploy.md#5-connect-your-clients) and [install.md](install.md).

The Linux and Android apps load the UI from the server, so an unreachable hostname opens their retry
page rather than a cached interface.

## Updating

Follow [Updating](deploy.md#updating) in the same `~/orbit` directory, after a backup. The tunnel itself
needs no change on update.

## Troubleshooting

**Cloudflare error 1033, or `502` from the hostname.** Cloudflare has no origin to talk to. On the VPS,
check `systemctl status cloudflared`, then `docker compose ps` and `curl -sS http://127.0.0.1:8080/auth`.
The tunnel being up does not mean the server is.

**The tunnel never registers.** `sudo journalctl -u cloudflared -n 50`. A wrong or revoked token is the
usual cause; re-run `sudo cloudflared service install <token>` with a fresh one.

**`{"setup":true}` after you thought the account existed.** The server is reading a different data
directory than you expect, usually because Compose ran from another directory. Compare
`docker compose exec orbit ls -la /data` with `ls -la ~/orbit/data`.

**Android cannot connect (`cleartext not permitted`).** Use the HTTPS hostname, as in
[deploy.md troubleshooting](deploy.md#troubleshooting).

**Certificate error in a browser.** The hostname is deeper than one level below the zone (see
[Edge settings](#7-edge-settings-that-matter)), or the public hostname was added without a DNS record.
Tunnels normally create the record themselves.

**`rate_limited` with HTTP 429.** The sign-in lockout described above, not a proxy problem.
