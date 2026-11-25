# Quick add and tasks

For daily users who want to capture a task in one line and know what Orbit understands. The same parser runs in the web app, the desktop capture overlay, the Android widget and `orbit add`.

## Before you start

- Orbit installed on at least one device ([Install](install.md)) and signed in.
- Know the names of your Areas, Projects, Goals, Pages and Labels. Names are matched case-insensitively and keep their stored casing.

## Steps

1. Type a title and add tokens anywhere in the line. The parser removes recognised tokens from the title:

   | Token | Meaning | Example |
   | --- | --- | --- |
   | `today`, `tomorrow`, `tmrw`, `fri`, `next fri`, `this fri`, `in 3 days`, `next week`, `next month`, `next year`, `this weekend`, `end of month`, `mid jan`, `dec 5th`, `5 dec 2027`, `the 27th`, `27/1`, `2026-12-01` | due date (`next week` is next Monday; a bare weekday is its next occurrence; `next fri` is the one in next week; `27/1` and `3/12` read day/month) | `pay rent on fri` |
   | `6pm`, `18:00`, `9:15am`, `6 pm`, `at noon`, `at midnight` | due time (the next time the clock shows it if no date: `9am` typed at 10:00 means tomorrow) | `call mom tomorrow 6pm` |
   | `today at 10pm`, `fri at 7pm`, `9am fri`, `tonight`, `tomorrow morning`, `this evening`, `in the evening` | date plus time or a part of the day (morning 09:00, afternoon 14:00, evening 18:00, night 21:00, tonight 20:00) | `gym tomorrow morning` |
   | `in 5 minutes`, `in 2 hours`, `in half an hour`, `in 1h30m`, `in an hour and a half`, `90 minutes from now`, `in 2 months` | relative to now, with the time when it is hours or minutes | `check oven in 20 min` |
   | `p1` to `p4` | priority, last one wins | `fix bug p1` |
   | `#label` | label | `buy milk #errand` |
   | `@Name` | Area or Project | `read chapter @College` |
   | `$Name` | Goal | `run 5k $Get fit` |
   | `%Name` | Page | `book flights %Trip notes` |
   | `every day`, `every mon,wed`, `every weekday at 9am`, `every! 3 weeks` | recurrence (`every!` counts from completion; `at` before the time is optional) | `water plants every 3 days` |
   | `!30m`, `!9am`, `!fri 9am`, `!in 10 minutes`, `!tomorrow morning` | reminder (offset before due, or a fixed time) | `dentist fri 3pm !1h` |

2. Or click instead of typing. While you type, a row of options sits under the box (destination, date, priority, labels, goal, reminder), in the New task window, the bottom bar and the desktop quick-capture window. Each option shows what the line currently means, and picking one (a month calendar for dates) rewrites the typed text with the same syntax, so you can mix typing and clicking.
3. Use a backslash to keep a sigil literal: `issue \#1` stays `issue #1`.
4. An unknown name stays in the title and shows as unresolved instead of guessing.
5. Organise later. Captures with no destination land in the Inbox. Move a task by dragging it, from its menu, or by editing it in the task drawer.
6. Group tasks inside an Area or a Project with **Sections**: use **+ New section** at the end of the list. Drag tasks between sections and drag headings to reorder. Deleting a section moves its tasks to the parent list, never deletes them.

## Verify it worked

```sh
orbit add "call the dentist tomorrow 9am p2 #health"
orbit ls
```

The task is listed with tomorrow's date, priority 2 and the `health` label, and its title is `call the dentist`.

## Troubleshooting

**A word became part of the title instead of a date.** Only whole phrases count: `may`, `5` and `27th` alone are not dates. Write `may 5`, `on the 27th`, an ISO date, or a weekday. `at 5` is not a time either: say `at 5pm` or `at 17:00`.

**Something like `1/2 cup` became a date.** `d/m` reads as a date. Write the amount another way, for example `half a cup`.

**`@Name` did not resolve.** The name must match an existing Area or Project and end at a word boundary. Multi-word names match the longest name first.

**A task I added offline is missing on another device.** Changes wait in the queue until the next sync. See [Install, Troubleshooting](install.md#troubleshooting).

## Next

[Views and filters](views.md): see the tasks you captured the way you want.
