# Android manual checklist

Run on a real device before a release. Emulators skip the OEM battery behaviour that matters here.

Package: `dev.orbit.app`. Replace `<serial>` with the output of `adb devices` (omit `-s` with one device).

Useful commands:

```sh
adb -s <serial> logcat -s Orbit               # app log (sync, reminder, alarm lines)
adb -s <serial> shell dumpsys alarm | grep -A3 dev.orbit.app   # armed alarm
adb -s <serial> shell am force-stop dev.orbit.app             # kill the process
adb -s <serial> reboot                                        # reboot
adb -s <serial> shell date -s "<YYYY-MM-DD HH:MM:SS>"        # root/emulator only: move the clock
```

## #69 Reminders

Create a Task due a few minutes ahead with a timed due (or a reminder at a set time).

1. **Offline fire.** Turn on airplane mode. Wait for the reminder time. Expect the notification at the time, with no network.
   - Log: `adb logcat -s Orbit` shows no network error; `dumpsys alarm` lists dev.orbit.app before the time and not after.
2. **Process dead.** Create the reminder, then `adb shell am force-stop dev.orbit.app`. Wait for the time. Expect the notification to appear and tapping it to open Orbit.
3. **After reboot.** Create the reminder, then `adb reboot`. Unlock the device without opening Orbit. Wait for the time. Expect the notification (the BOOT_COMPLETED receiver re-arms the alarm).
4. **Late reminder.** Put the phone in airplane mode and force-stop Orbit before the time; reopen Orbit 20 minutes after it. Expect the reminder to show at once on open.
5. **Actions.** On a notification, tap Snooze: it returns 10 minutes later. Tap Complete: the Task leaves the list after the next sync and no further reminder appears.
6. **Permission.** On Android 13+, a fresh install asks for notification permission on first open. Deny it, then confirm in Settings that the app shows no notifications; allow it and confirm they appear.
7. **Battery guidance.** On first open a dialog explains unrestricted battery use; "Open app info" opens the app's settings page. It must not appear again after dismissing it.
8. **OEM battery saver (Samsung, Xiaomi, Huawei, etc.).** Set the app to unrestricted battery use, leave the phone idle for 30 minutes, and confirm the reminder still fires on time.
