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
   | `every day`, `every mon,wed`, `every weekday at 9am`, `every! 3 weeks`, `every other fri`, `every quarter`, `every 2nd wed`, `every last fri`, `every first workday`, `every 15th workday`, `every 3 workdays`, `every day until dec 25`, `every week starting dec 1`, `every day for 3 weeks` | recurrence (`every!` counts from completion; `at` before the time is optional). `other` = every 2nd (`every other fri` = every 2 weeks on Friday); `quarter` = 3 months; `1st`..`5th`/`last` + weekday = that weekday of the month (a 5th is skipped in months without one); `Nth workday`/`last workday` count Mon-Fri days of the month. Optional bounds after the rule: `starting`/`from <date>`, `until`/`ending`/`till`/`through <date>` (inclusive), `for N days/weeks/months/years`; completing the last occurrence completes the task. No hourly rules (a due is a date plus an optional time) | `water plants every 3 days` |
   | `{march 30}`, `{next fri}`, `{2026-12-24}`, `by friday`, `by dec 5`, `by end of month`, `by next fri`, `by in 3 days` | deadline (a date; any time in the phrase is dropped). Sets the deadline only, never the due date, so `report tomorrow by fri` has both. `by` counts only when a date follows (`written by Tom` and `by 5pm` stay as typed). Any date phrase from the first rows works inside the braces | `send report by fri` |
   | `for 1h`, `for 30 min`, `for 1h15m`, `for 90 minutes`, `for an hour`, `for half an hour`, `for 1.5 hours` | duration, 1 minute to 24 hours (`for 25h` and `for 3 days` stay in the title; so does `for the team`) | `write essay for 1h30m` |
   | `3-4pm`, `9:30-10:00`, `9am-10:30am`, `from 9 to 10:30` | time range: due time = the start, duration = the length (`tomorrow 3-4pm` works; the end must be after the start, `9-10` without a colon or am/pm is not a range, and ranges across midnight are not read) | `standup tomorrow 9:30-9:45` |
   | `!30m`, `!9am`, `!fri 9am`, `!in 10 minutes`, `!tomorrow morning` | reminder (offset before due, or a fixed time) | `dentist fri 3pm !1h` |

2. Or click instead of typing. While you type, a row of options sits under the box (destination, date, deadline, duration, priority, labels, goal, reminder), in the New task window, the bottom bar and the desktop quick-capture window. Each option shows what the line currently means, and picking one (a month calendar for dates and deadlines, 15m/30m/1h/2h or custom minutes for duration) rewrites the typed text with the same syntax, so you can mix typing and clicking.
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

**`by` or `for` stayed in my title.** They only count when what follows parses: a date after `by` (`by fri`, `by dec 5`; a bare time like `by 5pm` is not one, write `5pm` for the due time) and a length after `for` (`for 1h`, `for 30 min`; at most 24 hours, no days). Only the first deadline and the first duration count. Words after `every` never become a due date, but `for 1h` and `by fri` after a rule still set duration and deadline.

**`3-4` or `9-10` stayed in the title.** A range needs a colon or am/pm: `3-4pm`, `9:00-10:00`, `from 9 to 10`.

**`@Name` did not resolve.** The name must match an existing Area or Project and end at a word boundary. Multi-word names match the longest name first.

**A task I added offline is missing on another device.** Changes wait in the queue until the next sync. See [Install, Troubleshooting](install.md#troubleshooting).

## Next

[Views and filters](views.md): see the tasks you captured the way you want.
