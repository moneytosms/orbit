# Orbit

Orbit is a self-hostable personal planner. You capture tasks quickly, then organise them under Areas and Projects and tie them to Goals and Routines. A small Rust server holds the data in SQLite. The web app, a Linux desktop app, an Android app and a CLI sync with it, and each keeps an offline copy.

**Install:** set up the server with [docs/deploy.md](docs/deploy.md), then install the clients (web, Linux, Android, CLI) with [docs/install.md](docs/install.md).

## Features

Scope and status are tracked in [docs/scope.md](docs/scope.md) (still marked draft). Some items there are planned, not built.

- Areas, Projects, and an Inbox; tasks with subtasks, priorities, labels, due dates and recurrence.
- Goals with progress, and Routines with check-ins and streaks, kept separate from recurring tasks. See [GLOSSARY.md](GLOSSARY.md).
- Todoist-style quick-add syntax, shared by the UI and the CLI.
- Offline use with a queued, idempotent sync.
- Reminders on Android and Linux desktop, scheduled on the device.
- Android widgets (quick add, task list, routines, goals) and a share target.
- Single-user login with password, optional TOTP, and revocable Devices.
- Todoist import (`orbit import todoist`), an MCP server (`orbit mcp`), and server backup/restore commands.

Status: all v1 features listed above are built. Android is verified on an emulator; the real-device checklist ([docs/android-checklist.md](docs/android-checklist.md)) is still pending.

## Architecture

Rust (axum) server with SQLite as the source of truth. Clients sync through client-generated ids, an idempotent command queue, sync-token deltas and per-field last-writer-wins. One Svelte 5 UI ships as a PWA and inside Tauri 2 (Linux, Android). Shared logic lives in `orbit-core` and runs natively and as wasm. See [docs/adr/0001-stack.md](docs/adr/0001-stack.md).

| Path | Role |
| --- | --- |
| `crates/orbit-core` | parser, recurrence, reminders, Stardust |
| `crates/orbit-wasm` | wasm wrapper over core |
| `crates/orbit-client` | sync client, SQLite replica, outbox |
| `crates/orbit-server` | axum + SQLite server |
| `crates/orbit-cli` | the `orbit` CLI |
| `app/` | Svelte UI; `app/src-tauri` is the Tauri app |

## Install and run

### Server

Environment variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `ORBIT_ADDR` | `127.0.0.1:8080` | listen address |
| `ORBIT_DATA` | `data` | data directory (`orbit.db`, `backups/`) |
| `ORBIT_WEB` | unset | directory of the built web app to serve |

Docker (image is built in `Dockerfile`; released as `ghcr.io/<owner>/<repo>`, see [docs/releasing.md](docs/releasing.md)):

```sh
docker run --rm -p 8080:8080 -v orbit-data:/data ghcr.io/<owner>/<repo>:<version>
```

From source (no web UI unless you build it and set `ORBIT_WEB`):

```sh
cargo run --release -p orbit-server
```

First run: with no user yet, the server prints `First run: open the app and enter setup code <code>` to stderr (check `docker logs`). Open the app and enter the code to create the account. Lost password: `orbit-server reset-password` (reads the new password from stdin; signs out every Device). Other subcommands: `orbit-server backup <dir>` and `orbit-server restore <snapshot>` (stop the server first). The server also snapshots daily into `ORBIT_DATA/backups`.

Put it behind HTTPS before exposing it. Tailscale serve and Caddy with a domain are both covered in [docs/deploy.md](docs/deploy.md).

### Web / PWA

Open the server's address in a browser; it can be installed as a PWA. Browser reminders only fire while a tab is open. Steps: [docs/install.md](docs/install.md#2-web--pwa-any-desktop-or-phone-browser).

### Linux desktop

Tauri app `orbit-app` with tray, quick-capture hotkey and reminder notifications. Released as an AppImage; an AUR package (`orbit-bin`) is prepared in `packaging/aur`. Install steps: [docs/install.md](docs/install.md#3-linux-desktop). Details and flags: [docs/linux-desktop.md](docs/linux-desktop.md).

### Android

Android 12+ (minSdk 31). Download `Orbit-<version>-android.apk` from the GitHub Release and sideload it (Obtainium can track releases). On first launch the app shows a page asking for your server URL, saves it, then opens it. The release APK only talks to HTTPS servers. Install steps, widgets and battery settings: [docs/install.md](docs/install.md#4-android). The manual test plan is [docs/android-checklist.md](docs/android-checklist.md).

Status: verified on an emulator only. It has not yet been tested on real devices, so OEM battery behaviour and reminder reliability are unconfirmed.

### CLI

`orbit` signs in to a server, keeps a local replica and queues changes offline.

```sh
cargo install --path crates/orbit-cli
orbit login https://orbit.example.com   # password via ORBIT_PASSWORD or stdin
orbit add "buy milk"
orbit ls
orbit done 1
orbit sync
orbit guide                              # short reference
```

Full reference: [docs/cli.md](docs/cli.md), man pages in `docs/man`. Install and MCP setup: [docs/install.md](docs/install.md#5-cli-and-mcp).

## Development

Toolchain (Rust, JDK, Gradle, Android SDK) is expected under `~/.toolchain`; `rust-toolchain.toml` pins stable plus the wasm32 target. You also need `just`, `pnpm` and `wasm-pack`.

```sh
just check            # the gate: svelte-check, vitest, web build, cargo fmt/clippy/test, wasm build
just hooks            # once per clone; pre-push runs `just check`
just fmt src/lib/sync.ts   # cargo fmt + prettier on named app files (paths relative to app/)
just server           # server on 127.0.0.1:8080, data in ./data
just web              # Vite dev server, proxies /sync to the server
```

See [CLAUDE.md](CLAUDE.md) for more detail.

## Release

Releases are lockstep semver cut with `just release X.Y.Z`. See [docs/releasing.md](docs/releasing.md).

## License

GNU AGPL v3. See [LICENSE](LICENSE).
