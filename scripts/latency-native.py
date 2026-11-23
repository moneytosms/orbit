#!/usr/bin/env python3
"""Measure the Desktop native replica against CLI on disposable endpoints."""

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
import pathlib
import secrets
import re
import shutil
import sqlite3
import sys
import subprocess
import tempfile
import threading
import time
import urllib.request

sys.dont_write_bytecode = True

import latency as harness

ROOT = pathlib.Path(__file__).resolve().parents[1]
SAMPLES = 100
GAP_MS = 2_300
TRACE = re.compile(r"native_sync_trace device=([^ ]+) started_at=(\d+) completed_at=(\d+)")


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def cli_run(binary, args, env):
    completed = subprocess.run([str(binary), "--json", *args], cwd=ROOT, env=env,
                               capture_output=True, text=True, check=True)
    return json.loads(completed.stdout.strip().splitlines()[-1])


def row(db_path, task_id):
    try:
        with sqlite3.connect(f"file:{db_path}?mode=ro", uri=True, timeout=1) as db:
            value = db.execute("SELECT data FROM rows WHERE id = ?", (task_id,)).fetchone()
    except sqlite3.OperationalError as error:
        if str(error) in ("unable to open database file", "no such table: rows"):
            return None
        raise
    return json.loads(value[0]) if value else None


class CliObserver:
    def __init__(self, binary, client):
        self.binary = binary
        self.client = client
        self.pending = {}
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.error = None

    def start(self):
        self.thread.start()

    def run(self):
        db = self.client["dir"] / "orbit.db"
        while not self.stop.is_set():
            started = harness.now_ms()
            try:
                cli_run(self.binary, ["sync"], self.client["env"])
                completed = harness.now_ms()
                with sqlite3.connect(f"file:{db}?mode=ro", uri=True, timeout=1) as conn:
                    ids = [item for item in self.pending
                           if conn.execute("SELECT 1 FROM rows WHERE id = ?", (item,)).fetchone()]
                for item in ids:
                    sample = self.pending.pop(item)
                    sample["observed_at"] = completed
                    sample["receiver_sync_start"] = started
            except Exception:
                self.error = "CLI receiver poll failed"
                return
            self.stop.wait(0.25)


def desktop_environment(base, endpoint_url, data_home):
    home = base / "desktop-home"
    paths = {name: base / name for name in (
        "xdg-config", "xdg-cache", "xdg-runtime",
    )}
    for path in [home, *paths.values()]:
        path.mkdir(parents=True, exist_ok=True, mode=0o700)
        path.chmod(0o700)
    dbus_tmp = pathlib.Path(tempfile.mkdtemp(
        prefix="d14-", dir=os.environ.get("TMPDIR", "/tmp")
    ))
    env = {
        "PATH": os.environ.get("PATH", os.defpath),
        "HOME": str(home),
        "XDG_CONFIG_HOME": str(paths["xdg-config"]),
        "XDG_DATA_HOME": str(data_home),
        "XDG_CACHE_HOME": str(paths["xdg-cache"]),
        "XDG_RUNTIME_DIR": str(paths["xdg-runtime"]),
        "TMPDIR": str(dbus_tmp),
        "GDK_BACKEND": "x11",
        "WEBKIT_DISABLE_DMABUF_RENDERER": "1",
        "NO_AT_BRIDGE": "1",
        "ORBIT_SYNC_TRACE": "1",
        # Only the disposable origin and Quick Tunnel bypass this dead proxy.
        "NO_PROXY": ",".join(("127.0.0.1", "localhost", ".trycloudflare.com")),
        "no_proxy": ",".join(("127.0.0.1", "localhost", ".trycloudflare.com")),
        "HTTP_PROXY": "http://127.0.0.1:9",
        "HTTPS_PROXY": "http://127.0.0.1:9",
        "ALL_PROXY": "http://127.0.0.1:9",
        "http_proxy": "http://127.0.0.1:9",
        "https_proxy": "http://127.0.0.1:9",
        "all_proxy": "http://127.0.0.1:9",
    }
    return env


