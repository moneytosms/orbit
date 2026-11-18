# Android background sync and exact alarms, sideloaded APK (orbit#8)

Question: Tauri 2 app + Kotlin module, sideloaded (not Play), Android 14-16. Needs: (a) offline task reminders via AlarmManager, (b) reschedule after reboot, (c) flush native outbox via WorkManager on network return with app closed. Researched 2026-11-19.

Verification legend: **[verbatim]** read from the page text; **[summary]** read via a summarising fetch, re-read before quoting verbatim; **[reasoning]** inference, not stated by a source; **UNVERIFIED** not confirmed in any fetched source.

## Short answer
- (a) Reliable via `setAlarmClock()` or `setExactAndAllowWhileIdle()`. Permission: `USE_EXACT_ALARM` (auto-granted, not user-revocable) or `SCHEDULE_EXACT_ALARM` (user grants, revocable). https://developer.android.com/develop/background-work/services/alarms [summary]
- (b) Alarms are cleared on reboot. Re-register from a `BOOT_COMPLETED` receiver. Needs `RECEIVE_BOOT_COMPLETED`; works only after the user has launched the app at least once. https://developer.android.com/develop/background-work/services/alarms [summary]; https://developer.android.com/reference/android/app/AlarmManager [summary]
- (c) WorkManager with a network constraint persists across app exit and reboot. Deferred in Doze; no timing guarantee. https://developer.android.com/develop/background-work/background-tasks/persistent [summary]
- A user force-stop cancels alarms, jobs and pending intents until the app is reopened. https://developer.android.com/about/versions/15/behavior-changes-all [summary]; https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/work/work-runtime/src/main/java/androidx/work/impl/utils/ForceStopRunnable.java [verbatim, source]
- OEM task killers can defeat all of this. Only user-side settings help. https://dontkillmyapp.com/ [summary]
- Play policy does not bind a sideload. No fetched page says so explicitly. [reasoning]

## Permissions

| Item | SCHEDULE_EXACT_ALARM | USE_EXACT_ALARM |
|---|---|---|
| What it is | Special app access ("Alarms & reminders"); revocable by user or system. https://developer.android.com/reference/android/Manifest.permission | Same capability, granted without a user prompt. Same page. |
| Added / requires | API 31 | API 33; app must target 33+ |
| Grant | User grants. Broader use cases. https://developer.android.com/develop/background-work/services/alarms [summary] | Granted automatically; user cannot revoke. Same page [summary] |
| Default state | Not pre-granted to fresh installs targeting 13+. Denied after backup/restore to Android 14. Pre-granted if an existing app already held it when the device upgraded to 14. [summary] | Granted at install |
| Android 12/12L default | UNVERIFIED | n/a |
| Android 15/16 default | UNVERIFIED as a separate change; the alarms page states the 13+ rule generically | Same |
| Settings intent | `Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM` (API 31), optionally with `package:` data URI; `RESULT_OK` if granted. https://developer.android.com/reference/android/provider/Settings [summary] | n/a |
| Check | `AlarmManager.canScheduleExactAlarms()`. https://developer.android.com/reference/android/app/AlarmManager [summary] | Same |
| On revoke | Exact alarms cancelled, app stopped. On grant the system sends `ACTION_SCHEDULE_EXACT_ALARM_PERMISSION_STATE_CHANGED`; re-check and reschedule. [summary] | n/a |
| Missing permission | `SecurityException` for apps targeting 12+. `setExact` with an `OnAlarmListener` does not need it. [summary] | Same |
| Category restriction | None | Docs cite "limited use cases" and an upcoming Play policy. Platform reference says "only intended for apps that rely on exact alarms for their core functionality". No platform enforcement found. https://developer.android.com/reference/android/Manifest.permission [summary] |
| Play policy | Not applicable to a sideload [reasoning] | Play allows alarm/timer and calendar-event apps. https://support.google.com/googleplay/android-developer/answer/9888170 [summary] |
| Coexistence | If needing `SCHEDULE_EXACT_ALARM` below 33 and `USE_EXACT_ALARM` on 33+, set `android:maxSdkVersion="32"` on the first. Reference page [summary] | Same |

