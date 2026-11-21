# Install and sign in to the Orbit CLI

The `orbit` command runs on your own device (laptop, desktop, server shell). It signs in to your Orbit
server, keeps a local copy of your data and queues changes while you are offline. You do not need the
server, a checkout of this repository or a Rust toolchain.

You need a running server and its address. Set one up with [deploy.md](deploy.md); the address is the
`https://` URL from [Connect your clients](deploy.md#5-connect-your-clients).

## 1. Install

### Linux (x86_64 and aarch64)

One command. It picks the right release asset for your CPU, checks it against the release's
`SHA256SUMS` and puts `orbit` in `~/.local/bin`:

```sh
curl -fsSL https://raw.githubusercontent.com/moneytosms/orbit/main/scripts/install-cli.sh | sh
```

Set `ORBIT_BIN=/usr/local/bin` (with `sudo sh`) to install somewhere else, or `ORBIT_VERSION=0.1.2` to
pin a version. Make sure the install directory is on your `PATH`.

Prefer to do it by hand? Download `orbit-<version>-linux-<arch>.tar.gz` and `SHA256SUMS` from the
[latest release](https://github.com/moneytosms/orbit/releases/latest), then:

```sh
sha256sum -c SHA256SUMS --ignore-missing
tar -xzf orbit-<version>-linux-<arch>.tar.gz
install -m 755 orbit-<version>-linux-<arch>/orbit ~/.local/bin/orbit
```

The tarball also holds the man pages in `man/`.

### macOS, Windows and other CPUs

There is no prebuilt binary yet. Build it from a checkout; this needs the Rust toolchain
(<https://rustup.rs>):

```sh
git clone https://github.com/moneytosms/orbit && cd orbit
cargo install --path crates/orbit-cli --locked
```

The binary lands in `~/.cargo/bin`.

## 2. Sign in

```sh
orbit login https://orbit.example.com
```

It reads your password from the `ORBIT_PASSWORD` environment variable or from stdin, and signs in. Add
`--totp <code>` if your account has two-factor codes. Each sign-in is a **Device** on your account; the
default name is `orbit-cli`, and `--name` changes it:

```sh
echo "$PASSWORD" | orbit login https://orbit.example.com --name laptop
```

Settings, Account and devices in the web app lists every Device and can revoke one.

## 3. Use it

```sh
orbit add "buy milk friday 5pm p1 @Home"   # same quick-add syntax as the apps
orbit ls
orbit done 1
orbit sync                                  # push queued changes and pull new ones
orbit guide                                 # command and exit-code reference
```

Run `orbit sync` by hand before you shut a laptop or after being offline. Sign out of this device with `orbit logout`; changes not yet synced
wait for your next login. The full command list is in [cli.md](cli.md), and man pages are in
`docs/man/`.

## 4. Update

Run the install command again; it replaces the binary with the latest release. Run `orbit sync` first
so no queued changes are left behind. A build-from-source install updates with `git pull` and
`cargo install --path crates/orbit-cli --locked --force`.

## 5. Use it from AI tools

`orbit mcp` runs the same commands as an MCP server over stdio. Sign in first, then for Claude Code:

```sh
claude mcp add orbit -- orbit mcp
```