def start_xserver(base):
    binary = os.environ.get("ORBIT_XVFB") or shutil.which("Xvfb")
    if not binary:
        raise RuntimeError("Xvfb is required for an isolated Desktop runtime")
    if not pathlib.Path(binary).is_file() or not os.access(binary, os.X_OK):
        raise RuntimeError("ORBIT_XVFB must name an executable isolated Xvfb binary")
    read_fd, write_fd = os.pipe()
    log = (base / "xvfb.log").open("w")
    process = subprocess.Popen(
        [binary, "-displayfd", str(write_fd), "-screen", "0", "1280x900x24",
         "-nolisten", "tcp", "-noreset"],
        pass_fds=(write_fd,), stdout=subprocess.DEVNULL, stderr=log,
    )
    os.close(write_fd)
    try:
        import select
        ready, _, _ = select.select([read_fd], [], [], 10)
        if not ready:
            raise RuntimeError("isolated Xvfb did not become ready")
        display = os.read(read_fd, 32).decode().strip()
        if not display.isdecimal() or process.poll() is not None:
            raise RuntimeError("isolated Xvfb did not return a display number")
        return process, f":{display}"
    except Exception:
        process.terminate()
        process.wait(timeout=5)
        raise
    finally:
        os.close(read_fd)
        log.close()


def native_device(local_url, account):
    result, _, _ = harness.request(
        f"{local_url}/devices", account["token"], {"name": "Issue14 native Desktop"}
    )
    return result["device"], result["token"]