Standby bucket: holders "always stay in the WORKING_SET or lower" (stated for `USE_EXACT_ALARM`, and for `SCHEDULE_EXACT_ALARM` on API 33 and below). https://developer.android.com/reference/android/Manifest.permission [summary]

For a sideload, `USE_EXACT_ALARM` has the least friction: no user step. Its revocation path is absent by design. [reasoning]

## Doze and boot

| API | Fires in Doze | Notes |
|---|---|---|
| `set`, `setExact`, `setWindow` | No; deferred to maintenance window | https://developer.android.com/training/monitoring-device-state/doze-standby [summary] |
| `setAndAllowWhileIdle` | Yes, inexact. On 12+, fires within one hour unless battery saver or Doze applies. | Alarms page [summary] |
| `setExactAndAllowWhileIdle` | Yes; rate limited; about 10 s temporary exemption on dispatch | https://developer.android.com/reference/android/app/AlarmManager [summary] |
| `setAlarmClock` | Yes; system does not adjust delivery | https://developer.android.com/develop/background-work/services/alarms [summary] |

- Rate limit conflict: the Doze page says `setAndAllowWhileIdle` / `setExactAndAllowWhileIdle` fire at most once per nine minutes per app. The AlarmManager reference says "not more than about every minute" normally, and about 15 minutes in idle. The pages disagree. `setAlarmClock` is not described as rate limited.
- Reboot: alarms are retained while asleep but "cleared if it is turned off and rebooted". https://developer.android.com/reference/android/app/AlarmManager [summary]
- Boot receiver: declare `RECEIVE_BOOT_COMPLETED` and a `BOOT_COMPLETED` receiver. Docs example uses `android:enabled="false"` and `setComponentEnabledSetting`; the enabled state persists across reboots. https://developer.android.com/develop/background-work/services/alarms [summary]
- Direct boot: `LOCKED_BOOT_COMPLETED` requires `android:directBootAware="true"`; only device-protected storage is readable before unlock. https://developer.android.com/privacy-and-security/direct-boot [summary]. Credential-encrypted storage (where Tauri app data normally lives) is unavailable until first unlock [reasoning]. Use `BOOT_COMPLETED` unless alarms are needed before unlock.
- Android 13 restricted state: if the user sets the app to "restricted" and it targets 13, `BOOT_COMPLETED` and `LOCKED_BOOT_COMPLETED` are not delivered until the app starts for another reason. https://developer.android.com/about/versions/13/behavior-changes-13 [summary]
- Android 15 force-stop: pending intents are cancelled when the app enters the stopped state; `BOOT_COMPLETED` is delivered after the user takes the app out of it. https://developer.android.com/about/versions/15/behavior-changes-all [summary; re-read the "package stopped state" section before quoting]
- Other broadcasts: `ACTION_MY_PACKAGE_REPLACED`, `ACTION_TIME_CHANGED`, `ACTION_TIMEZONE_CHANGED`. https://developer.android.com/reference/android/content/Intent [summary]. Alarms page warns RTC alarms can behave unexpectedly after a user clock change. Re-deriving alarms on these three broadcasts is a recommendation [reasoning].
- Android 14: exact alarms are exempt from foreground-service launch restrictions. [summary]
- Android 14 PendingIntents: use an explicit component and `FLAG_IMMUTABLE`. https://developer.android.com/about/versions/14/behavior-changes-14 [summary]
- `OnAlarmListener` alarms are dropped once the app leaves its lifecycle; use the PendingIntent variant. https://developer.android.com/reference/android/app/AlarmManager [summary]
- UNVERIFIED: whether alarms survive an APK update; whether manifest-declared receivers get the three broadcasts on 14-16.

