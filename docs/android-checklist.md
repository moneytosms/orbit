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

## #70 Widgets

Add the Quick add and Task list widgets to the home screen.

1. **Quick add.** Tap the Quick add widget, type `Buy milk @Home`, submit. Expect the sheet to close and the task to appear in Orbit after the next sync.
   - Log: `adb logcat -s Orbit` shows the capture; the widget redraws.
2. **List source.** Tap the Task list header. Pick a view, then an area. Expect the rows to change to that source. The + button opens the sheet with the source prefilled (`@Area`); for built-in views it is empty.
3. **Circle undo.** Tap a row's circle. Expect a strike-through and a filled circle. Tap it again within 2 seconds: the row returns to normal and the task is not completed. Tap once and wait: after 2 seconds the task leaves the list.
4. **Row open.** Tap a row's title. Expect Orbit to open on that task.
5. **Upcoming groups.** On the Upcoming view, expect day header rows (Overdue and No date included) and the due chip, glyphs, and Area and Project line on each row.
6. **Refresh timing.** Change a task in Orbit, then check the widget within 15 minutes (the periodic refresh) or sooner after a sync. Day start is covered by the same 15-minute refresh, so the widget may show yesterday's Today list for up to 15 minutes after midnight.
7. **Resize.** Resize the Quick add widget from 1 to 5 cells wide, and the Task list in both directions. Expect no clipped controls.
8. **Light and dark.** Switch the system theme. Expect the text and background to follow it. The accent is the system accent on Android 12+ and a fixed blue below.
## #71 Routines and Goal widgets

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
