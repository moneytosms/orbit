# Sync latency baseline

**Measured:** CLI and Web on 2026-11-22; Desktop ↔ CLI on 2026-11-23  
**Product commit:** `ab38d06` (each run prints its `source_commit`)  
**Servers measured:** a local loopback server, and an isolated Cloudflare Quick Tunnel created for each invocation (shown here as `<quick-tunnel>`)  
**Status:** provisional. The 4 s verdict below is not measured.

CLI and Web rows use a disposable SQLite server, fresh CLI profiles, and a temporary synthetic account. No live Orbit hostname or reusable credentials were accessed. Probe tasks were placed in a dedicated Area and explicitly trashed; the Area, database, profiles, scratch `HOME`, and tunnel process were removed.

## Method

Each endpoint/variant has 100 samples, alternating 50 A→B and 50 B→A. Each sample independently randomizes the receiver's initial phase in `[0, 250 ms)`, then polls every 250 ms. The output includes five phase bins per variant; the distributions were 13–26 samples per 50 ms bin, with no fixed-phase cluster.

The CLI sender queues a task in the probe Area and runs `orbit sync`. The receiver either polls through a second CLI profile or uses authenticated raw HTTP `POST /sync` with no commands. The Quick Tunnel is created on each invocation with `HOME` set to empty scratch, `--config /dev/null`, an `--origincert` path that does not exist, and the disposable loopback server as its only origin.

`ORBIT_SYNC_TRACE=1` enables the server trace for a run. It records processing start (after the DB lock), completed sync processing, applied task IDs, and returned row IDs. It is off by default. Native builds add opt-in timestamps only (`native_sync_trace`); sync behavior and the data model are unchanged.

Segments: `enqueue → upload` is task `created_at` to sender sync invocation; `upload → server` is invocation to server processing; `server apply` is server processing duration; `server applied → fetch` is the receiver's server processing start minus sender completion (poll wait plus queueing); `fetch → observed` is receiver completion to client response. `upload → observed` is sender completion to client observation. Timestamps are host-local; the Cloudflare edge adds network transit. All values are milliseconds. Percentiles use `floor((n − 1) × p + 0.5)`.

## Aggregate results: CLI

Each row is 100 samples per endpoint/variant, 50 each direction. Cells are p50 / p95 / max.

| Endpoint and pair | Enqueue → upload | Upload → server | Server apply | Server applied → fetch | Fetch → observed | Upload → observed |
|---|---:|---:|---:|---:|---:|---:|
| Local CLI ↔ CLI | 1 / 2 / 2 | 3 / 5 / 5 | 1 / 2 / 2 | 125 / 237 / 244 | 1 / 1 / 249 | 127 / 240 / 250 |
| Local CLI upload ↔ raw HTTP poll | 1 / 2 / 2 | 4 / 5 / 5 | 2 / 3 / 4 | 143 / 235 / 240 | 0 / 1 / 1 | 144 / 236 / 241 |
| Quick Tunnel CLI ↔ CLI | 1 / 2 / 2 | 225 / 244 / 254 | 2 / 3 / 6 | 133 / 267 / 633 | 49 / 53 / 284 | 185 / 314 / 685 |
| Quick Tunnel CLI upload ↔ raw HTTP poll | 1 / 2 / 3 | 171 / 191 / 471 | 3 / 4 / 6 | 120 / 224 / 244 | 48 / 54 / 57 | 174 / 273 / 295 |

### Directional observed latency (CLI)

Each row is 50 samples. Cells are p50 / p95 / max, enqueue → observed.

| Endpoint and pair | Direction | Enqueue → observed |
|---|---|---:|
| Local CLI ↔ CLI | A→B | 143 / 239 / 245 |
| Local CLI ↔ CLI | B→A | 132 / 250 / 255 |
| Local CLI ↔ raw HTTP poll | A→B | 147 / 241 / 244 |
| Local CLI ↔ raw HTTP poll | B→A | 156 / 247 / 248 |
| Quick Tunnel CLI ↔ CLI | A→B | 421 / 520 / 825 |
| Quick Tunnel CLI ↔ CLI | B→A | 423 / 580 / 942 |
| Quick Tunnel CLI ↔ raw HTTP poll | A→B | 346 / 456 / 464 |
| Quick Tunnel CLI ↔ raw HTTP poll | B→A | 342 / 466 / 714 |

## Aggregate results: Web

Real headless Chromium profiles, the app's normal visible 60 s polling, and no forced focus sync. Each endpoint/pair has 100 samples, alternating directions 50/50. Cells are p50 / p95 / max, in milliseconds.

| Endpoint and pair | Enqueue → upload | Upload → server | Server apply | Server applied → fetch | Fetch → observed | Enqueue → observed |
|---|---:|---:|---:|---:|---:|---:|
| Local Web ↔ CLI | 782 / 2,058 / 2,062 | 2 / 5 / 6 | 1 / 1 / 2 | 3,087 / 4,381 / 59,085 | 79 / 447 / 507 | 3,304 / 4,818 / 59,178 |
| Local Web ↔ Web | 2,057 / 2,062 / 2,078 | 0 / 1 / 1 | 1 / 2 / 5 | 2,299 / 2,318 / 39,256 | 226 / 331 / 499 | 4,583 / 4,665 / 41,542 |
| Quick Tunnel Web ↔ CLI | 889 / 2,062 / 2,105 | 212 / 253 / 485 | 2 / 3 / 4 | 3,092 / 4,274 / 26,856 | 267 / 386 / 518 | 3,833 / 4,863 / 27,607 |
| Quick Tunnel Web ↔ Web | 2,051 / 2,063 / 2,143 | 45 / 59 / 140 | 1 / 1 / 3 | 2,297 / 2,456 / 43,330 | 169 / 447 / 643 | 4,565 / 4,717 / 45,976 |