def write_native_token(data_home, server, device, token):
    orbit_dir = data_home / "orbit"
    orbit_dir.mkdir(parents=True, mode=0o700)
    (orbit_dir / "server").write_text(server)
    token_path = orbit_dir / "token"
    fd = os.open(token_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump({"server": server, "device": device, "token": token}, stream)
    (orbit_dir / "orbit.db").symlink_to("replica.db")
    return orbit_dir


def native_syncs(app_log):
    app_log.flush()
    app_log.seek(0)
    return [(m.group(1), int(m.group(2)), int(m.group(3)))
            for m in TRACE.finditer(app_log.read())]


def event_for(events, device, task_id, key):
    return next((event for event in events if event["device"] == device
                 and task_id in event[key]), None)


def sync_for(syncs, device, timestamp):
    return next(((start, end) for target, start, end in syncs
                 if target == device and start <= timestamp <= end), None)


def read_server_events(server_log):
    server_log.flush()
    server_log.seek(0)
    events = []
    for line in server_log:
        marker = "sync_trace "
        if marker not in line:
            continue
        try:
            events.append(json.loads(line.split(marker, 1)[1]))
        except json.JSONDecodeError:
            pass
    return events


def stats(values):
    return harness.summarize([{"value": value} for value in values], "value")


def print_pair(endpoint, rows):
    print(f"native {endpoint} Desktop↔CLI samples={len(rows)} "
          f"directions={sum(r['direction'] == 'Desktop→CLI' for r in rows)}/"
          f"{sum(r['direction'] == 'CLI→Desktop' for r in rows)}", flush=True)
    for key in ("observed_ms", "enqueue_to_upload_ms", "upload_to_server_ms",
                "server_apply_ms", "server_applied_to_fetch_ms", "fetch_to_observed_ms",
                "poll_wait_ms"):
        s = stats([row[key] for row in rows])
        print(f"native {endpoint} Desktop↔CLI {key}: p50={s['p50']:.2f}ms "
              f"p95={s['p95']:.2f}ms max={s['max']:.2f}ms", flush=True)
    for direction in ("Desktop→CLI", "CLI→Desktop"):
        group = [r for r in rows if r["direction"] == direction]
        s = stats([row["observed_ms"] for row in group])
        print(f"native {endpoint} Desktop↔CLI {direction} observed_ms: "
              f"p50={s['p50']:.2f}ms p95={s['p95']:.2f}ms max={s['max']:.2f}ms", flush=True)


def run_android(wrapper, apk, endpoint, url, base, account, area_id,
                cli_binary, cli_clients, pairs, chromium, playwright, node, probe_ids):
    output = base / f"{endpoint}-android-results.json"
    config = {
        "endpointName": endpoint, "url": url, "password": account["password"],
        "areaId": area_id, "areaName": harness.AREA_NAME, "samples": SAMPLES,
        "pairs": pairs, "output": str(output),
        "androidLog": str(base / f"{endpoint}-android-private.log"),
        "profileRoot": str(base / endpoint / "android-web-profiles"),
        "serverLog": str(base / "server.log"), "cliBin": str(cli_binary),
        "cliProfiles": {name: {"dir": str(client["dir"]), "env": client["env"]}
                        for (client_endpoint, name), client in cli_clients.items()
                        if client_endpoint == endpoint},
        "apkSha256": sha256(apk),
        "root": str(ROOT), "chromium": chromium, "playwright": playwright, "node": node,
    }
    config_path = base / f"{endpoint}-android-config.json"
    fd = os.open(config_path, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w") as stream:
        json.dump(config, stream)
    callback = ROOT / "scripts" / "latency-native-android.py"
    env = {"PATH": os.environ.get("PATH", os.defpath),
           "ORBIT_NATIVE_CONFIG": str(config_path),
           "ORBIT_AVD_LOG": os.environ.get(
               "ORBIT_AVD_LOG",
               str(pathlib.Path(os.environ.get("TMPDIR", "/tmp")) / f"14-{endpoint}-avd.log"),
           )}
    completed = subprocess.run([str(wrapper), str(apk), str(callback)], cwd=ROOT,
                               env=env, capture_output=True, text=True)
    if completed.stdout:
        print(completed.stdout.rstrip(), flush=True)
    if output.is_file():
        probe_ids.extend(json.loads(output.read_text()).get("probes", []))
    if completed.returncode:
        diagnostics = completed.stderr.strip().splitlines()
        safe = next((line for line in reversed(diagnostics)
                     if line.startswith(("RuntimeError: ", "SystemExit: "))), "")
        private_values = [account.get("password"), account.get("token")]
        for client in cli_clients.values():
            database = client["dir"] / "orbit.db"
            private_values.extend(harness.meta(database, key) for key in ("auth", "token"))
        for secret in private_values:
            if secret and len(secret) >= 8:
                safe = safe.replace(secret, "[redacted]")
        print(f"android_runner_exit={completed.returncode}; detail={safe or 'private diagnostics unavailable'}",
              flush=True)
        raise RuntimeError("isolated Android harness failed; private diagnostics suppressed")
    if not output.is_file():
        raise RuntimeError("Android harness produced no measurement result")


def measure_endpoint(endpoint, url, base, account, area_id, desktop_binary,
                     cli_binary, server_log, samples):
    profile = base / endpoint / "desktop-profile"
    data_home = profile / "data"
    env = desktop_environment(base, url, data_home)
    native_id, native_token = native_device(account["local_url"], account)
    orbit_dir = write_native_token(data_home, url, native_id, native_token)
    desktop_log = (base / f"{endpoint}-desktop.log").open("w+")
    launcher = os.environ.get("ORBIT_XVFB") or shutil.which("Xvfb")
    bus = shutil.which("dbus-run-session")
    if not launcher or not bus:
        raise RuntimeError("private Xvfb and D-Bus runtimes are required")
    xserver, display = start_xserver(base)
    env["DISPLAY"] = display
    bus_config = base / "dbus-session.conf"
    bus_config.write_text(
        "<busconfig><type>session</type>"
        f"<listen>unix:tmpdir={env['TMPDIR']}</listen>"
        "<auth>EXTERNAL</auth><policy context=\"default\">"
        "<allow send_destination=\"*\"/><allow receive_sender=\"*\"/>"
        "<allow own=\"*\"/></policy></busconfig>"
    )
    process = subprocess.Popen(
        [bus, f"--config-file={bus_config}", "--", str(desktop_binary), "--hidden", "--server", url],
        cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=desktop_log,
    )
    try:
        clients = harness.make_clients([(endpoint, url)], base, cli_binary, account["password"])
        cli_a, cli_b = clients[(endpoint, "a")], clients[(endpoint, "b")]
        desktop_db = orbit_dir / "replica.db"
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            exit_status = process.poll()
            if exit_status is not None:
                print(f"native {endpoint} startup_cause=desktop_exited_before_replica_ready exit_status={exit_status}",
                      flush=True)
                raise RuntimeError("baseline Desktop shell exited before native replica readiness")
            try:
                area = row(desktop_db, area_id)
            except sqlite3.Error:
                print(f"native {endpoint} startup_cause=replica_database_error exit_status=running",
                      flush=True)
                raise RuntimeError("Desktop replica database startup failed") from None
            if area and area.get("kind") == "area":
                break
            time.sleep(0.1)
        else:
            print(f"native {endpoint} startup_cause=replica_ready_timeout exit_status=running",
                  flush=True)
            raise RuntimeError("Desktop native replica did not fetch the disposable probe Area")

        observer = CliObserver(cli_binary, cli_b)
        observer.start()
        pending = []
        initial_delay = secrets.randbelow(60_000)
        print(f"native {endpoint} Desktop↔CLI randomized_start_delay_ms={initial_delay}; normal_poll_ms=60000",
              flush=True)
        time.sleep(initial_delay / 1000)
        next_slot = time.monotonic()
        for index in range(samples):
            time.sleep(max(0, next_slot - time.monotonic()))
            desktop_to_cli = index % 2 == 0
            sender = orbit_dir if desktop_to_cli else cli_a["dir"]
            sender_env = {**env, "ORBIT_DIR": str(orbit_dir)} if desktop_to_cli else cli_a["env"]
            title = f"issue14-native-{endpoint}-desktop-{index:03d} @{harness.AREA_NAME}"
            queued_at = harness.now_ms()
            result = cli_run(cli_binary, ["add", title], sender_env)
            task_id = str(result["id"])
            task = row(desktop_db if desktop_to_cli else cli_a["dir"] / "orbit.db", task_id)
            if not task or task.get("parent_id") != area_id:
                raise RuntimeError("native probe was not queued in the dedicated Area")
            sample = {
                "direction": "Desktop→CLI" if desktop_to_cli else "CLI→Desktop",
                "id": task_id,
                "enqueue_at": task["created_at"] or queued_at,
                "sender_device": native_id if desktop_to_cli else harness.json.loads(
                    harness.meta(cli_a["dir"] / "orbit.db", "hlc"))["device"],
                "receiver_device": harness.json.loads(
                    harness.meta(cli_b["dir"] / "orbit.db", "hlc"))["device"] if desktop_to_cli else native_id,
            }
            if desktop_to_cli:
                observer.pending[task_id] = sample
            else:
                # The visible CLI send is immediate; the desktop's native replica keeps its 60 s loop.
                push_start = harness.now_ms()
                cli_run(cli_binary, ["sync"], cli_a["env"])
                sample["cli_sync_start"] = push_start
            pending.append(sample)
            next_slot += GAP_MS / 1000
            if (index + 1) % 20 == 0:
                print(f"native {endpoint} Desktop↔CLI queued={index + 1}/{samples}", flush=True)
        deadline = time.monotonic() + 180
        while time.monotonic() < deadline and any("observed_at" not in sample for sample in pending):
            if observer.error:
                raise RuntimeError(observer.error)
            desktop_syncs = native_syncs(desktop_log)
            events = read_server_events(server_log)
            for sample in pending:
                if sample["direction"] == "CLI→Desktop" and "observed_at" not in sample:
                    if not row(desktop_db, sample["id"]):
                        continue
                    fetched = event_for(events, native_id, sample["id"], "delta")
                    interval = sync_for(desktop_syncs, native_id, fetched["received_at"]) if fetched else None
                    if interval:
                        sample["receiver_sync_start"], sample["observed_at"] = interval
            time.sleep(0.1)
        missing = sum("observed_at" not in sample for sample in pending)
        if missing:
            raise RuntimeError(f"Desktop↔CLI receiver timed out with {missing} samples outstanding")
        desktop_log.flush()
        desktop_syncs = native_syncs(desktop_log)
        events = read_server_events(server_log)
        rows = []
        for sample in pending:
            applied = event_for(events, sample["sender_device"], sample["id"], "commands")
            fetched = event_for(events, sample["receiver_device"], sample["id"], "delta")
            if not applied or not fetched:
                raise RuntimeError("native sync trace could not correlate a probe")
            sender_interval = (sync_for(desktop_syncs, native_id, applied["received_at"])
                               if sample["direction"] == "Desktop→CLI" else None)
            sender_sync = sender_interval[0] if sender_interval else sample.get("cli_sync_start")
            receiver_sync = sample.get("receiver_sync_start")
            if sender_sync is None or receiver_sync is None:
                raise RuntimeError("native poll trace was unavailable for a measured sample")
            sample.update({
                "enqueue_to_upload_ms": max(0, sender_sync - sample["enqueue_at"]),
                "upload_to_server_ms": max(0, applied["received_at"] - sender_sync),
                "server_apply_ms": max(0, applied["applied_at"] - applied["received_at"]),
                "server_applied_to_fetch_ms": max(0, fetched["received_at"] - applied["applied_at"]),
                "poll_wait_ms": max(0, receiver_sync - applied["applied_at"]),
                "fetch_to_observed_ms": max(0, sample["observed_at"] - fetched["applied_at"]),
                "observed_ms": max(0, sample["observed_at"] - sample["enqueue_at"]),
            })
            rows.append(sample)
        print_pair(endpoint, rows)
        return [sample["id"] for sample in pending], native_id
    finally:
        if "observer" in locals():
            observer.stop.set()
            observer.thread.join(timeout=3)
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
        if xserver.poll() is None:
            xserver.terminate()
            try:
                xserver.wait(timeout=5)
            except subprocess.TimeoutExpired:
                xserver.kill()
                xserver.wait()
        desktop_log.close()
        shutil.rmtree(env["TMPDIR"], ignore_errors=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--preflight", action="store_true",
                        help="start the disposable web-serving origin and check its root; no samples")
    parser.add_argument("--endpoint", choices=("local", "quick-tunnel", "both"), default="local")
    parser.add_argument("--samples", type=int, default=SAMPLES)
    parser.add_argument("--platform", choices=("desktop", "android"), default="desktop")
    parser.add_argument("--desktop-binary", type=pathlib.Path)
    parser.add_argument("--desktop-sha256")
    parser.add_argument("--android-apk", type=pathlib.Path)
    parser.add_argument("--android-sha256")
    parser.add_argument("--android-host-wrapper", type=pathlib.Path)
    parser.add_argument("--pair", choices=("android-cli", "android-web", "both"), default="both")
    args = parser.parse_args()
    commit = subprocess.check_output(["git", "-C", str(ROOT), "rev-parse", "HEAD"], text=True).strip()
    if args.platform == "desktop" and not args.preflight:
        if not args.desktop_binary or not args.desktop_sha256:
            raise SystemExit("Desktop runs require --desktop-binary and --desktop-sha256")
        artifact, expected_hash = args.desktop_binary.resolve(strict=True), args.desktop_sha256
    else:
        if not args.android_apk or not args.android_sha256 or not args.android_host_wrapper:
            raise SystemExit("Android runs require --android-apk, --android-sha256, and --android-host-wrapper")
        artifact, expected_hash = args.android_apk.resolve(strict=True), args.android_sha256
    digest = sha256(artifact)
    if digest != expected_hash:
        raise SystemExit(f"{args.platform} artifact SHA-256 does not match the recorded hash")
    if args.platform == "desktop":
        with artifact.open("rb") as stream:
            if b"native_sync_trace" not in stream.read():
                raise SystemExit("Desktop artifact lacks opt-in native_sync_trace instrumentation")
    if args.check:
        required = ["flock"]
        required += ["dbus-run-session"] if args.platform == "desktop" else ["node", "mise", "cloudflared"]
        missing = [tool for tool in required if shutil.which(tool) is None]
        xvfb = os.environ.get("ORBIT_XVFB")
        if args.platform == "desktop" and not xvfb and shutil.which("Xvfb") is None:
            missing.append("Xvfb (set ORBIT_XVFB or PATH)")
        if xvfb and (not pathlib.Path(xvfb).is_file() or not os.access(xvfb, os.X_OK)):
            missing.append(f"executable ORBIT_XVFB ({xvfb})")
        target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "release"
        missing += [str(target / name) for name in ("orbit-server", "orbit")
                    if not (target / name).is_file()]
        if args.platform == "android" and not args.android_host_wrapper.is_file():
            missing.append(str(args.android_host_wrapper))
        playwright = node = chromium = None
        if args.platform == "android":
            mise_env = {**os.environ, "MISE_QUIET": "1"}
            try:
                playwright = subprocess.check_output(["mise", "which", "playwright"], text=True,
                                                     stderr=subprocess.DEVNULL, env=mise_env).strip()
                node = subprocess.check_output(["mise", "which", "node"], text=True,
                                               stderr=subprocess.DEVNULL, env=mise_env).strip()
            except subprocess.CalledProcessError:
                pass
            chromium = shutil.which("chromium")
            if not playwright or not pathlib.Path(playwright).is_file():
                missing.append("Playwright via mise")
            if not node or not pathlib.Path(node).is_file():
                missing.append("Node via mise")
            if not chromium:
                missing.append("Chromium")
        print(f"source_commit={commit}")
        print(f"{args.platform}_sha256={digest}")
        print(f"target_dir={target}")
        if missing:
            raise SystemExit("missing native prerequisites: " + ", ".join(missing))
        print("check=ok")
        return
    if args.preflight:
        with tempfile.TemporaryDirectory(prefix="orbit-issue14-preflight-",
                                         dir=os.environ.get("TMPDIR", "/dev/shm")) as scratch:
            server = None
            log = None
            try:
                server, log, data_dir, url, _ = harness.start_server(
                    pathlib.Path(scratch), serve_web=True
                )
                with urllib.request.urlopen(f"{url}/", timeout=5) as response:
                    root = response.read(4096).decode("utf-8", "replace")
                    if response.status != 200 or "<html" not in root.lower():
                        raise RuntimeError("disposable web frontend root did not return HTML")
                print("preflight=ok; server_ready=true; web_root=200; samples=0")
            finally:
                if server and server.poll() is None:
                    server.terminate()
                    try:
                        server.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        server.kill()
                        server.wait()
                if log:
                    log.close()
        return
    if args.samples != SAMPLES:
        raise SystemExit("native runtime measurements require exactly 100 samples")
    if args.endpoint not in ("local", "quick-tunnel", "both"):
        raise SystemExit("invalid endpoint")
    target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "release"
    cli_binary = target / "orbit"
    server_binary = target / "orbit-server"
    cli_version = subprocess.check_output([str(cli_binary), "--version"], text=True).strip()
    print(f"run_started_utc={datetime.now(timezone.utc).isoformat(timespec='seconds')}")
    print(f"source_commit={commit}; cli_version={cli_version}")
    print(f"cli_sha256={sha256(cli_binary)}; server_sha256={sha256(server_binary)}")
    print(f"native_artifact_sha256={digest}")
    playwright = node = chromium = None
    if args.platform == "android":
        mise_env = {**os.environ, "MISE_QUIET": "1"}
        playwright = subprocess.check_output(["mise", "which", "playwright"], text=True,
                                             stderr=subprocess.DEVNULL, env=mise_env).strip()
        node = subprocess.check_output(["mise", "which", "node"], text=True,
                                       stderr=subprocess.DEVNULL, env=mise_env).strip()
        chromium = shutil.which("chromium")
        if not chromium:
            raise SystemExit("Chromium missing")
    scratch_parent = pathlib.Path(os.environ.get("TMPDIR", "/dev/shm"))
    with tempfile.TemporaryDirectory(prefix="orbit-issue14-native-", dir=scratch_parent) as scratch:
        base = pathlib.Path(scratch)
        server = tunnel = None
        server_log = tunnel_log = None
        account = None
        area_id = None
        server_token = None
        probe_ids = []
        try:
            server, server_log, _, local_url, cli_binary = harness.start_server(
                base, serve_web=True
            )
            password = harness.secrets.token_urlsafe(24)
            account = harness.setup_account(local_url, server_log, password)
            account["local_url"] = local_url
            account["password"] = password
            area_id, server_token = harness.create_probe_area(local_url, account, base)
            endpoints = [("local", local_url)] if args.endpoint in ("local", "both") else []
            if args.endpoint in ("quick-tunnel", "both"):
                tunnel, tunnel_log, tunnel_url = harness.start_quick_tunnel(base, local_url)
                endpoints.append(("quick-tunnel", tunnel_url))
            for endpoint, url in endpoints:
                if args.platform == "desktop":
                    print(f"endpoint={endpoint} url={url} samples=100 pair=Desktop↔CLI", flush=True)
                    ids, _ = measure_endpoint(endpoint, url, base, account, area_id,
                                              artifact, cli_binary, server_log, args.samples)
                    probe_ids.extend(ids)
                else:
                    clients = harness.make_clients([(endpoint, url)], base, cli_binary, password)
                    pairs = (["Android↔CLI", "Android↔Web"] if args.pair == "both"
                             else ["Android↔CLI"] if args.pair == "android-cli"
                             else ["Android↔Web"])
                    print(f"endpoint={endpoint} url={url} samples=100 pairs={args.pair}", flush=True)
                    run_android(args.android_host_wrapper.resolve(), artifact,
                                endpoint, url, base, account, area_id,
                                                 cli_binary, clients, pairs, chromium, playwright, node,
                                probe_ids)
        finally:
            if account and area_id:
                try:
                    cleaned = harness.cleanup(local_url, account, server_token,
                                              account["device"], probe_ids, area_id)
                    print(f"cleanup=explicitly trashed {cleaned - 1} native probe tasks and Area", flush=True)
                except Exception:
                    print("cleanup=failed; disposable database removal will follow", flush=True)
            if tunnel:
                if tunnel.poll() is None:
                    tunnel.terminate()
                    try:
                        tunnel.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        tunnel.kill()
                        tunnel.wait()
            if server:
                if server.poll() is None:
                    server.terminate()
                    try:
                        server.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        server.kill()
                        server.wait()
            if tunnel_log:
                tunnel_log.close()
            if server_log:
                server_log.close()
        print(f"cleanup=private {args.platform} profiles, disposable database, and tunnel scratch removed", flush=True)


if __name__ == "__main__":
    main()