## WorkManager
- Persistence: work is stored in SQLite, "rescheduled across device reboots", for work that must run after the user leaves or the app exits. https://developer.android.com/develop/background-work/background-tasks/persistent [summary]
- Doze: WorkManager uses JobScheduler, which does not run in Doze. https://developer.android.com/training/monitoring-device-state/doze-standby [summary]
- Network constraint: `Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED)`. If the constraint is lost mid-run, the worker stops and retries later. https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work [summary]
- `ACCESS_NETWORK_STATE`: on targetSdk 34+, JobScheduler network constraints throw `SecurityException` without it. https://developer.android.com/about/versions/14/behavior-changes-14 [summary]. Whether WorkManager merges it in: UNVERIFIED; declare it.
- Periodic minimum interval 15 minutes; timing not exact. define-work page [summary]
- Backoff minimum 10 s; default exponential from 30 s. define-work page [summary]
- Expedited: quota based on standby bucket and process importance; may be deferred; cannot be scheduled for later. define-work page [summary]
- Android 16: job and expedited-job quotas also depend on standby bucket, whether the job started while the app was top-visible, and concurrency with a foreground service. Debug with `WorkInfo.getStopReason()`. https://developer.android.com/about/versions/16/behavior-changes-all [summary]
- Force-stop: `ForceStopRunnable` states alarms and jobs are cancelled on force-stop, and WorkManager reschedules unfinished work when the app is next started. https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/work/work-runtime/src/main/java/androidx/work/impl/utils/ForceStopRunnable.java [verbatim, source]. Nothing flushes while force-stopped [reasoning].
- `WorkInfo.STOP_REASON_USER` covers force-stop, adb, uninstall. https://developer.android.com/reference/androidx/work/WorkInfo [summary]
- Sideload: no WorkManager page lists a Play-only or special permission requirement. Absence in fetched docs, not an explicit statement.
- Android 15 network: reportedly, requests started outside a valid process lifecycle throw `UnknownHostException` or `IOException`. https://developer.android.com/about/versions/15/behavior-changes-all [summary]. UNVERIFIED wording.
- Dependency: `androidx.work:work-runtime-ktx:2.12.0`. https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started [summary]

## OEM (dontkillmyapp.com)
All pages read via summarising fetch; re-read before quoting. Index: https://dontkillmyapp.com/

- **Xiaomi (MIUI):** https://dontkillmyapp.com/xiaomi. Fixes: lock in Recents, Autostart, Battery Saver "No restrictions", "Keep running after screen off". MIUI Optimizations can break background tasks. Covers MIUI 10-14 only; HyperOS UNVERIFIED. Alarms and boot not mentioned.
- **Huawei:** https://dontkillmyapp.com/huawei. Fixes: App launch "Manage manually", Startup manager. PowerGenie kills non-whitelisted apps; page says only adb uninstall fully fixes it. Alarms not mentioned.
- **Honor:** https://dontkillmyapp.com/honor returned 404. UNVERIFIED.
- **Samsung:** https://dontkillmyapp.com/samsung. Quote: "After 3 days any unused app will not be able to start from background (e.g. alarms will not work anymore)". Fixes: "Never sleeping apps", disable "Put unused apps to sleep", battery "Don't optimize". Page: "No known solution on dev end."
- **OnePlus:** https://dontkillmyapp.com/oneplus. Fixes: battery optimization off, disable Deep optimization and Auto-Launch, lock in Recents ("not 100%"). Settings "started reverting randomly". Alarms and boot not mentioned.
- **Oppo:** https://dontkillmyapp.com/oppo. Covers F1S only. Needs four user settings; no dev-side fix.
- **realme:** https://dontkillmyapp.com/realme. Power saving may stop background work, including alarms; check "App battery management". "Not fully uncovered yet."
- **Vivo:** https://dontkillmyapp.com/vivo. Fixes: Autostart, unrestricted battery, lock in recents. "Not fully uncovered yet."
- Not checked: Nothing, Google Pixel, others. UNVERIFIED.

Battery optimisation exemption:
- `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` (normal protection) is needed for `ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` (data `package:<id>`). https://developer.android.com/reference/android/Manifest.permission [summary]
- Platform stance: that action is "only for unusual applications that need to deeply control their own execution". https://developer.android.com/reference/android/provider/Settings [summary]
- Doze page lists "core function is scheduling automated actions" as acceptable; Play prohibits the prompt unless core function is adversely affected. https://developer.android.com/training/monitoring-device-state/doze-standby [summary]. Sideload not affected by Play policy [reasoning].
- Alternative with no prompt: `ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS`. Check state with `PowerManager.isIgnoringBatteryOptimizations()`.
- Does not stop force-stop or OEM sleeping-app lists.

