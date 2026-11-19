# PROTOTYPE: Tauri 2 Android spike (throwaway, never merged)

Answers issue #4: can a Tauri 2 APK host a Kotlin module for widget capture, share capture,
a WorkManager-flushed outbox and offline reboot-proof reminders?

Native code: `src-tauri/gen/android/app/src/main/java/dev/orbit/spike/Spike.kt` (+ manifest entries marked PROTOTYPE).
Rust side: `src-tauri/src/lib.rs` (inline Tauri plugin `orbit`, `run_mobile_plugin` into Kotlin).

## Run

```sh
source ~/.toolchain/env.sh
python spike_server.py &                      # stand-in sync server, emulator hits 10.0.2.2:8787
emulator -avd orbit &
cargo tauri android build --debug --apk --target x86_64
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
adb shell pm grant dev.orbit.spike android.permission.POST_NOTIFICATIONS
```

## Results (emulator, API 36 google_apis x86_64, tauri 2.12.7, 2026-11-19)

| Check | How | Result |
|---|---|---|
| Rust -> Kotlin bridge | `run_mobile_plugin("schedule"/"alarms")` from an app command | PASS |
| Share target | `am start -a SEND` text, airplane mode on | PASS, JSON file written, no UI |
| Capture screen (widget target) | typed into `CaptureActivity`, tapped Add | PASS |
| Widget registered | `dumpsys appwidget` lists `CaptureWidget` | PASS |
| Outbox flush, app dead | 2 files offline, `kill -9`, airplane off | PASS, new process, both POSTed within ~1 s |
| Reminder offline, app dead | +60 s, airplane on, `kill -9` | PASS, fired on time in fresh process |
| Reminder across reboot | +180 s, offline, `adb reboot` | PASS, BootReceiver re-armed, fired on time |
| Outbox across reboot | capture offline, reboot, airplane off | PASS, POSTed 13 s after network |
| Rust reads Kotlin outbox | app `state` command lists `files/outbox` + `files/sent` | PASS |

Notes: `am kill` does not kill a recently foreground app; use `adb root` + `kill -9`.
Force-stop (not tested) halts alarms and WorkManager by design (issue #8).
Flush order is by file name (uuid); real impl must order by HLC/created.
