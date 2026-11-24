# Android app

For people installing Orbit on an Android phone (Android 12 or newer, minSdk 31) and checking that it behaves on their device.

## Before you start

- A running Orbit server reachable over **HTTPS**. The release APK is signed with the project's release key and refuses cleartext HTTP. Set one up with [Deploy](deploy.md).
- The server address and your account password. See [Connect your clients](deploy.md#5-connect-your-clients).

## Steps

### Install the APK

Either:

- **Sideload:** download `Orbit-<version>-android.apk` from the [GitHub Releases](https://github.com/moneytosms/orbit/releases) page. Open it on the phone and allow installing from that browser or file manager when Android asks (Settings > Apps > Special app access > Install unknown apps).
- **Obtainium:** add the app with the repo URL `https://github.com/moneytosms/orbit` and set the APK filter to `Orbit-.*-android\.apk`, as in [Releasing](releasing.md#obtainium). Obtainium then installs and updates the APK from the releases.

### First run

1. Open Orbit. The first page asks for your server URL. Enter the HTTPS address and continue. The app checks the server is reachable, saves the URL and opens it.
2. Log in. The phone becomes a Device (listed in Settings > Account and devices).
3. Grant **notifications** when asked (`POST_NOTIFICATIONS`).
4. Reminders use exact alarms. The manifest declares `USE_EXACT_ALARM`, so there is no separate prompt.
5. Battery: on first launch Orbit shows a one-time dialog about battery savers. Tap **Open** to reach Orbit's app settings, then open **Battery** and set it to **Unrestricted**. Some phone makers hide this under a different name; see [Troubleshooting](#troubleshooting).

### Widgets

Long-press the home screen, open **Widgets**, find Orbit, and drag a widget out. Available widgets (labels from `res/values/strings.xml`):

- **Quick add**: tap to add a task to Orbit.
- **Task list**: shows a view, area, project or label as a list. Pick what it shows when you add it (the widget configuration screen opens).
- **Routines**: today's Routines; tap one to check it in.
- **Goal**: one Goal as a card, or a list of Goals; tap to open it in Orbit. Pick the Goal when you add it.

### Share to Orbit

Share any text from another app (browser, notes, a chat) and choose **Orbit**. The text becomes an Inbox task. This works offline; the task syncs when you are back online.

## Verify it worked

Add a task with the Quick add widget, then run `orbit sync && orbit ls` on a laptop signed in to the same server. The task should be listed. For the full on-device test plan, see [Does it work on my phone?](#does-it-work-on-my-phone) below.

## Troubleshooting

**Can't connect to the server.** The URL must start with `https://`. Check the certificate in the phone's browser. With Tailscale, the Tailscale app must be running and connected to the same tailnet. More in [Install, Troubleshooting](install.md#troubleshooting).

**Reminders are late or missing.**

- Set Orbit to **Unrestricted** in battery settings (step 5 of First run). Battery savers can delay or kill alarms.
- Check notifications are allowed for Orbit.
- Some phone makers add their own "auto-start" or "background activity" switch; enable it for Orbit.
- Alarms do not survive a reboot; Orbit arms the next reminder again on boot (`BootReceiver`).

**Updating fails with a signature error.** Android only updates an app when the new APK has the same signature. Always use the release APK, never a debug build.

## Does it work on my phone?

Run this on a real device before a release. Emulators skip the OEM battery behaviour that matters here. Orbit is currently smoke-tested on an emulator and one real phone (Android 16), so the checks below are the plan for wider device testing, not a record of passing runs.


Package: `dev.orbit.app`. Replace `<serial>` with the output of `adb devices` (omit `-s` with one device).

Useful commands:

```sh
adb -s <serial> logcat -s Orbit               # app log (sync, reminder, alarm lines)
adb -s <serial> shell dumpsys alarm | grep -A3 dev.orbit.app   # armed alarm
adb -s <serial> shell am force-stop dev.orbit.app             # kill the process
adb -s <serial> reboot                                        # reboot
adb -s <serial> shell date -s "<YYYY-MM-DD HH:MM:SS>"        # root/emulator only: move the clock
```

### Android app and JNI bridge

Install the signed release APK on a real device for this section.

1. **Install and log in.** Install the signed release APK, set the server URL, and log in. Expect the Today view to load.
2. **Share while offline.** Turn on airplane mode. Share text from another app into Orbit. Expect an Inbox Task to be created. Turn airplane mode off: the Task appears on the server after reconnect.
3. **Outbox after force-stop.** Turn on airplane mode and create several tasks. Kill the process with `adb -s <serial> shell am force-stop dev.orbit.app` (kill -9 equivalent). Turn airplane mode off. Expect the outbox to flush in order: the tasks appear on the server in the order they were created.
4. **Outbox after reboot.** Turn on airplane mode and create several tasks. Run `adb -s <serial> reboot`. Turn airplane mode off. Expect the outbox to flush: all tasks appear on the server.
5. **Token survives restart.** Force-stop the app and reopen it. Expect no login prompt: the session token is still in use.

### Reminders

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

### Widget checks

Add the Quick add and Task list widgets to the home screen.

1. **Quick add.** Tap the Quick add widget, type `Buy milk @Home`, submit. Expect the sheet to close and the task to appear in Orbit after the next sync.
   - Log: `adb logcat -s Orbit` shows the capture; the widget redraws.
2. **List source.** Tap the Task list header. Pick a view, then an area. Expect the rows to change to that source. The + button opens the sheet with the source prefilled (`@Area`); for built-in views it is empty.
3. **Circle undo.** Tap a row's circle. Expect a strike-through and a filled circle. Tap it again within 2 seconds: the row returns to normal and the task is not completed. Tap once and wait: after 2 seconds the task leaves the list.
4. **Row open.** Tap a row's title. Expect Orbit to open on that task.
5. **Upcoming groups.** On the Upcoming view, expect day header rows (Overdue and No date included) and the due chip, glyphs, and Area and Project line on each row.
6. **Refresh timing.** Change a task in Orbit, then check the widget within 15 minutes (the periodic refresh) or sooner after a sync. Day start is covered by the same 15-minute refresh, so the widget may show yesterday's Today list for up to 15 minutes after midnight.
7. **Resize.** Resize the Quick add widget from 1 to 5 cells wide, and the Task list in both directions. Expect no clipped controls.
8. **Light and dark.** Switch the system theme. Expect the text and background to follow it. Widgets follow the system light and dark theme and use the app's sage accent on all Android versions.

### Routines and Goal widget checks

Add each widget from the launcher's widget picker. Routines needs Android 12 (API 31) or later for the list.

1. **Routines list.** Add the Routines widget. Expect today's unchecked Routines, each with its streak in muted text. A Routine already checked today is absent.
2. **Check-in and undo.** Tap a Routine. Expect its name struck through and an "Undo" label. Tap it again within 2 s: it returns unchanged, and no check-in is recorded. Let the 2 s pass instead: the Routine leaves the list and `checkin` shows it as done for today.
3. **All done.** Check in every Routine for today. Expect the "All done today" empty state.
4. **Area filter.** Add a second Routines widget, pick one Area in its setup. Expect only that Area's Routines.
5. **Goal card.** Add the Goal widget, pick one Goal. Expect its name, a progress bar with a percentage, and a target date when it has one. Tap it: Orbit opens.
6. **Goal list.** Add another Goal widget and pick "All Goals (list)". Expect each Goal with a bar and a percentage. Tap one: Orbit opens on that Goal.
7. **Behind pace.** A Goal behind its pace shows the "Behind" label in both the card and the list.
8. **Refresh.** Change a Routine or Goal in Orbit or sync from another device. Expect both widgets to update after the next sync.
9. **Removed Goal.** Delete the Goal shown in a card widget. Expect "This Goal is gone" after the next refresh.

## Next

[Linux desktop app](linux-desktop.md): the tray app with the quick capture hotkey.
