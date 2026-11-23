#!/usr/bin/env python3
"""Run the foreground Android WebView pairs inside the verified disposable AVD."""

import hashlib
import html
import json
import os
import pathlib
import re
import socket
import subprocess
import sys
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
SDK = pathlib.Path(os.environ.get("ANDROID_SDK_ROOT", pathlib.Path.home() / "Android" / "Sdk"))
ADB = SDK / "platform-tools" / "adb"
SERIAL = os.environ.get("ORBIT_ANDROID_SERIAL", "emulator-5554")
PORT = int(os.environ.get("ORBIT_ADB_PORT", "5037"))
AVD_NAME = os.environ.get("ORBIT_AVD_NAME", "orbit-latency")
PACKAGE = "dev.orbit.app"
JS_STAGES = {
    "android_cdp_attach", "android_page_attach", "android_login_or_ready",
    "android_area_route", "android_visibility", "android_device_lookup",
    "android_ui_ready", "android_cli_pair", "web_peer_ready",
    "android_web_pair", "write_results",
}
JS_EXCEPTIONS = {"Error", "TimeoutError", "TypeError", "ProtocolError", "TargetClosedError"}


class AdbError(RuntimeError):
    def __init__(self, operation):
        super().__init__(f"isolated adb operation failed: {operation}")


def run(*args, input=None, timeout=90):
    if args[:3] == ("emu", "avd", "name"):
        operation = "verify_avd_name"
    elif args[:1] == ("uninstall",):
        operation = "uninstall_app"
    elif args[:1] == ("install",):
        operation = "install_apk"
    elif args[:1] == ("reverse",):
        operation = "configure_loopback_reverse"
    elif args[:1] == ("forward",):
        operation = "forward_webview"
    elif args[:2] == ("shell", "run-as") or (args[:1] == ("shell",) and args[1:2] and args[1].startswith("run-as ")):
        operation = "run_as_preferences"
    elif args[:3] == ("shell", "am", "start"):
        operation = "launch_app"
    elif args[:3] == ("shell", "am", "force-stop"):
        operation = "stop_app"
    elif args[:2] == ("shell", "pidof"):
        operation = "read_app_pid"
    elif args[:3] == ("shell", "pm", "clear"):
        operation = "clear_app_data"
    else:
        operation = "other_allowlisted_adb_step"
    try:
        result = subprocess.run([str(ADB), "-P", str(PORT), "-s", SERIAL, *args],
                                input=input, capture_output=True, text=True,
                                timeout=timeout)
    except subprocess.TimeoutExpired:
        raise RuntimeError(f"isolated adb operation timed out: {operation}") from None
    except OSError:
        raise RuntimeError(f"isolated adb operation unavailable: {operation}") from None
    if result.returncode:
        raise AdbError(operation)
    return result.stdout.strip()


def digest(path):
    sha = hashlib.sha256()
    with open(path, "rb") as source:
        for block in iter(lambda: source.read(1 << 20), b""):
            sha.update(block)
    return sha.hexdigest()


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def wait_webview(port, url):
    end = time.monotonic() + 40
    while time.monotonic() < end:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{port}/json", timeout=1) as response:
                targets = json.load(response)
            if any(item.get("type") == "page" and item.get("url", "").startswith(url)
                   for item in targets):
                return
        except Exception:
            pass
        time.sleep(0.25)
    raise RuntimeError("Android WebView debugging target did not expose the disposable origin")


def main(apk_path):
    config_path = pathlib.Path(os.environ["ORBIT_NATIVE_CONFIG"])
    config = json.loads(config_path.read_text())
    apk = pathlib.Path(apk_path).resolve(strict=True)
    if digest(apk) != config["apkSha256"]:
        raise SystemExit("Android APK hash mismatch")
    if run("emu", "avd", "name").splitlines()[0] != AVD_NAME:
        raise SystemExit(f"refusing to run outside the isolated {AVD_NAME} AVD (set ORBIT_AVD_NAME)")
    probe_ids = []
    forward = None
    reverse = None
    logcat = None
    android_log = pathlib.Path(config["androidLog"]).open("w")
    try:
        run("install", "-r", str(apk), timeout=180)
        run("shell", "pm", "clear", PACKAGE)
        if config["endpointName"] == "local":
            port = int(config["url"].rsplit(":", 1)[1])
            reverse = f"tcp:{port}"
            run("reverse", reverse, reverse)
        escaped = html.escape(config["url"], quote=True)
        prefs = (f'<?xml version="1.0" encoding="utf-8"?><map>'
                 f'<string name="server">{escaped}</string>'
                 '<boolean name="sync_trace" value="true"/></map>')
        run("shell", f"run-as {PACKAGE} sh -c 'mkdir -p shared_prefs; cat > shared_prefs/orbit.xml'",
            input=prefs)
        logcat = subprocess.Popen([str(ADB), "-P", str(PORT), "-s", SERIAL,
                                   "logcat", "-v", "epoch", "-s", "Orbit:I", "*:S"],
                                  stdout=android_log, stderr=subprocess.DEVNULL)
        run("shell", "am", "start", "-W", "-n", f"{PACKAGE}/.MainActivity", timeout=60)
        pid = run("shell", "pidof", PACKAGE).split()[0]
        forward = f"tcp:{free_port()}"
        devtools = f"webview_devtools_remote_{pid}"
        run("forward", forward, f"localabstract:{devtools}")
        port = int(forward.split(":", 1)[1])
        wait_webview(port, config["url"])
        config["webviewPort"] = port
        config["apkSha256"] = digest(apk)
        input_data = json.dumps(config)
        playwright = config["playwright"]
        node = subprocess.run(
            [config["node"], str(ROOT / "scripts" / "latency-native-webview.mjs")],
            cwd=ROOT,
            env={"PATH": os.environ.get("PATH", os.defpath), "PLAYWRIGHT_CLI": playwright,
                 "HOME": str(config_path.parent), "TMPDIR": str(config_path.parent)},
            input=input_data,
            capture_output=True,
            text=True,
            timeout=6 * 60 * 60,
        )
        if node.stdout:
            print(node.stdout.rstrip(), flush=True)
        if node.returncode:
            failure = re.search(
                r"^native_failure stage=([a-z_]+) exception=([A-Za-z]+)$",
                node.stderr,
                re.MULTILINE,
            )
            stage, exception = failure.groups() if failure else ("android_webview_startup", "Error")
            if stage not in JS_STAGES or exception not in JS_EXCEPTIONS:
                stage, exception = "android_webview_startup", "Error"
            print(f"android_webview_failure stage={stage} exception={exception}", flush=True)
            raise RuntimeError("Android foreground measurement failed; private diagnostics suppressed")
        result = json.loads(pathlib.Path(config["output"]).read_text())
        probe_ids.extend(result.get("probes", []))
        print(f"android_artifact_sha256={digest(apk)}; avd={run('emu', 'avd', 'name').splitlines()[0]}; serial={SERIAL}", flush=True)
    finally:
        if logcat and logcat.poll() is None:
            logcat.terminate()
            try:
                logcat.wait(timeout=5)
            except subprocess.TimeoutExpired:
                logcat.kill()
                logcat.wait()
        android_log.close()
        if forward:
            cleanup("forward", "--remove", forward)
        if reverse:
            cleanup("reverse", "--remove", reverse)
        cleanup("shell", "am", "force-stop", PACKAGE)
        cleanup("uninstall", PACKAGE)


def cleanup(*args):
    try:
        run(*args)
    except RuntimeError as error:
        print(f"android_cleanup_warning={error}", flush=True)


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("expected staged APK path")
    main(sys.argv[1])
