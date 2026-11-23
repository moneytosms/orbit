#!/usr/bin/env python3
"""Measure Orbit sync latency against fresh local and optional Quick Tunnel endpoints."""

import argparse
import json
import os
import pathlib
import re
import secrets
import shutil
import socket
import sqlite3
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid

ROOT = pathlib.Path(__file__).resolve().parents[1]
SAMPLES = 100
POLL_MS = 250
AREA_NAME = "Issue14Probes"


def now_ms():
    return time.time_ns() // 1_000_000


def request(url, token=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    headers = {"Content-Type": "application/json"} if data is not None else {}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(url, data=data, headers=headers)
    start = time.monotonic_ns()
    with urllib.request.urlopen(req, timeout=15) as response:
        content = response.read()
    end = time.monotonic_ns()
    return (json.loads(content) if content else None), start, end


class Poller:
    def __init__(self, url, token, change_token, cli_bin, client=None):
        self.url = url
        self.auth = token
        self.change_token = change_token
        self.cli_bin = cli_bin
        self.client = client
        self.phase_ms = secrets.randbelow(POLL_MS)
        self.target = None
        self.found = threading.Event()
        self.stop = threading.Event()
        self.error = None
        self.result = None
        self.thread = threading.Thread(target=self.run, daemon=True)

    def start(self):
        self.thread.start()

    def run(self):
        next_poll = time.monotonic() + self.phase_ms / 1000
        while not self.stop.is_set():
            delay = next_poll - time.monotonic()
            if delay > 0 and self.stop.wait(delay):
                return
            try:
                target = self.target
                if self.client:
                    start = time.monotonic_ns()
                    run_json([str(self.cli_bin), "--json", "sync"], self.client["env"])
                    end = time.monotonic_ns()
                    present = False
                    if target:
                        with sqlite3.connect(self.client["dir"] / "orbit.db") as db:
                            present = db.execute("SELECT 1 FROM rows WHERE id = ?", (target,)).fetchone() is not None
                    if target and present:
                        self.result = (start, end, now_ms())
                        self.found.set()
                        return
                else:
                    response, start, end = request(
                        f"{self.url}/sync", self.auth,
                        {"v": 1, "token": self.change_token, "commands": []},
                    )
                    self.change_token = response["token"]
                    if target and any(row.get("id") == target for row in response.get("delta", [])):
                        self.result = (start, end, now_ms())
                        self.found.set()
                        return
            except Exception as exc:
                self.error = str(exc)
                self.found.set()
                return
            next_poll += POLL_MS / 1000
            if next_poll < time.monotonic():
                next_poll = time.monotonic()

    def close(self):
        self.stop.set()
        self.thread.join(timeout=3)


def run_json(command, env):
    completed = subprocess.run(command, cwd=ROOT, env=env, check=True, text=True, capture_output=True)
    return json.loads(completed.stdout.strip().splitlines()[-1])


def meta(db_path, key):
    with sqlite3.connect(db_path) as db:
        return db.execute("SELECT value FROM meta WHERE key = ?", (key,)).fetchone()[0]


def local_task(db_path, task_id):
    with sqlite3.connect(db_path) as db:
        raw = db.execute("SELECT data FROM rows WHERE id = ?", (task_id,)).fetchone()[0]
    return json.loads(raw)


def trace_for_task(server_log, task_id, sender_device, receiver_device):
    server_log.flush()
    server_log.seek(0)
    applied = fetched = None
    for line in server_log:
        marker = "sync_trace "
        if marker not in line:
            continue
        event = json.loads(line.split(marker, 1)[1])
        if event["device"] == sender_device and task_id in event["commands"]:
            applied = event
        if event["device"] == receiver_device and task_id in event["delta"]:
            fetched = event
    return applied, fetched


def percentile(values, pct):
    values = sorted(values)
    rank = max(0, int((len(values) - 1) * pct + .5))
    return values[rank]


def summarize(samples, key):
    values = [sample[key] for sample in samples]
    return {"p50": percentile(values, .50), "p95": percentile(values, .95), "max": max(values)}


def sync_body(token, commands):
    return {"v": 1, "token": token, "commands": commands}


def make_command(device, op_type, args, counter):
    return {
        "uuid": str(uuid.uuid4()),
        "type": op_type,
        "args": args,
        "hlc": {"wall": now_ms(), "counter": counter, "device": device},
    }


def check(endpoint, web=False):
    required = ["python3", "cargo", "flock"] + (["cloudflared"] if endpoint != "local" else [])
    if web:
        required.extend(["node", "mise", "chromium"])
    missing = [tool for tool in required if shutil.which(tool) is None]
    scratch = pathlib.Path(os.environ.get("TMPDIR", "/dev/shm"))
    if not scratch.is_dir() or not os.access(scratch, os.W_OK):
        missing.append(f"writable TMPDIR ({scratch})")
    target_dir = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    print(f"target_dir={target_dir}")
    print(f"tmpdir={scratch}")
    print(f"samples_per_variant_endpoint={SAMPLES}; directions=interleaved; poll_ms={POLL_MS}; initial_phase=randomized")
    if missing:
        raise SystemExit(f"missing prerequisites: {', '.join(missing)}")
    if web:
        try:
            mise_env = {**os.environ, "MISE_QUIET": "1"}
            playwright = subprocess.check_output(
                ["mise", "which", "playwright"], text=True, stderr=subprocess.DEVNULL, env=mise_env
            ).strip()
            node = subprocess.check_output(
                ["mise", "which", "node"], text=True, stderr=subprocess.DEVNULL, env=mise_env
            ).strip()
        except subprocess.CalledProcessError as exc:
            raise SystemExit("mise could not resolve Playwright and Node") from exc
        if not pathlib.Path(playwright).is_file() or not pathlib.Path(node).is_file():
            raise SystemExit("mise which did not return Playwright and Node executables")
        print(f"playwright={playwright}")
        print(f"node={node}")
        print("web_poll_ms=60000; chromium_profiles=isolated; forced_focus_sync=disabled")
    print("check=ok")


def start_server(base, serve_web=False):
    target_dir = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    server_bin = target_dir / "release" / "orbit-server"
    cli_bin = target_dir / "release" / "orbit"
    if not server_bin.is_file() or not cli_bin.is_file():
        raise SystemExit("release binaries missing; run scripts/latency.sh")

    data_dir = base / "server"
    data_dir.mkdir()
    sock = socket.socket()
    sock.bind(("127.0.0.1", 0))
    port = sock.getsockname()[1]
    sock.close()
    env = {"ORBIT_DATA": str(data_dir), "ORBIT_ADDR": f"127.0.0.1:{port}", "ORBIT_SYNC_TRACE": "1"}
    if serve_web:
        web_dir = ROOT / "app" / "dist"
        if not (web_dir / "index.html").is_file():
            raise SystemExit(f"web bundle missing: {web_dir}/index.html")
        env["ORBIT_WEB"] = str(web_dir)
    log = (base / "server.log").open("w+")
    proc = subprocess.Popen([str(server_bin)], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=log)
    url = f"http://127.0.0.1:{port}"
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        try:
            request(f"{url}/auth")
            return proc, log, data_dir, url, cli_bin
        except (urllib.error.URLError, TimeoutError):
            if proc.poll() is not None:
                raise RuntimeError("disposable orbit-server exited before readiness")
            time.sleep(.05)
    raise RuntimeError("disposable orbit-server did not become ready")


def setup_account(local_url, server_log, password):
    server_log.flush()
    server_log.seek(0)
    log = server_log.read()
    match = re.search(r"setup code ([a-f0-9]+)", log)
    if not match:
        raise RuntimeError("temporary server setup code was unavailable in its private log")
    result, _, _ = request(f"{local_url}/auth/setup", body={
        "code": match.group(1), "password": password,
        "name": "Issue 14 disposable account", "platform": "script",
    })
    return result


def start_quick_tunnel(base, local_url):
    home = base / "cloudflared-home"
    home.mkdir()
    log = (base / "cloudflared.log").open("w+")
    cert = base / "no-origin-cert.pem"
    env = {"PATH": os.environ.get("PATH", os.defpath), "HOME": str(home)}
    proc = subprocess.Popen([
        "cloudflared", "tunnel", "--no-autoupdate", "--config", "/dev/null",
        "--origincert", str(cert), "--url", local_url,
    ], cwd=base, env=env, stdout=log, stderr=log)
    endpoint = None
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError("isolated cloudflared Quick Tunnel exited before registering")
        log.flush()
        log.seek(0)
        text = log.read()
        matches = re.findall(r"https://[a-z0-9-]+\.trycloudflare\.com", text)
        if matches:
            endpoint = matches[-1]
            try:
                request(f"{endpoint}/auth")
                return proc, log, endpoint
            except Exception:
                pass
        time.sleep(.2)
    raise RuntimeError("isolated Quick Tunnel did not become reachable")


def create_probe_area(local_url, account, base):
    setup_token = account["token"]
    device = account["device"]
    command = make_command(device, "area.create", {
        "id": str(uuid.uuid4()), "name": AREA_NAME, "color": "", "order": "",
    }, 0)
    response, _, _ = request(f"{local_url}/sync", setup_token,
                             sync_body(None, [command]))
    area_id = command["args"]["id"]
    if not any(row.get("id") == area_id for row in response.get("delta", [])):
        raise RuntimeError("disposable probe Area was not created")
    return area_id, response["token"]


def make_clients(endpoints, base, cli_bin, password):
    clients = {}
    for endpoint_name, url in endpoints:
        for name in ("a", "b"):
            client_dir = base / endpoint_name / name
            client_dir.mkdir(parents=True)
            home = base / "home"
            home.mkdir(exist_ok=True)
            env = {
                "PATH": os.environ.get("PATH", os.defpath),
                "HOME": str(home),
                "ORBIT_DIR": str(client_dir),
                "ORBIT_PASSWORD": password,
            }
            run_json([str(cli_bin), "--json", "login", url, "--name", f"issue-14-{endpoint_name}-{name}"], env)
            client = {"name": name, "url": url, "env": env, "dir": client_dir}
            run_json([str(cli_bin), "--json", "sync"], env)
            clients[(endpoint_name, name)] = client
    return clients


def measure_variant(endpoint_name, url, variant, clients, area_id, samples_count, cli_bin, server_log, include_raw):
    results = []
    change_tokens = {
        name: int(meta(clients[(endpoint_name, name)]["dir"] / "orbit.db", "token"))
        for name in ("a", "b")
    }
    auth_tokens = {
        name: meta(clients[(endpoint_name, name)]["dir"] / "orbit.db", "auth")
        for name in ("a", "b")
    }
    probe_ids = []
    for number in range(samples_count):
        sender_name, receiver_name = ("a", "b") if number % 2 == 0 else ("b", "a")
        sender = clients[(endpoint_name, sender_name)]
        receiver = clients[(endpoint_name, receiver_name)]
        cli_receiver = receiver if variant == "CLI↔CLI" else None
        poller = Poller(url, auth_tokens[receiver_name], change_tokens[receiver_name], cli_bin, cli_receiver)
        poller.start()
        try:
            title = f"issue14-{endpoint_name}-{variant}-{number:03d} @{AREA_NAME}"
            added = run_json([str(cli_bin), "--json", "add", title], sender["env"])
            task_id = added["id"]
            probe_ids.append(task_id)
            enqueue_time = local_task(sender["dir"] / "orbit.db", task_id)
            if enqueue_time.get("parent_id") != area_id:
                raise RuntimeError("probe task did not land in the dedicated Area")
            poller.target = task_id
            push_start_wall = now_ms()
            run_json([str(cli_bin), "--json", "sync"], sender["env"])
            if not poller.found.wait(20):
                raise RuntimeError(f"timeout awaiting sample {number + 1}")
            if poller.error:
                raise RuntimeError(poller.error)
            _, _, observed_wall = poller.result
            sender_device = json.loads(meta(sender["dir"] / "orbit.db", "hlc"))["device"]
            receiver_device = json.loads(meta(receiver["dir"] / "orbit.db", "hlc"))["device"]
            applied, fetched = trace_for_task(server_log, task_id, sender_device, receiver_device)
            if not applied or not fetched:
                raise RuntimeError("opt-in server trace missed a probe apply or fetch event")
            enqueue_at = enqueue_time["created_at"]
            results.append({
                "direction": f"{sender_name}→{receiver_name}",
                "phase_ms": poller.phase_ms,
                "enqueue_to_upload_ms": max(0, push_start_wall - enqueue_at),
                "upload_to_server_ms": max(0, applied["received_at"] - push_start_wall),
                "server_apply_ms": max(0, applied["applied_at"] - applied["received_at"]),
                "server_applied_to_fetch_ms": max(0, fetched["received_at"] - applied["applied_at"]),
                "fetch_to_observed_ms": max(0, observed_wall - fetched["applied_at"]),
                "upload_to_fetch_ms": max(0, observed_wall - applied["applied_at"]),
                "poll_wait_ms": max(0, fetched["received_at"] - applied["applied_at"]),
                "observed_ms": max(0, observed_wall - enqueue_at),
            })
            change_tokens[receiver_name] = (
                int(meta(receiver["dir"] / "orbit.db", "token"))
                if variant == "CLI↔CLI" else poller.change_token
            )
            if (number + 1) % 20 == 0:
                print(f"{endpoint_name} {variant} completed={number + 1}/{samples_count}", flush=True)
        finally:
            poller.close()

    print(f"server={url} variant={variant} samples={len(results)} directions=" + ",".join(
        f"{direction}={sum(row['direction'] == direction for row in results)}"
        for direction in ("a→b", "b→a")
    ), flush=True)
    keys = ("observed_ms", "enqueue_to_upload_ms", "upload_to_server_ms", "server_apply_ms",
            "server_applied_to_fetch_ms", "fetch_to_observed_ms", "upload_to_fetch_ms",
            "poll_wait_ms")
    for key in keys:
        stats = summarize(results, key)
        print(f"{endpoint_name} {variant} {key}: p50={stats['p50']:.2f}ms p95={stats['p95']:.2f}ms max={stats['max']:.2f}ms", flush=True)
    bins = [sum(50 * i <= row["phase_ms"] < 50 * (i + 1) for row in results) for i in range(5)]
    print(f"{endpoint_name} {variant} randomized_initial_phase_bins_ms=0-49:{bins[0]},50-99:{bins[1]},100-149:{bins[2]},150-199:{bins[3]},200-249:{bins[4]}", flush=True)
    for direction in ("a→b", "b→a"):
        rows = [row for row in results if row["direction"] == direction]
        for key in keys:
            stats = summarize(rows, key)
            print(f"{endpoint_name} {variant} {direction} {key}: p50={stats['p50']:.2f}ms p95={stats['p95']:.2f}ms max={stats['max']:.2f}ms", flush=True)
    if include_raw:
        print(f"{endpoint_name} {variant} raw_samples=" + json.dumps(results, separators=(",", ":")), flush=True)
    return probe_ids, results


def cleanup(local_url, account, token, device, probe_ids, area_id):
    targets = list(dict.fromkeys(probe_ids))
    cleaned = 0
    for offset in range(0, len(targets), 100):
        commands = [make_command(device, "item.delete", {"id": item_id}, index)
                    for index, item_id in enumerate(targets[offset:offset + 100])]
        response, _, _ = request(f"{local_url}/sync", account["token"],
                                 sync_body(token, commands))
        deleted = {row.get("id") for row in response.get("delta", [])
                   if row.get("trashed_at") is not None}
        expected = {command["args"]["id"] for command in commands}
        if not expected.issubset(deleted):
            raise RuntimeError("explicit trash cleanup did not confirm every probe row")
        cleaned += len(expected)
        token = response["token"]
    if area_id:
        command = make_command(device, "item.delete", {"id": area_id}, 0)
        response, _, _ = request(f"{local_url}/sync", account["token"],
                                 sync_body(token, [command]))
        if not any(row.get("id") == area_id and row.get("trashed_at") is not None
                   for row in response.get("delta", [])):
            raise RuntimeError("explicit trash cleanup did not confirm the dedicated Area")
        cleaned += 1
    return cleaned


def measure(endpoint, samples_count, include_raw):
    scratch_parent = pathlib.Path(os.environ.get("TMPDIR", "/dev/shm"))
    with tempfile.TemporaryDirectory(prefix="orbit-issue14-", dir=scratch_parent) as scratch:
        base = pathlib.Path(scratch)
        password = secrets.token_urlsafe(24)
        server = tunnel = None
        server_log = tunnel_log = None
        account = None
        area_id = None
        server_token = None
        probe_ids = []
        local_url = None
        try:
            server, server_log, data_dir, local_url, cli_bin = start_server(base)
            account = setup_account(local_url, server_log, password)
            area_id, server_token = create_probe_area(local_url, account, base)
            endpoints = [("local", local_url)]
            if endpoint in ("quick-tunnel", "both"):
                tunnel, tunnel_log, tunnel_url = start_quick_tunnel(base, local_url)
                endpoints = [("quick-tunnel", tunnel_url)] if endpoint == "quick-tunnel" else endpoints + [("quick-tunnel", tunnel_url)]
            clients = make_clients(endpoints, base, cli_bin, password)
            print(f"local_server={local_url}", flush=True)
            for endpoint_name, url in endpoints:
                print(f"endpoint={endpoint_name} url={url}", flush=True)
                for variant in ("CLI↔CLI", "CLI↔raw-HTTP"):
                    ids, _ = measure_variant(endpoint_name, url, variant, clients, area_id,
                                             samples_count, cli_bin, server_log, include_raw)
                    probe_ids.extend(ids)
        finally:
            try:
                if account and area_id:
                    cleaned = cleanup(local_url, account, server_token, account["device"], probe_ids, area_id)
                    print(f"cleanup=explicitly trashed {cleaned - 1} probe tasks and the dedicated Area", flush=True)
            finally:
                if tunnel:
                    tunnel.terminate()
                    try:
                        tunnel.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        tunnel.kill()
                        tunnel.wait()
                if server:
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
        print("cleanup=disposable SQLite database, client profiles, HOME, and tunnel scratch removed", flush=True)


def measure_web(endpoint, samples_count):
    scratch_parent = pathlib.Path(os.environ.get("TMPDIR", "/dev/shm"))
    with tempfile.TemporaryDirectory(prefix="orbit-issue14-web-", dir=scratch_parent) as scratch:
        base = pathlib.Path(scratch)
        password = secrets.token_urlsafe(24)
        server = tunnel = None
        server_log = tunnel_log = None
        account = None
        area_id = None
        server_token = None
        probe_ids = []
        local_url = None
        try:
            server, server_log, _, local_url, cli_bin = start_server(base, serve_web=True)
            account = setup_account(local_url, server_log, password)
            area_id, server_token = create_probe_area(local_url, account, base)
            endpoints = [("local", local_url)] if endpoint in ("local", "both") else []
            if endpoint in ("quick-tunnel", "both"):
                tunnel, tunnel_log, tunnel_url = start_quick_tunnel(base, local_url)
                endpoints.append(("quick-tunnel", tunnel_url))
            clients = make_clients(endpoints, base, cli_bin, password)
            mise_env = {**os.environ, "MISE_QUIET": "1"}
            playwright = subprocess.check_output(
                ["mise", "which", "playwright"], text=True, stderr=subprocess.DEVNULL, env=mise_env
            ).strip()
            node = subprocess.check_output(
                ["mise", "which", "node"], text=True, stderr=subprocess.DEVNULL, env=mise_env
            ).strip()
            for endpoint_name, url in endpoints:
                client = clients[(endpoint_name, "b")]
                result_path = base / f"{endpoint_name}-web-results.json"
                config = {
                    "endpointName": endpoint_name,
                    "url": url,
                    "areaId": area_id,
                    "areaName": AREA_NAME,
                    "password": password,
                    "profileRoot": str(base / endpoint_name / "browser-profiles"),
                    "serverLog": str(base / "server.log"),
                    "samples": samples_count,
                    "output": str(result_path),
                    "chromium": shutil.which("chromium"),
                    "cliBin": str(cli_bin),
                    "cliDir": str(client["dir"]),
                    "cliDevice": json.loads(meta(client["dir"] / "orbit.db", "hlc"))["device"],
                    "cliEnv": client["env"],
                    "root": str(ROOT),
                }
                print(f"web endpoint={endpoint_name} url={url} samples_per_pair={samples_count}", flush=True)
                child_env = {
                    "PATH": "/usr/bin:/bin",
                    "HOME": str(base / "web-home"),
                    "TMPDIR": os.environ.get("TMPDIR", "/dev/shm"),
                    "PLAYWRIGHT_CLI": playwright,
                }
                (base / "web-home").mkdir(exist_ok=True)
                child = subprocess.Popen(
                    [node, str(ROOT / "scripts" / "latency-web.mjs")],
                    cwd=ROOT,
                    env=child_env,
                    stdin=subprocess.PIPE,
                    text=True,
                )
                child.communicate(json.dumps(config))
                if result_path.is_file():
                    web_results = json.loads(result_path.read_text())
                    probe_ids.extend(web_results.get("probes", []))
                if child.returncode != 0:
                    raise RuntimeError("Chromium Web measurement failed; private diagnostics suppressed")
            print(f"local_server={local_url}", flush=True)
        finally:
            try:
                if account and area_id:
                    cleaned = cleanup(local_url, account, server_token, account["device"], probe_ids, area_id)
                    print(f"cleanup=explicitly trashed {cleaned - 1} Web probe tasks and the dedicated Area", flush=True)
            finally:
                if tunnel:
                    tunnel.terminate()
                    try:
                        tunnel.wait(timeout=5)
                    except subprocess.TimeoutExpired:
                        tunnel.kill()
                        tunnel.wait()
                if server:
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
        print("cleanup=Web profiles, CLI profiles, disposable SQLite DB, HOME, and tunnel scratch removed", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="check harness prerequisites only")
    parser.add_argument("--endpoint", choices=("local", "quick-tunnel", "both"), default="local",
                        help="measure loopback, a fresh isolated Cloudflare Quick Tunnel, or both")
    parser.add_argument("--samples", type=int, default=SAMPLES,
                        help="interleaved samples per variant and endpoint (default: 100)")
    parser.add_argument("--raw", action="store_true", help="print per-sample JSON without IDs or credentials")
    parser.add_argument("--web", action="store_true", help="measure Chromium Web↔CLI and Web↔Web on disposable endpoints")
    args = parser.parse_args()
    if args.check:
        check(args.endpoint, args.web)
        return
    if args.samples < 2:
        raise SystemExit("--samples must be at least 2 so both directions run")
    if args.samples != SAMPLES:
        print(f"warning: recorded runs use {SAMPLES} samples per variant and endpoint", flush=True)
    if args.web:
        measure_web(args.endpoint, args.samples)
    else:
        measure(args.endpoint, args.samples, args.raw)


if __name__ == "__main__":
    main()
