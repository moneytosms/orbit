# Your first ten minutes

For someone who has just found Orbit and wants a running server with one task synced to their laptop.

## Before you start

- Docker with the Compose plugin: `docker compose version` must print a version.
- A laptop or desktop with a terminal. The server can run on the same machine to begin with.

## Steps

1. Start the server. This downloads the Compose file and starts Orbit on `127.0.0.1:8080`:

   ```sh
   mkdir orbit && cd orbit
   curl -fsSLO https://raw.githubusercontent.com/moneytosms/orbit/main/deploy/compose.yaml
   docker compose up -d
   docker compose logs orbit | grep "setup code"
   ```

   The last line prints the one-time setup code:

   ```text
   First run: open the app and enter setup code <code>
   ```

2. Create your account. Open <http://127.0.0.1:8080> in a browser, enter the setup code, and choose a password of at least 8 characters. Details are in [Create your account](deploy.md#create-your-account).

3. Install the CLI. On Linux, one command puts `orbit` in `~/.local/bin`:

   ```sh
   curl -fsSL https://raw.githubusercontent.com/moneytosms/orbit/main/scripts/install-cli.sh | sh
   ```

   Other systems: see [cli-install.md](cli-install.md#1-install).

4. Sign in and add a task. Use the server address from step 1:

   ```sh
   orbit login http://127.0.0.1:8080
   orbit add "buy milk friday 5pm p1"
   orbit sync
   orbit ls
   ```

   `orbit login` takes the password from stdin or from `ORBIT_PASSWORD`. `orbit ls` should list the task you just added.

## Verify it worked

```sh
curl -sS http://127.0.0.1:8080/auth; echo
orbit status
```

The first command prints `{"setup":false}`, which means the account exists. `orbit status` should show the server you signed in to.

## Troubleshooting

**The setup code is missing from the log.** Restart the container with `docker compose restart orbit` and read the log again. Only the newest code works. More in [deploy.md troubleshooting](deploy.md#troubleshooting).

**`Port already in use` when the server starts.** Another process holds port 8080. Stop it, or change the host port in `compose.yaml` as described in [deploy.md troubleshooting](deploy.md#troubleshooting).

**`orbit: command not found`.** `~/.local/bin` is not on your `PATH`. Add it to your shell profile and open a new terminal.

**The task does not appear on your phone or in the web app.** Run `orbit sync` once while online. Changes made offline wait in the queue until then. See [install.md troubleshooting](install.md#8-troubleshooting).

## Next

[Install Orbit on your other devices](install.md): the web app, the Linux desktop app, Android and the CLI.