## Verdict

Reliable:
- Reminders via `setAlarmClock()` (system leaves low-power modes to deliver it), or `setExactAndAllowWhileIdle()` with `USE_EXACT_ALARM`, within the rate limit.
- Reboot recovery via `BOOT_COMPLETED`, once the app has been launched at least once.
- Outbox persistence in SQLite plus WorkManager: survives process death and reboot.

Not reliable:
- Anything after a user force-stop, until the app is reopened.
- Android 13 "restricted" state: no boot broadcast.
- Samsung after 3 days unused (documented to stop alarms).
- Huawei Startup manager and PowerGenie; Xiaomi, Oppo, OnePlus, Vivo restriction settings.
- WorkManager timing in Doze; 15-minute periodic floor; expedited quota.
- Exact alarms faster than the rate limit.

Manifest:
```xml
<uses-permission android:name="android.permission.USE_EXACT_ALARM"/>
<uses-permission android:name="android.permission.RECEIVE_BOOT_COMPLETED"/>
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>
<uses-permission android:name="android.permission.POST_NOTIFICATIONS"/>  <!-- UNVERIFIED here: 13+ runtime flow -->
<uses-permission android:name="android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS"/>  <!-- optional -->
```
Alternative to `USE_EXACT_ALARM`: `SCHEDULE_EXACT_ALARM` plus `canScheduleExactAlarms()` check and a Settings prompt. `INTERNET` also needed (not researched).

API calls:
- `AlarmManager.setAlarmClock(AlarmClockInfo, PendingIntent)` or `setExactAndAllowWhileIdle(RTC_WAKEUP, ...)`, explicit-component PendingIntent with `FLAG_IMMUTABLE`.
- `BOOT_COMPLETED` receiver that re-registers future alarms from persisted storage.
- `WorkManager.enqueueUniqueWork(OneTimeWorkRequest)` with `NetworkType.CONNECTED` and exponential backoff.
- Periodic safety-net work at 15 minutes or more.
- Optional: `PowerManager.isIgnoringBatteryOptimizations()` with `ACTION_IGNORE_BATTERY_OPTIMIZATION_SETTINGS`.

## Open questions
- Default `SCHEDULE_EXACT_ALARM` state on Android 12/12L and a per-version 15/16 statement.
- `USE_EXACT_ALARM` protection level (not on the fetched reference).
- Do alarms survive an APK update (`MY_PACKAGE_REPLACED`)?
- Does WorkManager merge `ACCESS_NETWORK_STATE`?
- Honor and HyperOS OEM pages; `POST_NOTIFICATIONS` runtime flow on 13+.
- Does the Tauri 2 Android activity lifecycle interfere with the Kotlin module? Not a platform question; needs a spike.
- Resolve the 9-minute vs about-1-minute rate-limit conflict.

## Caveats
- Most pages were read via summarising fetch, not verbatim. Source-specific wording marked above.
- Research was done by a subagent whose tools could not write files; the spot-check of the alarms guide was the only re-verification.

## Sources
- https://developer.android.com/develop/background-work/services/alarms
- https://developer.android.com/reference/android/app/AlarmManager
- https://developer.android.com/reference/android/Manifest.permission
- https://developer.android.com/reference/android/provider/Settings
- https://developer.android.com/reference/android/content/Intent
- https://developer.android.com/training/monitoring-device-state/doze-standby
- https://developer.android.com/privacy-and-security/direct-boot
- https://developer.android.com/develop/background-work/background-tasks/persistent
- https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started
- https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work
- https://developer.android.com/reference/androidx/work/WorkInfo
- https://developer.android.com/about/versions/13/behavior-changes-13
- https://developer.android.com/about/versions/14/behavior-changes-14
- https://developer.android.com/about/versions/15/behavior-changes-all
- https://developer.android.com/about/versions/16/behavior-changes-all
- https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/work/work-runtime/src/main/java/androidx/work/impl/utils/ForceStopRunnable.java
- https://support.google.com/googleplay/android-developer/answer/9888170
- https://dontkillmyapp.com/ (and /xiaomi, /huawei, /samsung, /oneplus, /oppo, /vivo, /realme, /honor)