Directional enqueue → observed p50 / p95 / max, 50 samples per direction:

| Endpoint and pair | A→B | B→A |
|---|---:|---:|
| Local Web ↔ CLI | 2,174 / 2,296 / 2,314 | 4,811 / 4,822 / 59,178 |
| Local Web ↔ Web | 4,583 / 4,665 / 4,783 | 4,583 / 4,675 / 41,542 |
| Quick Tunnel Web ↔ CLI | 2,456 / 2,722 / 2,820 | 4,849 / 4,892 / 27,607 |
| Quick Tunnel Web ↔ Web | 4,565 / 4,717 / 4,897 | 4,569 / 4,769 / 45,976 |

The large maxima are observed samples, not removed outliers. The 60 s browser timer is the primary source of long-tail delay. The earlier Quick Tunnel Web ↔ CLI run completed its 100 samples but failed in the following Web ↔ Web pair before cleanup was confirmed; the Web ↔ Web retry completed and confirmed cleanup. The Web ↔ CLI cleanup status from that earlier run is not counted as success.

## Aggregate results: Desktop ↔ CLI

Native Desktop build with the opt-in `native_sync_trace` hook, using the app's normal visible 60 s polling. Each endpoint has 100 samples. The parent ledger does not name the metric for these two lines, so the segment they measure is not stated here.

| Endpoint and pair | p50 | p95 | max | Date | Source |
|---|---:|---:|---:|---|---|
| Local Desktop ↔ CLI | 31,279 | 56,800 | 59,692 | not stated in ledger | parent ledger |
| Quick Tunnel Desktop ↔ CLI | 32,497 | 57,573 | 61,045 | 2026-11-23 | parent ledger, `logs/14-desktop-tunnel-r1.log` |

Both rows are ms. The p95 values sit near the 60 s timer, which is the same pattern as the Web rows.

## Native coverage and verdict

Measured: CLI, raw HTTP, Chromium Web, and Desktop ↔ CLI, on the local and Quick Tunnel endpoints, as above.

Not yet measured (no numbers in this document):

- Deployed server (any non-disposable, non-local origin).
- Desktop ↔ Android.
- Desktop ↔ Desktop and Android ↔ Android (same-client pairs).
- Android foreground ↔ CLI and Android foreground ↔ Web. The Android runner (`scripts/latency-native-android.py`) exists and its isolated-AVD gate is in place, but the run has not been executed for this document.
- Android background (WorkManager, 15 min schedule). It is not accelerated by the harness, and 100 independent samples are not possible under that schedule, so it stays unsupported at this granularity.
- Desktop ↔ Web. The webview script (`scripts/latency-native-webview.mjs`) has `desktopPeer` code, but `scripts/latency-native.py` has no flag that supplies it, so this pair is not runnable from the harness yet.
- The 4 s interval itself, and native timer behavior under real network variation.

Verdict (provisional): against a 5 s p95 target, a 60 s poll interval cannot meet the target, because poll wait can approach 60 s. Desktop ↔ CLI at 56.8 s p95 (loopback) and 57.6 s p95 (Quick Tunnel) confirms that the timer dominates. A 4 s interval is a candidate: under uniform phase it implies about 3.8 s p95 poll wait. That candidate has not been measured, so it is not a recommendation yet.

Request volume for one continuously visible client: 60 s polling is 1,440 requests/day, 5 s is 17,280, and 4 s is 21,600 (15× the current count). Estimated incremental Tunnel plan charge is $0 within the first 1,000 tunnels; origin compute, network transfer, and other products are excluded. See [Cloudflare Tunnel](cloudflare-tunnel.md#request-volume-and-tunnel-cost) for the cost section.

## Re-run

One command per endpoint for the CLI and Web harness (Web is included by `--web`):

```sh
# local endpoint
scripts/latency.sh --endpoint local --web

# Quick Tunnel endpoint (creates a new isolated tunnel per invocation)
scripts/latency.sh --endpoint quick-tunnel --web
```

`--samples N` is for exploratory runs; the baseline is 100. `--raw` prints per-sample CLI timings without identifiers. Passwords, setup codes, auth tokens, and Cloudflare logs stay in private scratch and are never printed.

Native Desktop (requires a traced build and its SHA-256; set `ORBIT_XVFB` to a private Xvfb if it is not on `PATH`):

```sh
python3 scripts/latency-native.py --platform desktop --endpoint local \
  --desktop-binary /path/to/orbit-app-traced --desktop-sha256 <sha256>

python3 scripts/latency-native.py --platform desktop --endpoint quick-tunnel \
  --desktop-binary /path/to/orbit-app-traced --desktop-sha256 <sha256>
```

Native Android (requires the isolated AVD named by `ORBIT_AVD_NAME`, default `orbit-latency`, and `ANDROID_SDK_ROOT` if the SDK is not at `~/Android/Sdk`). The runner refuses to run outside that AVD:

```sh
python3 scripts/latency-native.py --platform android --endpoint local --pair both \
  --android-apk /path/to/orbit-android.apk --android-sha256 <sha256> \
  --android-host-wrapper /path/to/host-wrapper.zsh

python3 scripts/latency-native.py --platform android --endpoint quick-tunnel --pair both \
  --android-apk /path/to/orbit-android.apk --android-sha256 <sha256> \
  --android-host-wrapper /path/to/host-wrapper.zsh
```

Bounded origin and Web UI preflight (zero samples, no app launch):

```sh
python3 scripts/latency-native.py --platform android --preflight \
  --android-apk /path/to/orbit-android.apk --android-sha256 <sha256> \
  --android-host-wrapper /path/to/host-wrapper.zsh
```

The native runner requires exactly 100 samples and alternates direction 50/50.
